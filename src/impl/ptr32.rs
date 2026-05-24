//! 32-bit lock-free atomic tagged pointer backend.
//!
//! This module implements the `AtomicTaggedPtrImpl` using `AtomicU64` for 32-bit platforms that
//! native-support 64-bit atomic operations (e.g., ARMv7-A with `ldrd/strd` and x86 i686 with `cmpxchg8b`).
//! It splits the 64-bit word into two equal parts:
//! - **High 32 bits**: Used for storing `tag` (generation count) for extremely robust ABA protection
//!   (4.2 billion generations before wrapping).
//! - **Low 32 bits**: Used for storing the complete 32-bit pointer address.
//!
//! Since pointer addresses under 32-bit operating systems fit perfectly inside 32 bits, this layout
//! does not make any assumptions about virtual address zero-bits and retains 100% address precision.

use super::Tag;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, Ordering};

pub const TAG_MASK: usize = 0xFFFF_FFFF;

/// A 32-bit platform implementation of `AtomicTaggedPtr` leveraging 64-bit atomics.
///
/// Under supported 32-bit architectures, this structure maps atomic operations directly to high-performance
/// double-word CAS instructions (such as `cmpxchg8b` on x86/i686).
pub(crate) struct AtomicTaggedPtrImpl<T> {
    bits: AtomicU64,
    _marker: PhantomData<*mut T>,
}

// Safety: AtomicTaggedPtrImpl handles physical atomics and interior synchronization safely.
unsafe impl<T> Send for AtomicTaggedPtrImpl<T> {}
unsafe impl<T> Sync for AtomicTaggedPtrImpl<T> {}

impl<T> AtomicTaggedPtrImpl<T> {
    /// Creates a new `AtomicTaggedPtrImpl` with the given pointer and tag.
    #[inline]
    pub(crate) fn new(ptr: Option<NonNull<T>>, tag: Tag) -> Self {
        let ptr_raw = ptr
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());
        let bits = Self::pack(ptr_raw, tag);
        Self {
            bits: AtomicU64::new(bits),
            _marker: PhantomData,
        }
    }

    /// Packs a pointer and a tag into a single 64-bit `u64` value.
    #[inline]
    fn pack(ptr: *const T, tag: Tag) -> u64 {
        let ptr_val = ptr as usize as u64;
        let tag_val = tag.value() as u64;
        (tag_val << 32) | ptr_val
    }

    /// Unpacks a 64-bit `u64` value into its component pointer and tag.
    #[inline]
    fn unpack(bits: u64) -> (Option<NonNull<T>>, Tag) {
        let ptr_val = (bits & 0xFFFFFFFF) as usize;
        let tag = Tag::new((bits >> 32) as usize);

        let ptr = if ptr_val == 0 {
            None
        } else {
            // Safety: The pointer address is safely reconstructed from the lower bits preserving strict provenance
            unsafe {
                Some(NonNull::new_unchecked(
                    core::ptr::with_exposed_provenance_mut(ptr_val),
                ))
            }
        };

        (ptr, tag)
    }

    /// Atomically loads the tagged pointer.
    ///
    /// Memory ordering must be one of `Acquire`, `Relaxed`, or `SeqCst`.
    #[inline]
    pub(crate) fn load(&self, order: Ordering) -> (Option<NonNull<T>>, Tag) {
        let bits = self.bits.load(order);
        Self::unpack(bits)
    }

    /// Atomically stores a new tagged pointer.
    ///
    /// Memory ordering must be one of `Release`, `Relaxed`, or `SeqCst`.
    #[inline]
    pub(crate) fn store(&self, ptr: Option<NonNull<T>>, tag: Tag, order: Ordering) {
        let raw_ptr = ptr
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());
        let bits = Self::pack(raw_ptr, tag);
        self.bits.store(bits, order);
    }

    /// Atomically exchanges the tagged pointer value if the current value matches expectations.
    ///
    /// Returns `Ok` with the new value if successful, or `Err` with the actual loaded value on failure.
    #[inline]
    pub(crate) fn compare_exchange(
        &self,
        current: (Option<NonNull<T>>, Tag),
        new: (Option<NonNull<T>>, Tag),
        success: Ordering,
        failure: Ordering,
    ) -> super::RawTaggedPtrResult<T> {
        let cur_raw = current
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());
        let new_raw = new
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());

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
    /// Suitable for spin-loops.
    #[inline]
    pub(crate) fn compare_exchange_weak(
        &self,
        current: (Option<NonNull<T>>, Tag),
        new: (Option<NonNull<T>>, Tag),
        success: Ordering,
        failure: Ordering,
    ) -> super::RawTaggedPtrResult<T> {
        let cur_raw = current
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());
        let new_raw = new
            .0
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());

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

    /// Atomically exchanges the value and returns the old value.
    #[inline]
    pub(crate) fn swap(
        &self,
        ptr: Option<NonNull<T>>,
        tag: Tag,
        order: Ordering,
    ) -> (Option<NonNull<T>>, Tag) {
        let raw_ptr = ptr
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null());
        let bits = Self::pack(raw_ptr, tag);
        let old_bits = self.bits.swap(bits, order);
        Self::unpack(old_bits)
    }

    /// Consumes the atomic and returns the inner value.
    #[inline]
    pub(crate) fn into_inner(self) -> (Option<NonNull<T>>, Tag) {
        Self::unpack(self.bits.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_sanity_32bit() {
        let val = 99;
        let ptr = NonNull::new(&val as *const i32 as *mut i32);

        let packed = AtomicTaggedPtrImpl::pack(ptr.unwrap().as_ptr(), Tag::new(0xDEADBEEF));
        let (unpacked_ptr, tag) = AtomicTaggedPtrImpl::unpack(packed);

        assert_eq!(tag.value(), 0xDEADBEEF);
        assert_eq!(unpacked_ptr, ptr);

        // Null pointer packing test
        let packed_null = AtomicTaggedPtrImpl::pack(core::ptr::null::<i32>(), Tag::new(42));
        let (unpacked_null, tag_null) = AtomicTaggedPtrImpl::unpack(packed_null);
        assert_eq!(tag_null.value(), 42);
        assert!(unpacked_null.is_none());
    }

    #[test]
    fn test_cas_loop_simulation_32bit() {
        let value = 500;
        let ptr = NonNull::new(&value as *const i32 as *mut i32);
        let atom = AtomicTaggedPtrImpl::new(ptr, Tag::new(0));

        let loaded = atom.load(Ordering::Acquire);
        assert_eq!(loaded.0, ptr);
        assert_eq!(loaded.1.value(), 0);

        let new_value = 600;
        let new_ptr = NonNull::new(&new_value as *const i32 as *mut i32);

        let cas_res = atom.compare_exchange(
            (ptr, Tag::new(0)),
            (new_ptr, Tag::new(0xABCDEF01)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );

        assert!(cas_res.is_ok());
        let loaded_new = atom.load(Ordering::Acquire);
        assert_eq!(loaded_new.0, new_ptr);
        assert_eq!(loaded_new.1.value(), 0xABCDEF01);
    }
}
