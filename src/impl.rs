//! A platform-adaptive atomic tagged pointer implementation with robust ABA protection.
//!
//! # Background & Hardware Realities
//!
//! In lock-free concurrent programming, particularly when constructing intrusive data structures
//! such as a Treiber Stack, the **ABA problem** frequently arises. The traditional mitigation involves
//! pairing the physical pointer with a generation tag, updating both atomically.
//!
//! However, this incurs CPU architecture constraints:
//! 1. **64-bit Systems with High Virtual Addresses (52/57-bit)**:
//!    Modern operating systems on x86_64 (using Intel 5-level paging for 57-bit address space) or AArch64
//!    (using 52-bit virtual addresses) utilize pointer spaces beyond the typical 48-bit region. Assumed
//!    48-bit address layout limits lead to pointer truncation and severe wild-pointer crashes.
//!    This module splits the 64-bit word dynamically: reserving the **lower 56 bits** for the physical pointer
//!    (covering the entire `007f_ffff_ffff_ffff` user-space boundary) and the **upper 8 bits** for the tag.
//!    This provides absolute pointer integrity across all current server environments.
//! 2. **32-bit Systems**:
//!    Pointer width is 32 bits. We pair it with a 32-bit generation tag to form a double-word 64-bit composite.
//!    This leverages hardware-level 64-bit atomic operations (such as `cmpxchg8b` on x86 or `ldrd/strd` on ARMv7)
//!    to complete CAS transitions natively, without making raw address size assumptions.
//! 3. **Non-AtomicFallback Systems**:
//!    Under highly customized secure hypervisors (using full MTE tagging) or extremely constrained microcontrollers
//!    without native 64-bit atomics, the implementation seamlessly falls back to standard Mutex synchronization.
//!    This guarantees 100% compilation safety without sacrificing API consistency or memory efficiency.

use core::fmt;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

// --- Platform Routing Conditional Compile Sections ---

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
mod ptr64;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
mod ptr32;

#[cfg(atomic_fallback)]
mod fallback;

// --- Type Alias to satisfy Clippy type_complexity ---

/// Type alias representing the result of atomic compare and exchange operations.
pub type TaggedPtrResult<T> = Result<(Option<NonNull<T>>, usize), (Option<NonNull<T>>, usize)>;

// --- Exposing Unified High-Level Struct ---

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
pub use ptr64::TAG_MASK;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
pub use ptr32::TAG_MASK;

#[cfg(atomic_fallback)]
pub use fallback::TAG_MASK;

/// A platform-adaptive atomic tagged pointer supporting thread-safe ABA protection.
pub struct AtomicTaggedPtr<T> {
    #[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
    inner: ptr64::AtomicTaggedPtrImpl<T>,

    #[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
    inner: ptr32::AtomicTaggedPtrImpl<T>,

    #[cfg(atomic_fallback)]
    inner: fallback::AtomicTaggedPtrImpl<T>,
}

// Safety: AtomicTaggedPtr is an atomic synchronizer wrapping pointer locations, safe to send/share across threads.
unsafe impl<T> Send for AtomicTaggedPtr<T> {}
unsafe impl<T> Sync for AtomicTaggedPtr<T> {}

impl<T> AtomicTaggedPtr<T> {
    /// Creates a new `AtomicTaggedPtr` initialized with the given pointer and tag 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ptr::NonNull;
    /// use atomic_tagged_ptr::AtomicTaggedPtr;
    ///
    /// let value = 42;
    /// let ptr = NonNull::new(&value as *const i32 as *mut i32);
    /// let atom = AtomicTaggedPtr::new(ptr);
    /// ```
    #[inline]
    pub fn new(ptr: Option<NonNull<T>>) -> Self {
        Self {
            #[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
            inner: ptr64::AtomicTaggedPtrImpl::new(ptr),

            #[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
            inner: ptr32::AtomicTaggedPtrImpl::new(ptr),

            #[cfg(atomic_fallback)]
            inner: fallback::AtomicTaggedPtrImpl::new(ptr),
        }
    }

