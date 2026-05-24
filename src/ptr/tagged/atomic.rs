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

use crate::{
    Tag,
    ptr::{Ptr, TaggedPtr},
};

// --- Platform Routing Conditional Compile Sections ---

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
mod ptr64;

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
use ptr64::AtomicTaggedPtrImpl;

#[cfg(all(target_pointer_width = "64", not(atomic_fallback)))]
pub use ptr64::TAG_MASK;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
mod ptr32;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
use ptr32::AtomicTaggedPtrImpl;

#[cfg(all(target_pointer_width = "32", not(atomic_fallback)))]
pub use ptr32::TAG_MASK;

#[cfg(atomic_fallback)]
mod fallback;

#[cfg(atomic_fallback)]
pub use fallback::TAG_MASK;

#[cfg(atomic_fallback)]
use fallback::AtomicTaggedPtrImpl;

/// Type alias representing the result of atomic compare and exchange operations.
pub type TaggedPtrResult<T> = Result<TaggedPtr<T>, TaggedPtr<T>>;

/// Type alias for raw results returned by internal platform implementations.
pub(crate) type RawTaggedPtrResult<T> =
    Result<(Option<NonNull<T>>, Tag), (Option<NonNull<T>>, Tag)>;

/// A platform-adaptive atomic tagged pointer supporting thread-safe ABA protection.
pub struct AtomicTaggedPtr<T> {
    inner: AtomicTaggedPtrImpl<T>,
}

// Safety: AtomicTaggedPtr is an atomic synchronizer wrapping pointer locations, safe to send/share across threads.
unsafe impl<T> Send for AtomicTaggedPtr<T> {}
unsafe impl<T> Sync for AtomicTaggedPtr<T> {}

impl<T> AtomicTaggedPtr<T> {
    /// Creates a new `AtomicTaggedPtr` initialized with the given tagged pointer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::ptr::NonNull;
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    ///
    /// let value = 42;
    /// let ptr = NonNull::new(&value as *const i32 as *mut i32);
    /// let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr, Tag::new(0)));
    /// ```
    #[inline]
    pub fn new(val: impl Into<TaggedPtr<T>>) -> Self {
        let val = val.into();
        Self {
            inner: AtomicTaggedPtrImpl::new(val.ptr.option(), val.tag),
        }
    }

    /// Loads the current values of the pointer and tag atomically.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Release` or `AcqRel`.
    #[inline]
    pub fn load(&self, order: Ordering) -> TaggedPtr<T> {
        let (raw_ptr, tag) = self.inner.load(order);
        TaggedPtr {
            ptr: Ptr::new(raw_ptr),
            tag,
        }
    }

    /// Stores a new pointer and tag atomically.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Acquire` or `AcqRel`.
    #[inline]
    pub fn store(&self, val: impl Into<TaggedPtr<T>>, order: Ordering) {
        let val = val.into();
        self.inner.store(val.ptr.option(), val.tag, order);
    }

    /// Stores a value into the pointer if the current value is the same as the `current` value.
    ///
    /// The return value is a result that indicates whether the new value was written and contains
    /// the previous value. On success, this value is guaranteed to be equal to `current`.
    ///
    /// `compare_exchange` takes two [`Ordering`] arguments to describe the memory model of this operation.
    /// The `success` ordering describes the memory ordering of the read-modify-write operation if the comparison
    /// succeeds. The `failure` ordering describes the memory ordering of the read operation if the comparison fails.
    ///
    /// # Panics
    ///
    /// Panics if `failure` is `Release` or `AcqRel`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    /// use std::ptr::NonNull;
    ///
    /// let val1 = 10;
    /// let val2 = 20;
    /// let ptr1 = NonNull::new(&val1 as *const i32 as *mut i32);
    /// let ptr2 = NonNull::new(&val2 as *const i32 as *mut i32);
    ///
    /// let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr1, Tag::new(1)));
    ///
    /// // Successful compare_exchange
    /// let res = atom.compare_exchange(
    ///     TaggedPtr::new(ptr1, Tag::new(1)),
    ///     TaggedPtr::new(ptr2, Tag::new(2)),
    ///     Ordering::SeqCst,
    ///     Ordering::SeqCst,
    /// );
    /// assert_eq!(res, Ok(TaggedPtr::new(ptr1, Tag::new(1))));
    /// assert_eq!(atom.load(Ordering::SeqCst), TaggedPtr::new(ptr2, Tag::new(2)));
    ///
    /// // Failed compare_exchange
    /// let res = atom.compare_exchange(
    ///     TaggedPtr::new(ptr1, Tag::new(1)),
    ///     TaggedPtr::new(ptr1, Tag::new(3)),
    ///     Ordering::SeqCst,
    ///     Ordering::SeqCst,
    /// );
    /// assert_eq!(res, Err(TaggedPtr::new(ptr2, Tag::new(2))));
    /// ```
    #[inline]
    pub fn compare_exchange(
        &self,
        current: impl Into<TaggedPtr<T>>,
        new: impl Into<TaggedPtr<T>>,
        success: Ordering,
        failure: Ordering,
    ) -> TaggedPtrResult<T> {
        let current = current.into();
        let new = new.into();
        match self.inner.compare_exchange(
            (current.ptr.option(), current.tag),
            (new.ptr.option(), new.tag),
            success,
            failure,
        ) {
            Ok((raw_ptr, tag)) => Ok(TaggedPtr {
                ptr: Ptr::new(raw_ptr),
                tag,
            }),
            Err((raw_ptr, tag)) => Err(TaggedPtr {
                ptr: Ptr::new(raw_ptr),
                tag,
            }),
        }
    }

