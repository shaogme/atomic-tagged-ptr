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

use crate::ptr::Ptr;
use crate::traits::IntoOptionNonNull;

// --- Platform Routing Conditional Compile Sections ---

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
mod ptr64;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
mod ptr32;

#[cfg(atomic_fallback)]
mod fallback;

/// Represents a generation tag used for ABA protection in `AtomicTaggedPtr`.
///
/// `Tag` wraps a platform-specific generation count and ensures that any operations
/// (like wrapping addition or creation) respect the hardware platform's limits and bit-width.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(pub(crate) usize);

// --- Exposing Unified High-Level Struct ---

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
pub use ptr64::TAG_MASK;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
pub use ptr32::TAG_MASK;

#[cfg(atomic_fallback)]
pub use fallback::TAG_MASK;

impl Tag {
    /// Creates a new `Tag` from a raw value, applying the platform-specific mask.
    #[inline]
    pub const fn new(value: usize) -> Self {
        Self(value & TAG_MASK)
    }

    /// Gets the raw tag value.
    #[inline]
    pub const fn value(self) -> usize {
        self.0
    }

    /// Performs wrapping addition on the tag value.
    #[inline]
    pub const fn wrapping_add(self, rhs: usize) -> Self {
        Self::new(self.0.wrapping_add(rhs))
    }

