//! Comprehensive integration tests for `atomic_tagged_ptr`.
//!
//! These tests verify the robustness of `AtomicTaggedPtr` under high-concurrency,
//! simulate high 57-bit virtual address (Intel 5-level paging) unpack safety,
//! and construct a full lock-free intrusive Treiber Stack to rigorously test
//! defense against the classic ABA problem.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

use atomic_tagged_ptr::{AtomicTaggedPtr, Tag};
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

// --- 1. Simulation of High 57-bit (5-Level Paging) Address Integrity ---

#[test]
fn test_57bit_virtual_address_integrity() {
    // 5-level paging user-space virtual addresses reside inside [0, 0x007F_FFFF_FFFF_FFFF].
    // We construct dummy pointers on these extreme boundary addresses to check for truncation.
    #[cfg(virt_addr_48)]
    let extreme_user_space_addresses = [
        0x0000_0000_0000_1000 as *const i32, // Classic low address
        0x0000_7FFF_FFFF_F000 as *const i32, // 48-bit ceiling
    ];
    #[cfg(not(virt_addr_48))]
    let extreme_user_space_addresses = [
        0x0000_0000_0000_1000 as *const i32, // Classic low address
        0x0000_7FFF_FFFF_F000 as *const i32, // 48-bit ceiling
        0x003F_FFFF_FFFF_C000 as *const i32, // Mid 5-level address
        0x007F_FFFF_FFFF_F000 as *const i32, // Extreme 5-level user-space ceiling
    ];

    for &original_addr in &extreme_user_space_addresses {
        let atom = AtomicTaggedPtr::new(atomic_tagged_ptr::TaggedPtr::new(
            NonNull::new(original_addr as *mut i32),
            Tag::new(0),
        ));

        for tag in [0, 1, 127, 255, 256, 1024, 0xABCDEF] {
            // Store with arbitrary tags
            atom.store(
                atomic_tagged_ptr::TaggedPtr::new(
                    NonNull::new(original_addr as *mut i32),
                    Tag::new(tag),
                ),
                Ordering::Release,
            );

            let loaded = atom.load(Ordering::Acquire);

            // Verify pointer was not corrupted or truncated!
            assert_eq!(
                loaded.ptr.option().map(|p| p.as_ptr() as usize),
                Some(original_addr as usize),
                "Pointer address corrupted on extreme address: {:#X}",
                original_addr as usize
            );

            // Verify tag is correctly masked under the current platform layout
            assert_eq!(
                loaded.tag.value(),
                tag & atomic_tagged_ptr::TAG_MASK,
                "Tag mismatch for tag value {:#X}",
                tag
            );
        }
    }
}

// --- 2. Treiber Stack Concurrency and ABA Protection Test ---

#[cfg(feature = "std")]
mod concurrent_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::vec::Vec;

    /// A node in our intrusive Treiber Stack.
    struct StackNode {
        value: usize,
        // Intrusive next pointer slot
        next: AtomicTaggedPtr<StackNode>,
    }

    /// A fully functional lock-free intrusive Treiber Stack powered by `AtomicTaggedPtr`.
    struct TreiberStack {
        // Head pointer along with generation tag for ABA defense
        head: AtomicTaggedPtr<StackNode>,
    }

    impl TreiberStack {
        fn new() -> Self {
            Self {
                head: AtomicTaggedPtr::new(atomic_tagged_ptr::TaggedPtr::default()),
            }
        }

        fn push(&self, node: &StackNode) {
            let node_ptr = NonNull::from(node);
            let mut bits = self.head.load(Ordering::Acquire);
            loop {
                // Intrusively link our node to the current head pointer
                node.next.store(bits, Ordering::Release);

                // Advance the generation tag to defend against ABA
                let next_tag = bits.tag.wrapping_add(1);

                match self.head.compare_exchange_weak(
                    bits,
                    atomic_tagged_ptr::TaggedPtr::new(Some(node_ptr), next_tag),
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(actual) => bits = actual,
                }
            }
        }

        fn pop(&self) -> Option<&StackNode> {
            let mut bits = self.head.load(Ordering::Acquire);
            loop {
                let head_ptr = bits.ptr.option()?;

                // Read the next node intrusively.
                // Safety: Under garbage collection or static node allocation, this node memory remains valid.
                let next_state = unsafe { head_ptr.as_ref().next.load(Ordering::Acquire) };

                // Advance generation tag to defend against ABA
                let next_tag = bits.tag.wrapping_add(1);

                match self.head.compare_exchange_weak(
                    bits,
                    atomic_tagged_ptr::TaggedPtr::new(next_state.ptr, next_tag),
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    // Safety: We reconstruct the reference safely
                    Ok(_) => return Some(unsafe { head_ptr.as_ref() }),
                    Err(actual) => bits = actual,
                }
            }
        }
    }

    #[test]
    fn test_treiber_stack_concurrent_aba_defense() {
        let num_threads = 8;
        let num_ops = 5000;

        let stack = Arc::new(TreiberStack::new());
        let mut handles = Vec::new();

        // Statically allocated nodes to ensure safe memory access across threads without GC overhead
        let nodes: Vec<StackNode> = (0..num_threads * num_ops)
            .map(|i| StackNode {
                value: i,
                next: AtomicTaggedPtr::new(atomic_tagged_ptr::TaggedPtr::default()),
            })
            .collect();

        let nodes_ref = Arc::new(nodes);
        let success_count = Arc::new(AtomicUsize::new(0));

        for thread_idx in 0..num_threads {
            let stack_clone = Arc::clone(&stack);
            let nodes_clone = Arc::clone(&nodes_ref);
            let success_clone = Arc::clone(&success_count);

            let handle = thread::spawn(move || {
                let start_idx = thread_idx * num_ops;

                // Dense push ops
                for op in 0..num_ops {
                    let node_ref = &nodes_clone[start_idx + op];
                    stack_clone.push(node_ref);
                }

                // Dense pop ops
                for _ in 0..num_ops {
                    if let Some(node) = stack_clone.pop() {
                        success_clone.fetch_add(1, Ordering::Relaxed);
                        // Ensure the popped node contains valid thread ranges
                        assert!(node.value < num_threads * num_ops);
                    }
                }
            });
            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        // Verify all nodes pushed were successfully popped without any ABA corruption
        assert_eq!(success_count.load(Ordering::SeqCst), num_threads * num_ops);
        assert!(stack.pop().is_none());
    }
}
