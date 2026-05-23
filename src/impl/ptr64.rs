//! 64-bit high-performance lock-free atomic tagged pointer backend.
//!
//! This module implements the `AtomicTaggedPtrImpl` using `AtomicUsize` under 64-bit platforms.
//! It splits the 64-bit word into two parts:
//! - **High 8 bits**: Used for storing `tag` (generation count) to protect against the ABA problem.
//! - **Low 56 bits**: Used for storing the actual physical pointer address.
//!
//! Since user-space virtual addresses under modern 64-bit operating systems (including Intel 5-level
//! paging with 57-bit address space, and ARMv8.2 with 52-bit address space) never exceed 56 bits
//! (with user-space address limit usually at `007f_ffff_ffff_ffff`), using the lower 56 bits for the
//! physical pointer ensures 100% address integrity and completely avoids truncation or pointer corruption.

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Masks for bitwise packing and unpacking.
#[cfg(virt_addr_48)]
pub(crate) const PTR_MASK: usize = 0x0000_FFFF_FFFF_FFFF;
#[cfg(virt_addr_48)]
pub(crate) const TAG_SHIFT: usize = 48;
#[cfg(virt_addr_48)]
pub const TAG_MASK: usize = 0xFFFF;

#[cfg(not(virt_addr_48))]
pub(crate) const PTR_MASK: usize = 0x00FF_FFFF_FFFF_FFFF;
#[cfg(not(virt_addr_48))]
pub(crate) const TAG_SHIFT: usize = 56;
#[cfg(not(virt_addr_48))]
pub const TAG_MASK: usize = 0xFF;

/// A 64-bit platform lock-free implementation of `AtomicTaggedPtr`.
///
/// Under 64-bit architectures, this structure maps atomic operations directly to high-performance
/// hardware CAS instructions (`cmpxchg` on x86_64 or `ldrex/strex` / `cas` on AArch64).
pub(crate) struct AtomicTaggedPtrImpl<T> {
    bits: AtomicUsize,
    _marker: PhantomData<*mut T>,
}

// Safety: AtomicTaggedPtrImpl handles physical atomics and interior synchronization safely.
unsafe impl<T> Send for AtomicTaggedPtrImpl<T> {}
unsafe impl<T> Sync for AtomicTaggedPtrImpl<T> {}

impl<T> AtomicTaggedPtrImpl<T> {
    /// Creates a new `AtomicTaggedPtrImpl` with the given pointer and an initial tag of 0.
    #[inline]
    pub(crate) fn new(ptr: Option<NonNull<T>>) -> Self {
        let ptr_val = ptr.map(|p| p.as_ptr() as usize).unwrap_or(0);
        // Ensure that the initial pointer does not overflow the limits
        debug_assert_eq!(
            ptr_val & !PTR_MASK,
            0,
            "Pointer address overflows limit!"
        );

        Self {
            bits: AtomicUsize::new(ptr_val & PTR_MASK),
            _marker: PhantomData,
        }
    }

    /// Packs a pointer and a tag into a single 64-bit `usize` value.
    ///
    /// # Safety
    ///
    /// The tag value will be truncated to fit the high 8 bits (modulo 256).
    /// The pointer address must reside within the valid 56-bit user-space memory limits.
    #[inline]
    fn pack(ptr: *const T, tag: usize) -> usize {
        let ptr_val = ptr as usize;
        // Verify pointer doesn't spill into the tag region under debug mode
        debug_assert_eq!(
            ptr_val & !PTR_MASK,
            0,
            "Attempted to pack a pointer exceeding the limits!"
        );

        let truncated_tag = tag & TAG_MASK;
        (truncated_tag << TAG_SHIFT) | (ptr_val & PTR_MASK)
    }

    /// Unpacks a 64-bit `usize` value into its component pointer and tag.
    #[inline]
    fn unpack(bits: usize) -> (Option<NonNull<T>>, usize) {
        let ptr_val = bits & PTR_MASK;
        let tag = bits >> TAG_SHIFT;

        let ptr = if ptr_val == 0 {
            None
        } else {
            // Safety: The pointer address is safely reconstructed from the lower 56 bits
            unsafe { Some(NonNull::new_unchecked(ptr_val as *mut T)) }
        };

        (ptr, tag)
    }

    /// Atomically loads the tagged pointer.
    ///
    /// Memory ordering must be one of `Acquire`, `Relaxed`, or `SeqCst`.
    #[inline]
    pub(crate) fn load(&self, order: Ordering) -> (Option<NonNull<T>>, usize) {
        let bits = self.bits.load(order);
        Self::unpack(bits)
    }