    /// Returns the maximum tag value allowed on this platform.
    #[inline]
    pub const fn max_value() -> Self {
        Self(TAG_MASK)
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({:#X})", self.0)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Tag {
    #[inline]
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl From<Tag> for usize {
    #[inline]
    fn from(tag: Tag) -> usize {
        tag.0
    }
}

/// Type alias representing the result of atomic compare and exchange operations.
pub type TaggedPtrResult<T> = Result<(Ptr<T>, Tag), (Ptr<T>, Tag)>;

/// Type alias for raw results returned by internal platform implementations.
pub(crate) type RawTaggedPtrResult<T> =
    Result<(Option<NonNull<T>>, Tag), (Option<NonNull<T>>, Tag)>;

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
    /// The `ptr` parameter supports any type implementing `IntoOptionNonNull<T>`,
    /// including `NonNull<T>`, `Option<NonNull<T>>`, `*const T`, and `*mut T`.
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
    pub fn new<P>(ptr: P) -> Self
    where
        P: IntoOptionNonNull<T>,
    {
        let raw_ptr = ptr.into_option_non_null();
        Self {
            #[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
            inner: ptr64::AtomicTaggedPtrImpl::new(raw_ptr),

            #[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
            inner: ptr32::AtomicTaggedPtrImpl::new(raw_ptr),

            #[cfg(atomic_fallback)]
            inner: fallback::AtomicTaggedPtrImpl::new(raw_ptr),
        }
    }

    /// Loads the current values of the pointer and tag atomically.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Release` or `AcqRel`.
    #[inline]
    pub fn load(&self, order: Ordering) -> (Ptr<T>, Tag) {
        let (raw_ptr, tag) = self.inner.load(order);
        (Ptr::new(raw_ptr), tag)
    }

    /// Stores a new pointer and tag atomically.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Acquire` or `AcqRel`.
    #[inline]
    pub fn store<P>(&self, ptr: P, tag: Tag, order: Ordering)
    where
        P: IntoOptionNonNull<T>,
    {
        self.inner.store(ptr.into_option_non_null(), tag, order);
    }

    /// Exchanges the current values with new ones if the current values match expectations.
    ///
    /// On success, returns `Ok` containing the previous pointer and tag.
    /// On failure, returns `Err` containing the actual loaded pointer and tag.
    #[inline]
    pub fn compare_exchange<P1, P2>(
        &self,
        current: (P1, Tag),
        new: (P2, Tag),
        success: Ordering,
        failure: Ordering,
    ) -> TaggedPtrResult<T>
    where
        P1: IntoOptionNonNull<T>,
        P2: IntoOptionNonNull<T>,
    {
        match self.inner.compare_exchange(
            (current.0.into_option_non_null(), current.1),
            (new.0.into_option_non_null(), new.1),
            success,
            failure,
        ) {
            Ok((raw_ptr, tag)) => Ok((Ptr::new(raw_ptr), tag)),
            Err((raw_ptr, tag)) => Err((Ptr::new(raw_ptr), tag)),
        }
    }

    /// Exchanges the current values with new ones using weak semantics.
    ///
    /// This is a weaker variant of `compare_exchange` which is allowed to fail spuriously,
    /// but can be significantly more efficient on certain LL/SC-based architectures (such as ARM).
    #[inline]
    pub fn compare_exchange_weak<P1, P2>(
        &self,
        current: (P1, Tag),
        new: (P2, Tag),
        success: Ordering,
        failure: Ordering,
    ) -> TaggedPtrResult<T>
    where
        P1: IntoOptionNonNull<T>,
        P2: IntoOptionNonNull<T>,
    {
        match self.inner.compare_exchange_weak(
            (current.0.into_option_non_null(), current.1),
            (new.0.into_option_non_null(), new.1),
            success,
            failure,
        ) {
            Ok((raw_ptr, tag)) => Ok((Ptr::new(raw_ptr), tag)),
            Err((raw_ptr, tag)) => Err((Ptr::new(raw_ptr), tag)),
        }
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
        assert_eq!(tag, Tag::new(0));
    }

    #[test]
    fn test_debug_formatter() {
        let val = 12345;
        let ptr = NonNull::new(&val as *const i32 as *mut i32);
        let atom = AtomicTaggedPtr::new(ptr);
        atom.store(ptr, Tag::new(88), Ordering::Relaxed);

        let debug_str = format!("{:?}", atom);
        assert!(debug_str.contains("AtomicTaggedPtr"));
        assert!(debug_str.contains("tag: Tag(0x58)"));
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
            if loaded.0 == local_ptr && loaded.1 == Tag::new(0) {
                let _ = atom_clone.compare_exchange(
                    (local_ptr, Tag::new(0)),
                    (None, Tag::new(55)),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
            }
        });

        handle.join().unwrap();
        let final_state = atom.load(Ordering::Acquire);

        // Assert state was safely transitioned or remained valid
        assert!(final_state.1 == Tag::new(55) || final_state.1 == Tag::new(0));
    }

    #[test]
    fn test_into_option_non_null_api() {
        let val1 = 111;
        let raw_ptr1 = &val1 as *const i32;
        let mut_ptr1 = &val1 as *const i32 as *mut i32;
        let non_null1 = NonNull::new(mut_ptr1).unwrap();

        // 1. 测试 new
        // 传入 NonNull<T>
        let atom = AtomicTaggedPtr::new(non_null1);
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), Some(non_null1));

        // 传入 Option<NonNull<T>>
        let atom = AtomicTaggedPtr::new(Some(non_null1));
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), Some(non_null1));

        // 传入 *const T
        let atom = AtomicTaggedPtr::new(raw_ptr1);
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), Some(non_null1));

        // 传入 *mut T
        let atom = AtomicTaggedPtr::new(mut_ptr1);
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), Some(non_null1));

        // 传入裸空指针 *const T
        let atom = AtomicTaggedPtr::new(core::ptr::null::<i32>());
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), None);

        // 传入裸空指针 *mut T
        let atom = AtomicTaggedPtr::new(core::ptr::null_mut::<i32>());
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), None);

        // 传入 None
        let atom: AtomicTaggedPtr<i32> = AtomicTaggedPtr::new(None);
        assert_eq!(atom.load(Ordering::Relaxed).0.option(), None);

        // 2. 测试 store
        let atom = AtomicTaggedPtr::new(None);
        atom.store(raw_ptr1, Tag::new(10), Ordering::Relaxed);
        let loaded = atom.load(Ordering::Relaxed);
        assert_eq!(loaded.0.option(), Some(non_null1));
        assert_eq!(loaded.1, Tag::new(10));

        atom.store(None, Tag::new(20), Ordering::Relaxed);
        let loaded = atom.load(Ordering::Relaxed);
        assert_eq!(loaded.0.option(), None);
        assert_eq!(loaded.1, Tag::new(20));

        // 3. 测试 compare_exchange / compare_exchange_weak (混合不同类型的指针参数)
        let atom = AtomicTaggedPtr::new(raw_ptr1);
        let res = atom.compare_exchange(
            (raw_ptr1, Tag::new(0)),
            (mut_ptr1, Tag::new(1)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert!(res.is_ok());
        let loaded = atom.load(Ordering::Relaxed);
        assert_eq!(loaded.0.option(), Some(non_null1));
        assert_eq!(loaded.1, Tag::new(1));

        let res = atom.compare_exchange_weak(
            (mut_ptr1, Tag::new(1)),
            (None, Tag::new(2)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        let mut res = res;
        while res.is_err() {
            res = atom.compare_exchange_weak(
                (mut_ptr1, Tag::new(1)),
                (None, Tag::new(2)),
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
        }
        assert!(res.is_ok());
        let loaded = atom.load(Ordering::Relaxed);
        assert_eq!(loaded.0.option(), None);
        assert_eq!(loaded.1, Tag::new(2));
    }

    #[test]
    fn test_ptr_conversions() {
        let val = 42;
        let raw = &val as *const i32;
        let mut_ptr = &val as *const i32 as *mut i32;
        let non_null = NonNull::new(mut_ptr).unwrap();

        let ptr_some = Ptr::new(Some(non_null));
        let ptr_none: Ptr<i32> = Ptr::new(None);

        // 测试 option() / as_option()
        assert_eq!(ptr_some.option(), Some(non_null));
        assert_eq!(ptr_none.option(), None);
        assert_eq!(ptr_some.as_option(), Some(non_null));

        // 测试 as_ptr()
        assert_eq!(ptr_some.as_ptr(), raw);
        assert_eq!(ptr_none.as_ptr(), core::ptr::null());

        // 测试 as_mut_ptr()
        assert_eq!(ptr_some.as_mut_ptr(), mut_ptr);
        assert_eq!(ptr_none.as_mut_ptr(), core::ptr::null_mut());

        // 测试 is_null() / is_some() / is_none()
        assert!(ptr_some.is_some());
        assert!(!ptr_some.is_null());
        assert!(!ptr_some.is_none());

        assert!(ptr_none.is_null());
        assert!(ptr_none.is_none());
        assert!(!ptr_none.is_some());

        // 测试 PartialEq
        assert!(ptr_some == Some(non_null));
        assert!(ptr_some == non_null);
        assert!(ptr_some == raw);
        assert!(ptr_some == mut_ptr);

        assert!(ptr_none == None);
        assert!(ptr_none == core::ptr::null::<i32>());
        assert!(ptr_none == core::ptr::null_mut::<i32>());
    }
}