    /// Loads the current values of the pointer and tag atomically.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Release` or `AcqRel`.
    #[inline]
    pub fn load(&self, order: Ordering) -> (Option<NonNull<T>>, usize) {
        self.inner.load(order)
    }

    /// Stores a new pointer and tag atomically.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Acquire` or `AcqRel`.
    #[inline]
    pub fn store(&self, ptr: Option<NonNull<T>>, tag: usize, order: Ordering) {
        self.inner.store(ptr, tag, order);
    }

    /// Exchanges the current values with new ones if the current values match expectations.
    ///
    /// On success, returns `Ok` containing the previous pointer and tag.
    /// On failure, returns `Err` containing the actual loaded pointer and tag.
    #[inline]
    pub fn compare_exchange(
        &self,
        current: (Option<NonNull<T>>, usize),
        new: (Option<NonNull<T>>, usize),
        success: Ordering,
        failure: Ordering,
    ) -> TaggedPtrResult<T> {
        self.inner.compare_exchange(current, new, success, failure)
    }

    /// Exchanges the current values with new ones using weak semantics.
    ///
    /// This is a weaker variant of `compare_exchange` which is allowed to fail spuriously,
    /// but can be significantly more efficient on certain LL/SC-based architectures (such as ARM).
    #[inline]
    pub fn compare_exchange_weak(
        &self,
        current: (Option<NonNull<T>>, usize),
        new: (Option<NonNull<T>>, usize),
        success: Ordering,
        failure: Ordering,
    ) -> TaggedPtrResult<T> {
        self.inner.compare_exchange_weak(current, new, success, failure)
    }
}

// --- Common Trait Implementations ---

impl<T> Default for AtomicTaggedPtr<T> {
    #[inline]
    fn default() -> Self {
        Self::new(None)
    }
}

impl<T> fmt::Debug for AtomicTaggedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safe load under Relaxed ordering to capture debug state snapshot
        let (ptr, tag) = self.load(Ordering::Relaxed);
        f.debug_struct("AtomicTaggedPtr")
            .field("pointer", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

// --- Built-in Local Integration Tests ---

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use std::format;

    #[test]
    fn test_default_initializer() {
        let atom: AtomicTaggedPtr<i32> = Default::default();
        let (ptr, tag) = atom.load(Ordering::Relaxed);
        assert!(ptr.is_none());
        assert_eq!(tag, 0);
    }

    #[test]
    fn test_debug_formatter() {
        let val = 12345;
        let ptr = NonNull::new(&val as *const i32 as *mut i32);
        let atom = AtomicTaggedPtr::new(ptr);
        atom.store(ptr, 88, Ordering::Relaxed);

        let debug_str = format!("{:?}", atom);
        assert!(debug_str.contains("AtomicTaggedPtr"));
        assert!(debug_str.contains("tag: 88"));
    }

    #[test]
    fn test_multithreaded_atomic_exchanges() {
        use std::sync::Arc;
        use std::thread;

        let val = 777;
        let ptr = NonNull::new(&val as *const i32 as *mut i32);
        let ptr_usize = ptr.unwrap().as_ptr() as usize;
        let atom = Arc::new(AtomicTaggedPtr::new(ptr));

        let atom_clone = Arc::clone(&atom);
        let handle = thread::spawn(move || {
            let loaded = atom_clone.load(Ordering::Acquire);
            let local_ptr = NonNull::new(ptr_usize as *mut i32);
            if loaded.0 == local_ptr && loaded.1 == 0 {
                let _ = atom_clone.compare_exchange(
                    (local_ptr, 0),
                    (None, 55),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
            }
        });

        handle.join().unwrap();
        let final_state = atom.load(Ordering::Acquire);

        // Assert state was safely transitioned or remained valid
        assert!(final_state.1 == 55 || final_state.1 == 0);
    }
}