    /// Stores a value into the pointer if the current value is the same as the `current` value.
    ///
    /// Unlike [`compare_exchange`], this function is allowed to spuriously fail even when the comparison
    /// succeeds, which can result in more efficient code generation on architectures that use load-link/store-conditional
    /// instructions (like ARM).
    ///
    /// `compare_exchange_weak` takes two [`Ordering`] arguments to describe the memory model of this operation.
    /// The `success` ordering describes the memory ordering of the read-modify-write operation if the comparison
    /// succeeds. The `failure` ordering describes the memory ordering of the read operation if the comparison fails.
    ///
    /// # Panics
    ///
    /// Panics if `failure` is `Release` or `AcqRel`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    /// use std::ptr::NonNull;
    ///
    /// let val1 = 10;
    /// let val2 = 20;
    /// let ptr1 = NonNull::new(&val1 as *const i32 as *mut i32);
    /// let ptr2 = NonNull::new(&val2 as *const i32 as *mut i32);
    ///
    /// let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr1, Tag::new(1)));
    ///
    /// let mut old = atom.load(Ordering::Relaxed);
    /// loop {
    ///     let new = TaggedPtr::new(ptr2, Tag::new(2));
    ///     match atom.compare_exchange_weak(old, new, Ordering::SeqCst, Ordering::SeqCst) {
    ///         Ok(_) => break,
    ///         Err(actual) => old = actual,
    ///     }
    /// }
    /// assert_eq!(atom.load(Ordering::SeqCst), TaggedPtr::new(ptr2, Tag::new(2)));
    /// ```
    #[inline]
    pub fn compare_exchange_weak(
        &self,
        current: impl Into<TaggedPtr<T>>,
        new: impl Into<TaggedPtr<T>>,
        success: Ordering,
        failure: Ordering,
    ) -> TaggedPtrResult<T> {
        let current = current.into();
        let new = new.into();
        match self.inner.compare_exchange_weak(
            (current.ptr.option(), current.tag),
            (new.ptr.option(), new.tag),
            success,
            failure,
        ) {
            Ok((raw_ptr, tag)) => Ok(TaggedPtr {
                ptr: Ptr::new(raw_ptr),
                tag,
            }),
            Err((raw_ptr, tag)) => Err(TaggedPtr {
                ptr: Ptr::new(raw_ptr),
                tag,
            }),
        }
    }

    /// Atomically exchanges the value and returns the old value.
    ///
    /// This method replaces the stored pointer and tag with `val` and returns the previously
    /// stored `TaggedPtr`.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `AcqRel`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    /// use std::ptr::NonNull;
    ///
    /// let val1 = 10;
    /// let val2 = 20;
    /// let ptr1 = NonNull::new(&val1 as *const i32 as *mut i32);
    /// let ptr2 = NonNull::new(&val2 as *const i32 as *mut i32);
    ///
    /// let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr1, Tag::new(1)));
    /// let old = atom.swap(TaggedPtr::new(ptr2, Tag::new(2)), Ordering::SeqCst);
    ///
    /// assert_eq!(old, TaggedPtr::new(ptr1, Tag::new(1)));
    /// assert_eq!(atom.load(Ordering::SeqCst), TaggedPtr::new(ptr2, Tag::new(2)));
    /// ```
    #[inline]
    pub fn swap(&self, val: impl Into<TaggedPtr<T>>, order: Ordering) -> TaggedPtr<T> {
        let val = val.into();
        let (raw_ptr, tag) = self.inner.swap(val.ptr.option(), val.tag, order);
        TaggedPtr {
            ptr: Ptr::new(raw_ptr),
            tag,
        }
    }