    /// Atomically stores a new tagged pointer.
    ///
    /// Memory ordering must be one of `Release`, `Relaxed`, or `SeqCst`.
    #[inline]
    pub(crate) fn store(&self, ptr: Option<NonNull<T>>, tag: usize, order: Ordering) {
        let raw_ptr = ptr
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(std::ptr::null());
        let bits = Self::pack(raw_ptr, tag);
        self.bits.store(bits, order);
    }

    /// Atomically exchanges the tagged pointer value if the current value matches expectations.
    ///
    /// Returns `Ok` with the new value if successful, or `Err` with the actual loaded value on failure.
    #[inline]
    pub(crate) fn compare_exchange(
        &self,
        current: (Option<NonNull<T>>, usize),
        new: (Option<NonNull<T>>, usize),
        success: Ordering,
        failure: Ordering,
    ) -> super::TaggedPtrResult<T> {
        let cur_raw = current
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(std::ptr::null());
        let new_raw = new
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(std::ptr::null());

        let cur_bits = Self::pack(cur_raw, current.1);
        let new_bits = Self::pack(new_raw, new.1);

        match self
            .bits
            .compare_exchange(cur_bits, new_bits, success, failure)
        {
            Ok(bits) => Ok(Self::unpack(bits)),
            Err(bits) => Err(Self::unpack(bits)),
        }
    }

    /// Atomically exchanges the tagged pointer value with weak semantics.
    ///
    /// Suitable for spin-loops. Performance on certain ARM platforms can be significantly better.
    #[inline]
    pub(crate) fn compare_exchange_weak(
        &self,
        current: (Option<NonNull<T>>, usize),
        new: (Option<NonNull<T>>, usize),
        success: Ordering,
        failure: Ordering,
    ) -> super::TaggedPtrResult<T> {
        let cur_raw = current
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(std::ptr::null());
        let new_raw = new
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(std::ptr::null());

        let cur_bits = Self::pack(cur_raw, current.1);
        let new_bits = Self::pack(new_raw, new.1);

        match self
            .bits
            .compare_exchange_weak(cur_bits, new_bits, success, failure)
        {
            Ok(bits) => Ok(Self::unpack(bits)),
            Err(bits) => Err(Self::unpack(bits)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_sanity() {
        let val = 42;
        let ptr = NonNull::new(&val as *const i32 as *mut i32);

        let packed = AtomicTaggedPtrImpl::pack(ptr.unwrap().as_ptr(), 12);
        let (unpacked_ptr, tag) = AtomicTaggedPtrImpl::unpack(packed);

        assert_eq!(tag, 12);
        assert_eq!(unpacked_ptr, ptr);

        // Null pointer packing test
        let packed_null = AtomicTaggedPtrImpl::pack(std::ptr::null::<i32>(), 250);
        let (unpacked_null, tag_null) = AtomicTaggedPtrImpl::<i32>::unpack(packed_null);
        assert_eq!(tag_null, 250);
        assert!(unpacked_null.is_none());
    }

    #[test]
    fn test_high_address_simulation() {
        #[cfg(virt_addr_48)]
        let high_addr = 0x0000_7FFF_FFFF_F000 as *const i32;
        #[cfg(not(virt_addr_48))]
        let high_addr = 0x007F_FFFF_FFFF_F000 as *const i32;

        let tag = 177;

        let packed = AtomicTaggedPtrImpl::<i32>::pack(high_addr, tag);
        let (unpacked_ptr, unpacked_tag) = AtomicTaggedPtrImpl::<i32>::unpack(packed);

        assert_eq!(unpacked_tag, tag);
        assert_eq!(
            unpacked_ptr.map(|p| p.as_ptr() as usize),
            Some(high_addr as usize)
        );
    }

    #[test]
    fn test_cas_loop_simulation() {
        let value = 100;
        let ptr = NonNull::new(&value as *const i32 as *mut i32);
        let atom = AtomicTaggedPtrImpl::new(ptr);

        let loaded = atom.load(Ordering::Acquire);
        assert_eq!(loaded.0, ptr);
        assert_eq!(loaded.1, 0);

        let new_value = 200;
        let new_ptr = NonNull::new(&new_value as *const i32 as *mut i32);

        let cas_res =
            atom.compare_exchange((ptr, 0), (new_ptr, 1), Ordering::SeqCst, Ordering::SeqCst);

        assert!(cas_res.is_ok());
        let loaded_new = atom.load(Ordering::Acquire);
        assert_eq!(loaded_new.0, new_ptr);
        assert_eq!(loaded_new.1, 1);
    }
}
