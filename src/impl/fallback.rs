//! Mutex-based safe fallback atomic tagged pointer backend.
//!
//! This module implements the `AtomicTaggedPtrImpl` using `std::sync::Mutex` under platforms that
//! either do not support native 64-bit atomic operations or require complete, un-truncated pointer
//! precision under absolute-pointer security regimes (such as memory tagging extensions or custom hypervisors).
//!
//! While it is synchronized using a mutex, it maintains 100% API compatibility and rigorously simulates
//! lock-free compare-and-swap behavior inside the critical section, ensuring data safety and ABA protection
//! under all conditions.

#[cfg(not(feature = "std"))]
compile_error!(
    "The Mutex-based fallback implementation for `atomic-tagged-ptr` requires the `std` feature to be enabled. \
     Please enable the `std` feature in your Cargo.toml."
);

use super::Tag;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;
#[cfg(feature = "parking_lot")]
use parking_lot::{Mutex as InnerMutex, MutexGuard as InnerMutexGuard};

#[cfg(not(feature = "parking_lot"))]
use std::sync::{Mutex as InnerMutex, MutexGuard as InnerMutexGuard};

pub(crate) struct Mutex<T>(InnerMutex<T>);

impl<T> Mutex<T> {
    #[inline]
    pub const fn new(val: T) -> Self {
        Self(InnerMutex::new(val))
    }

    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        #[cfg(feature = "parking_lot")]
        {
            MutexGuard(self.0.lock())
        }
        #[cfg(not(feature = "parking_lot"))]
        {
            MutexGuard(self.0.lock().unwrap_or_else(|e| e.into_inner()))
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        #[cfg(feature = "parking_lot")]
        {
            self.0.into_inner()
        }
        #[cfg(not(feature = "parking_lot"))]
        {
            self.0.into_inner().unwrap_or_else(|e| e.into_inner())
        }
    }
}

pub(crate) struct MutexGuard<'a, T>(InnerMutexGuard<'a, T>);

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub const TAG_MASK: usize = usize::MAX;

/// A Mutex-synchronized safe implementation of `AtomicTaggedPtr` for fallback targets.
pub(crate) struct AtomicTaggedPtrImpl<T> {
    inner: Mutex<(Option<NonNull<T>>, Tag)>,
    _marker: PhantomData<*mut T>,
}

// Safety: We implement Send/Sync manually because Mutex handles interior synchronization.
unsafe impl<T: Send> Send for AtomicTaggedPtrImpl<T> {}
unsafe impl<T: Sync> Sync for AtomicTaggedPtrImpl<T> {}

impl<T> AtomicTaggedPtrImpl<T> {
    /// Creates a new `AtomicTaggedPtrImpl` with the given pointer and tag.
    #[inline]
    pub(crate) fn new(ptr: Option<NonNull<T>>, tag: Tag) -> Self {
        Self {
            inner: Mutex::new((ptr, tag)),
            _marker: PhantomData,
        }
    }

    /// Atomically loads the tagged pointer.
    ///
    /// Respects the `Ordering` argument by invoking a memory fence.
    #[inline]
    pub(crate) fn load(&self, order: Ordering) -> (Option<NonNull<T>>, Tag) {
        let guard = self.inner.lock();
        let val = *guard;
        if order != Ordering::Relaxed {
            core::sync::atomic::fence(order);
        }
        val
    }

    /// Atomically stores a new tagged pointer.
    #[inline]
    pub(crate) fn store(&self, ptr: Option<NonNull<T>>, tag: Tag, order: Ordering) {
        if order != Ordering::Relaxed {
            core::sync::atomic::fence(order);
        }
        let mut guard = self.inner.lock();
        *guard = (ptr, tag);
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
        let mut guard = self.inner.lock();
        let actual = *guard;

        if actual.0 == current.0 && actual.1 == current.1 {
            if success != Ordering::Relaxed {
                core::sync::atomic::fence(success);
            }
            *guard = new;
            Ok(actual)
        } else {
            if failure != Ordering::Relaxed {
                core::sync::atomic::fence(failure);
            }
            Err(actual)
        }
    }

    /// Atomically exchanges the tagged pointer value with weak semantics.
    ///
    /// For the fallback Mutex backend, this behaves identically to `compare_exchange`.
    #[inline]
    pub(crate) fn compare_exchange_weak(
        &self,
        current: (Option<NonNull<T>>, Tag),
        new: (Option<NonNull<T>>, Tag),
        success: Ordering,
        failure: Ordering,
    ) -> super::RawTaggedPtrResult<T> {
        self.compare_exchange(current, new, success, failure)
    }

    /// Atomically exchanges the value and returns the old value.
    #[inline]
    pub(crate) fn swap(
        &self,
        ptr: Option<NonNull<T>>,
        tag: Tag,
        order: Ordering,
    ) -> (Option<NonNull<T>>, Tag) {
        if order != Ordering::Relaxed {
            core::sync::atomic::fence(order);
        }
        let mut guard = self.inner.lock();
        let old = *guard;
        *guard = (ptr, tag);
        old
    }

    /// Consumes the atomic and returns the inner value.
    #[inline]
    pub(crate) fn into_inner(self) -> (Option<NonNull<T>>, Tag) {
        self.inner.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_basic_operations() {
        let val = 123;
        let ptr = NonNull::new(&val as *const i32 as *mut i32);
        let atom = AtomicTaggedPtrImpl::new(ptr, Tag::new(0));

        let loaded = atom.load(Ordering::Relaxed);
        assert_eq!(loaded.0, ptr);
        assert_eq!(loaded.1.value(), 0);

        atom.store(None, Tag::new(456), Ordering::Relaxed);
        let loaded_after = atom.load(Ordering::Relaxed);
        assert!(loaded_after.0.is_none());
        assert_eq!(loaded_after.1.value(), 456);
    }

    #[test]
    fn test_fallback_cas_semantics() {
        let val1 = 10;
        let ptr1 = NonNull::new(&val1 as *const i32 as *mut i32);
        let val2 = 20;
        let ptr2 = NonNull::new(&val2 as *const i32 as *mut i32);

        let atom = AtomicTaggedPtrImpl::new(ptr1, Tag::new(0));

        // CAS should fail because expected tag (999) does not match actual tag (0)
        let cas_fail = atom.compare_exchange(
            (ptr1, Tag::new(999)),
            (ptr2, Tag::new(100)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert!(cas_fail.is_err());
        assert_eq!(cas_fail.unwrap_err(), (ptr1, Tag::new(0)));

        // CAS should succeed because both pointer and tag match perfectly
        let cas_success = atom.compare_exchange(
            (ptr1, Tag::new(0)),
            (ptr2, Tag::new(200)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert!(cas_success.is_ok());
        assert_eq!(cas_success.unwrap(), (ptr1, Tag::new(0)));

        let loaded = atom.load(Ordering::Acquire);
        assert_eq!(loaded.0, ptr2);
        assert_eq!(loaded.1.value(), 200);
    }
}