    /// Consumes the atomic and returns the inner value.
    ///
    /// This is safe because consuming `self` guarantees no other threads can concurrently
    /// access it, and therefore no synchronization is required.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr, Tag::new(100)));
    ///
    /// let inner = atom.into_inner();
    /// assert_eq!(inner.ptr.option(), ptr);
    /// assert_eq!(inner.tag.value(), 100);
    /// ```
    #[inline]
    pub fn into_inner(self) -> TaggedPtr<T> {
        let (raw_ptr, tag) = self.inner.into_inner();
        TaggedPtr {
            ptr: Ptr::new(raw_ptr),
            tag,
        }
    }

    /// Fetches the value, applies a function to it, and attempts to store the result.
    ///
    /// This is a convenience method for compare-and-swap loops. The function `f` is called with
    /// the current value, and can return `Some(new_value)` to attempt a CAS, or `None` to abort the update.
    ///
    /// On success, it returns `Ok(old_value)`. On failure (when `f` returns `None`), it returns `Err(old_value)`.
    ///
    /// `fetch_update` takes two [`Ordering`] arguments: `set_order` for the write ordering, and `fetch_order` for
    /// the read/load ordering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::default();
    ///
    /// // Increment the tag only if the tag is less than 5
    /// let res = atom.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |curr| {
    ///     if curr.tag.value() < 5 {
    ///         Some(curr.with_tag(curr.tag.wrapping_add(1)))
    ///     } else {
    ///         None
    ///     }
    /// });
    ///
    /// assert!(res.is_ok());
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 1);
    /// ```
    #[inline]
    pub fn fetch_update<F>(
        &self,
        set_order: Ordering,
        fetch_order: Ordering,
        mut f: F,
    ) -> Result<TaggedPtr<T>, TaggedPtr<T>>
    where
        F: FnMut(TaggedPtr<T>) -> Option<TaggedPtr<T>>,
    {
        let mut prev = self.load(fetch_order);
        while let Some(next) = f(prev) {
            match self.compare_exchange_weak(prev, next, set_order, fetch_order) {
                Ok(x) => return Ok(x),
                Err(next_prev) => prev = next_prev,
            }
        }
        Err(prev)
    }

    /// Atomically adds `val` to the tag of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::default();
    /// let old = atom.fetch_add_tag(5, Ordering::SeqCst);
    /// assert_eq!(old.tag.value(), 0);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 5);
    /// ```
    #[inline]
    pub fn fetch_add_tag(&self, val: usize, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        self.fetch_update(success, failure, |prev| {
            Some(prev.with_tag(prev.tag.wrapping_add(val)))
        })
        .unwrap()
    }

    /// Atomically subtracts `val` from the tag of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::new(TaggedPtr::new(None, Tag::new(10)));
    /// let old = atom.fetch_sub_tag(3, Ordering::SeqCst);
    /// assert_eq!(old.tag.value(), 10);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 7);
    /// ```
    #[inline]
    pub fn fetch_sub_tag(&self, val: usize, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        self.fetch_update(success, failure, |prev| {
            Some(prev.with_tag(prev.tag.wrapping_sub(val)))
        })
        .unwrap()
    }

    /// Atomically performs a bitwise AND on the tag of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::new(TaggedPtr::new(None, Tag::new(0b1111)));
    /// let old = atom.fetch_and_tag(0b1010, Ordering::SeqCst);
    /// assert_eq!(old.tag.value(), 0b1111);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 0b1010);
    /// ```
    #[inline]
    pub fn fetch_and_tag(&self, val: usize, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        self.fetch_update(success, failure, |prev| Some(prev.with_tag(prev.tag & val)))
            .unwrap()
    }

    /// Atomically performs a bitwise OR on the tag of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::new(TaggedPtr::new(None, Tag::new(0b0101)));
    /// let old = atom.fetch_or_tag(0b1010, Ordering::SeqCst);
    /// assert_eq!(old.tag.value(), 0b0101);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 0b1111);
    /// ```
    #[inline]
    pub fn fetch_or_tag(&self, val: usize, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        self.fetch_update(success, failure, |prev| Some(prev.with_tag(prev.tag | val)))
            .unwrap()
    }

    /// Atomically performs a bitwise XOR on the tag of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::new(TaggedPtr::new(None, Tag::new(0b0101)));
    /// let old = atom.fetch_xor_tag(0b1100, Ordering::SeqCst);
    /// assert_eq!(old.tag.value(), 0b0101);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 0b1001);
    /// ```
    #[inline]
    pub fn fetch_xor_tag(&self, val: usize, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        self.fetch_update(success, failure, |prev| Some(prev.with_tag(prev.tag ^ val)))
            .unwrap()
    }

    /// Atomically updates the pointer part of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag, Ptr};
    /// use std::sync::atomic::Ordering;
    /// use std::ptr::NonNull;
    ///
    /// let val1 = 42;
    /// let val2 = 84;
    /// let ptr1 = NonNull::new(&val1 as *const i32 as *mut i32);
    /// let ptr2 = NonNull::new(&val2 as *const i32 as *mut i32);
    ///
    /// let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr1, Tag::new(10)));
    /// let old = atom.fetch_set_ptr(ptr2, Ordering::SeqCst);
    /// assert_eq!(old.ptr.option(), ptr1);
    /// assert_eq!(old.tag.value(), 10);
    /// assert_eq!(atom.load(Ordering::SeqCst).ptr.option(), ptr2);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 10);
    /// ```
    #[inline]
    pub fn fetch_set_ptr(&self, ptr: impl Into<Ptr<T>>, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        let ptr = ptr.into();
        self.fetch_update(success, failure, |prev| Some(prev.with_ptr(ptr)))
            .unwrap()
    }

    /// Atomically updates the tag part of the current tagged pointer, returning the previous tagged pointer.
    ///
    /// This operation uses a CAS loop to perform the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{AtomicTaggedPtr, TaggedPtr, Tag, Ptr};
    /// use std::sync::atomic::Ordering;
    ///
    /// let atom = AtomicTaggedPtr::<i32>::default();
    /// let old = atom.fetch_set_tag(Tag::new(99), Ordering::SeqCst);
    /// assert_eq!(old.tag.value(), 0);
    /// assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 99);
    /// ```
    #[inline]
    pub fn fetch_set_tag(&self, tag: Tag, order: Ordering) -> TaggedPtr<T> {
        let (success, failure) = split_ordering(order);
        self.fetch_update(success, failure, |prev| Some(prev.with_tag(tag)))
            .unwrap()
    }
}

/// Splits a single `Ordering` into success and failure orderings for CAS loops.
#[inline]
const fn split_ordering(order: Ordering) -> (Ordering, Ordering) {
    match order {
        Ordering::SeqCst => (Ordering::SeqCst, Ordering::SeqCst),
        Ordering::AcqRel => (Ordering::AcqRel, Ordering::Acquire),
        Ordering::Acquire => (Ordering::Acquire, Ordering::Acquire),
        Ordering::Release => (Ordering::Release, Ordering::Relaxed),
        Ordering::Relaxed => (Ordering::Relaxed, Ordering::Relaxed),
        _ => (order, Ordering::Relaxed),
    }
}

// --- Common Trait Implementations ---

impl<T> Default for AtomicTaggedPtr<T> {
    #[inline]
    fn default() -> Self {
        Self::new(TaggedPtr::default())
    }
}

impl<T> fmt::Debug for AtomicTaggedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safe load under Relaxed ordering to capture debug state snapshot
        let val = self.load(Ordering::Relaxed);
        f.debug_struct("AtomicTaggedPtr")
            .field("pointer", &val.ptr)
            .field("tag", &val.tag)
            .finish()
    }
}

impl<T> From<TaggedPtr<T>> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(val: TaggedPtr<T>) -> Self {
        Self::new(val)
    }
}

impl<T> From<(Ptr<T>, Tag)> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(val: (Ptr<T>, Tag)) -> Self {
        Self::new(val)
    }
}

impl<T> From<Ptr<T>> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(ptr: Ptr<T>) -> Self {
        Self::new(TaggedPtr::new(ptr, Tag::default()))
    }
}

impl<T> From<Option<NonNull<T>>> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(ptr: Option<NonNull<T>>) -> Self {
        Self::new(TaggedPtr::new(ptr, Tag::default()))
    }
}

impl<T> From<NonNull<T>> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self::new(TaggedPtr::new(ptr, Tag::default()))
    }
}

impl<T> From<*const T> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(ptr: *const T) -> Self {
        Self::new(TaggedPtr::new(ptr, Tag::default()))
    }
}

impl<T> From<*mut T> for AtomicTaggedPtr<T> {
    #[inline]
    fn from(ptr: *mut T) -> Self {
        Self::new(TaggedPtr::new(ptr, Tag::default()))
    }
}

#[cfg(all(test, feature = "std"))]
mod tests;
