mod atomic;

pub use atomic::{AtomicTaggedPtr, TAG_MASK};

use core::fmt;
use core::ptr::NonNull;

use super::Ptr;

/// A packaged representation of a pointer and a generation tag.
/// Used for atomic operations with `AtomicTaggedPtr`.
pub struct TaggedPtr<T> {
    /// The physical pointer wrapper.
    pub ptr: Ptr<T>,
    /// The generation tag for ABA protection.
    pub tag: crate::Tag,
}

impl<T> Copy for TaggedPtr<T> {}

impl<T> Clone for TaggedPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for TaggedPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr && self.tag == other.tag
    }
}

impl<T> Eq for TaggedPtr<T> {}

impl<T> core::hash::Hash for TaggedPtr<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
        self.tag.hash(state);
    }
}

impl<T> Default for TaggedPtr<T> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: Ptr::default(),
            tag: crate::Tag::default(),
        }
    }
}

impl<T> TaggedPtr<T> {
    /// Creates a new `TaggedPtr` from a pointer and a tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// assert_eq!(tagged.ptr.option(), ptr);
    /// assert_eq!(tagged.tag.value(), 123);
    /// ```
    #[inline]
    pub fn new<P>(ptr: P, tag: crate::Tag) -> Self
    where
        P: Into<Ptr<T>>,
    {
        Self {
            ptr: ptr.into(),
            tag,
        }
    }

    /// Creates a new `TaggedPtr` with a null pointer and the default tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::TaggedPtr;
    ///
    /// let p: TaggedPtr<i32> = TaggedPtr::null();
    /// assert!(p.is_null());
    /// assert_eq!(p.tag.value(), 0);
    /// ```
    #[inline]
    pub const fn null() -> Self {
        Self {
            ptr: Ptr::null(),
            tag: crate::Tag::new(0),
        }
    }

    /// Casts the pointer part to a pointer of another type, keeping the tag unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42u8;
    /// let tagged = TaggedPtr::new(NonNull::new(&val as *const u8 as *mut u8), Tag::new(123));
    /// let casted: TaggedPtr<i8> = tagged.cast();
    /// assert_eq!(casted.tag.value(), 123);
    /// ```
    #[inline]
    pub fn cast<U>(self) -> TaggedPtr<U> {
        TaggedPtr {
            ptr: self.ptr.cast(),
            tag: self.tag,
        }
    }

    /// Deconstructs the `TaggedPtr` into a tuple of `(Ptr<T>, Tag)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag, Ptr};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// let (p, t) = tagged.decompose();
    /// assert_eq!(p.option(), ptr);
    /// assert_eq!(t.value(), 123);
    /// ```
    #[inline]
    pub fn decompose(self) -> (Ptr<T>, crate::Tag) {
        (self.ptr, self.tag)
    }

    /// Converts the pointer part into a raw const pointer `*const T`.
    /// Returns a null pointer if the underlying pointer is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// assert_eq!(unsafe { *tagged.as_ptr() }, 42);
    ///
    /// let null_tagged: TaggedPtr<i32> = TaggedPtr::null();
    /// assert!(null_tagged.as_ptr().is_null());
    /// ```
    #[inline]
    pub fn as_ptr(self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Converts the pointer part into a raw mutable pointer `*mut T`.
    /// Returns a null pointer if the underlying pointer is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 42;
    /// let ptr = NonNull::new(&mut val as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// assert_eq!(unsafe { *tagged.as_mut_ptr() }, 42);
    ///
    /// let null_tagged: TaggedPtr<i32> = TaggedPtr::null();
    /// assert!(null_tagged.as_mut_ptr().is_null());
    /// ```
    #[inline]
    pub fn as_mut_ptr(self) -> *mut T {
        self.ptr.as_mut_ptr()
    }

    /// Returns `true` if the pointer part is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::TaggedPtr;
    ///
    /// let p: TaggedPtr<i32> = TaggedPtr::null();
    /// assert!(p.is_null());
    /// ```
    #[inline]
    pub fn is_null(self) -> bool {
        self.ptr.is_null()
    }

    /// Returns `true` if the pointer part is not null (is some).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let p = TaggedPtr::new(NonNull::new(&val as *const i32 as *mut i32), Tag::new(10));
    /// assert!(p.is_some());
    /// ```
    #[inline]
    pub fn is_some(self) -> bool {
        self.ptr.is_some()
    }

    /// Returns `true` if the pointer part is null (is none).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::TaggedPtr;
    ///
    /// let p: TaggedPtr<i32> = TaggedPtr::null();
    /// assert!(p.is_none());
    /// ```
    #[inline]
    pub fn is_none(self) -> bool {
        self.ptr.is_none()
    }

    /// Returns a shared reference to the value if the pointer part is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// * The pointer is valid (aligned, points to a valid initialized value of type `T`).
    /// * The memory is not mutated while the reference is active.
    /// * The reference lifetime `'a` is correctly bounded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     assert_eq!(tagged.as_ref(), Some(&42));
    /// }
    /// ```
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        unsafe { self.ptr.as_ref() }
    }

    /// Returns a mutable reference to the value if the pointer part is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// * The pointer is valid (aligned, points to a valid initialized value of type `T`).
    /// * No other references (shared or mutable) to the same memory are active.
    /// * The reference lifetime `'a` is correctly bounded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 42;
    /// let ptr = NonNull::new(&mut val as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     let r = tagged.as_mut();
    ///     assert_eq!(r, Some(&mut 42));
    ///     *r.unwrap() = 100;
    /// }
    /// assert_eq!(val, 100);
    /// ```
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        unsafe { self.ptr.as_mut() }
    }

    /// Returns a new `TaggedPtr` with a different pointer but the same tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val1 = 10;
    /// let val2 = 20;
    /// let ptr1 = NonNull::new(&val1 as *const i32 as *mut i32);
    /// let ptr2 = NonNull::new(&val2 as *const i32 as *mut i32);
    ///
    /// let tagged = TaggedPtr::new(ptr1, Tag::new(100));
    /// let new_tagged = tagged.with_ptr(ptr2);
    /// assert_eq!(new_tagged.ptr.option(), ptr2);
    /// assert_eq!(new_tagged.tag.value(), 100);
    /// ```
    #[inline]
    pub fn with_ptr<U>(self, ptr: impl Into<Ptr<U>>) -> TaggedPtr<U> {
        TaggedPtr {
            ptr: ptr.into(),
            tag: self.tag,
        }
    }

    /// Returns a new `TaggedPtr` with a different tag but the same pointer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    ///
    /// let tagged = TaggedPtr::new(ptr, Tag::new(100));
    /// let new_tagged = tagged.with_tag(Tag::new(200));
    /// assert_eq!(new_tagged.ptr.option(), ptr);
    /// assert_eq!(new_tagged.tag.value(), 200);
    /// ```
    #[inline]
    pub fn with_tag(self, tag: crate::Tag) -> Self {
        Self { ptr: self.ptr, tag }
    }

    /// Maps the pointer part of the `TaggedPtr` using the given closure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42u8;
    /// let ptr = NonNull::new(&val as *const u8 as *mut u8);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(100));
    /// let mapped = tagged.map_ptr(|p| p.cast::<i8>());
    /// assert_eq!(unsafe { *mapped.as_ptr() }, 42);
    /// assert_eq!(mapped.tag.value(), 100);
    /// ```
    #[inline]
    pub fn map_ptr<U, F>(self, f: F) -> TaggedPtr<U>
    where
        F: FnOnce(Ptr<T>) -> Ptr<U>,
    {
        TaggedPtr {
            ptr: f(self.ptr),
            tag: self.tag,
        }
    }

    /// Reads the value from the pointer part without moving it. This leaves the memory unchanged.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for reads (correctly aligned, points to an initialized instance of `T`, etc.).
    /// * The memory must not be mutated by another thread while being read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     assert_eq!(tagged.read(), 42);
    /// }
    /// ```
    #[inline]
    pub unsafe fn read(self) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.read() }
    }

    /// Performs a volatile read of the value from the pointer part without moving it.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for reads.
    /// * The memory must not be mutated by another thread while being read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     assert_eq!(tagged.read_volatile(), 42);
    /// }
    /// ```
    #[inline]
    pub unsafe fn read_volatile(self) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.read_volatile() }
    }

    /// Reads the value from the pointer part without moving it, without requiring alignment.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for reads.
    /// * The memory must not be mutated by another thread while being read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = NonNull::new(&val as *const i32 as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     assert_eq!(tagged.read_unaligned(), 42);
    /// }
    /// ```
    #[inline]
    pub unsafe fn read_unaligned(self) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.read_unaligned() }
    }

    /// Overwrites the memory location at the pointer part with the given value.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for writes (correctly aligned, etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 0;
    /// let ptr = NonNull::new(&mut val as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     tagged.write(42);
    /// }
    /// assert_eq!(val, 42);
    /// ```
    #[inline]
    pub unsafe fn write(self, val: T) {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.write(val) }
    }

    /// Performs a volatile write to the memory location at the pointer part.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for writes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 0;
    /// let ptr = NonNull::new(&mut val as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     tagged.write_volatile(42);
    /// }
    /// assert_eq!(val, 42);
    /// ```
    #[inline]
    pub unsafe fn write_volatile(self, val: T) {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.write_volatile(val) }
    }

    /// Overwrites the memory location at the pointer part without requiring alignment.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for writes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 0;
    /// let ptr = NonNull::new(&mut val as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     tagged.write_unaligned(42);
    /// }
    /// assert_eq!(val, 42);
    /// ```
    #[inline]
    pub unsafe fn write_unaligned(self, val: T) {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.write_unaligned(val) }
    }

    /// Replaces the value at the pointer part with `val`, returning the old value.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for reads and writes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 10;
    /// let ptr = NonNull::new(&mut val as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(1));
    /// unsafe {
    ///     let old = tagged.replace(20);
    ///     assert_eq!(old, 10);
    /// }
    /// assert_eq!(val, 20);
    /// ```
    #[inline]
    pub unsafe fn replace(self, val: T) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { self.ptr.replace(val) }
    }

    /// Swaps the values at the pointer part of `self` and `with`.
    ///
    /// # Safety
    ///
    /// * Both pointers must be non-null.
    /// * Both pointers must be valid for reads and writes.
    /// * Both pointers must be properly aligned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut val1 = 10;
    /// let mut val2 = 20;
    /// let ptr1 = NonNull::new(&mut val1 as *mut i32);
    /// let ptr2 = NonNull::new(&mut val2 as *mut i32);
    /// let tagged1 = TaggedPtr::new(ptr1, Tag::new(1));
    /// let tagged2 = TaggedPtr::new(ptr2, Tag::new(2));
    /// unsafe {
    ///     tagged1.swap(tagged2);
    /// }
    /// assert_eq!(val1, 20);
    /// assert_eq!(val2, 10);
    /// ```
    #[inline]
    pub unsafe fn swap(self, with: TaggedPtr<T>) {
        // Safety: The caller guarantees both pointers are valid.
        unsafe { self.ptr.swap(with.ptr) }
    }

    /// Copies `count` items from the pointer part of `self` to `dest`. The regions may overlap.
    ///
    /// # Safety
    ///
    /// * Both pointers must be non-null.
    /// * Both pointers must be valid for reads and writes.
    /// * Both pointers must be properly aligned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [1, 2, 3];
    /// let ptr1 = NonNull::new(&mut arr[0] as *mut i32);
    /// let ptr2 = NonNull::new(&mut arr[1] as *mut i32);
    /// let tagged1 = TaggedPtr::new(ptr1, Tag::new(1));
    /// let tagged2 = TaggedPtr::new(ptr2, Tag::new(2));
    /// unsafe {
    ///     tagged1.copy_to(tagged2, 2);
    /// }
    /// assert_eq!(arr, [1, 1, 2]);
    /// ```
    #[inline]
    pub unsafe fn copy_to(self, dest: TaggedPtr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { self.ptr.copy_to(dest.ptr, count) }
    }

    /// Copies `count` items from the pointer part of `self` to `dest`. The regions must not overlap.
    ///
    /// # Safety
    ///
    /// * Both pointers must be non-null.
    /// * Both pointers must be valid for reads and writes.
    /// * Both pointers must be properly aligned.
    /// * The memory regions must not overlap.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr1 = [1, 2, 3];
    /// let mut arr2 = [0, 0, 0];
    /// let ptr1 = NonNull::new(&mut arr1[0] as *mut i32);
    /// let ptr2 = NonNull::new(&mut arr2[0] as *mut i32);
    /// let tagged1 = TaggedPtr::new(ptr1, Tag::new(1));
    /// let tagged2 = TaggedPtr::new(ptr2, Tag::new(2));
    /// unsafe {
    ///     tagged1.copy_to_nonoverlapping(tagged2, 3);
    /// }
    /// assert_eq!(arr2, [1, 2, 3]);
    /// ```
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: TaggedPtr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { self.ptr.copy_to_nonoverlapping(dest.ptr, count) }
    }

    /// Copies `count` items from `src` to the pointer part of `self`. The regions may overlap.
    ///
    /// # Safety
    ///
    /// * Both pointers must be non-null.
    /// * Both pointers must be valid for reads and writes.
    /// * Both pointers must be properly aligned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [1, 2, 3];
    /// let ptr1 = NonNull::new(&mut arr[1] as *mut i32);
    /// let ptr2 = NonNull::new(&mut arr[0] as *mut i32);
    /// let tagged1 = TaggedPtr::new(ptr1, Tag::new(1));
    /// let tagged2 = TaggedPtr::new(ptr2, Tag::new(2));
    /// unsafe {
    ///     tagged1.copy_from(tagged2, 2);
    /// }
    /// assert_eq!(arr, [1, 1, 2]);
    /// ```
    #[inline]
    pub unsafe fn copy_from(self, src: TaggedPtr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { self.ptr.copy_from(src.ptr, count) }
    }

    /// Copies `count` items from `src` to the pointer part of `self`. The regions must not overlap.
    ///
    /// # Safety
    ///
    /// * Both pointers must be non-null.
    /// * Both pointers must be valid for reads and writes.
    /// * Both pointers must be properly aligned.
    /// * The memory regions must not overlap.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr1 = [1, 2, 3];
    /// let mut arr2 = [0, 0, 0];
    /// let ptr1 = NonNull::new(&mut arr2[0] as *mut i32);
    /// let ptr2 = NonNull::new(&mut arr1[0] as *mut i32);
    /// let tagged1 = TaggedPtr::new(ptr1, Tag::new(1));
    /// let tagged2 = TaggedPtr::new(ptr2, Tag::new(2));
    /// unsafe {
    ///     tagged1.copy_from_nonoverlapping(tagged2, 3);
    /// }
    /// assert_eq!(arr2, [1, 2, 3]);
    /// ```
    #[inline]
    pub unsafe fn copy_from_nonoverlapping(self, src: TaggedPtr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { self.ptr.copy_from_nonoverlapping(src.ptr, count) }
    }

    /// Calculates the offset from the pointer part, returning a new `TaggedPtr` with the same tag.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one byte past the end of the same allocated object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = NonNull::new(&mut arr[0] as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// unsafe {
    ///     let offset_tagged = tagged.offset(1);
    ///     assert_eq!(offset_tagged.read(), 20);
    ///     assert_eq!(offset_tagged.tag.value(), 123);
    /// }
    /// ```
    #[inline]
    pub unsafe fn offset(self, count: isize) -> Self {
        Self {
            ptr: unsafe { self.ptr.offset(count) },
            tag: self.tag,
        }
    }

    /// Calculates the positive offset from the pointer part, returning a new `TaggedPtr` with the same tag.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one byte past the end of the same allocated object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = NonNull::new(&mut arr[0] as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// unsafe {
    ///     let offset_tagged = tagged.add(2);
    ///     assert_eq!(offset_tagged.read(), 30);
    ///     assert_eq!(offset_tagged.tag.value(), 123);
    /// }
    /// ```
    #[inline]
    pub unsafe fn add(self, count: usize) -> Self {
        Self {
            ptr: unsafe { self.ptr.add(count) },
            tag: self.tag,
        }
    }

    /// Calculates the negative offset from the pointer part, returning a new `TaggedPtr` with the same tag.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one byte past the end of the same allocated object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = NonNull::new(&mut arr[2] as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// unsafe {
    ///     let offset_tagged = tagged.sub(1);
    ///     assert_eq!(offset_tagged.read(), 20);
    ///     assert_eq!(offset_tagged.tag.value(), 123);
    /// }
    /// ```
    #[inline]
    pub unsafe fn sub(self, count: usize) -> Self {
        Self {
            ptr: unsafe { self.ptr.sub(count) },
            tag: self.tag,
        }
    }

    /// Calculates the wrapping offset from the pointer part, returning a new `TaggedPtr` with the same tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = NonNull::new(&mut arr[0] as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// let offset_tagged = tagged.wrapping_offset(1);
    /// assert_eq!(unsafe { offset_tagged.read() }, 20);
    /// assert_eq!(offset_tagged.tag.value(), 123);
    /// ```
    #[inline]
    pub fn wrapping_offset(self, count: isize) -> Self {
        Self {
            ptr: self.ptr.wrapping_offset(count),
            tag: self.tag,
        }
    }

    /// Calculates the positive wrapping offset from the pointer part, returning a new `TaggedPtr` with the same tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = NonNull::new(&mut arr[0] as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// let offset_tagged = tagged.wrapping_add(2);
    /// assert_eq!(unsafe { offset_tagged.read() }, 30);
    /// assert_eq!(offset_tagged.tag.value(), 123);
    /// ```
    #[inline]
    pub fn wrapping_add(self, count: usize) -> Self {
        Self {
            ptr: self.ptr.wrapping_add(count),
            tag: self.tag,
        }
    }

    /// Calculates the negative wrapping offset from the pointer part, returning a new `TaggedPtr` with the same tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::{TaggedPtr, Tag};
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = NonNull::new(&mut arr[2] as *mut i32);
    /// let tagged = TaggedPtr::new(ptr, Tag::new(123));
    /// let offset_tagged = tagged.wrapping_sub(1);
    /// assert_eq!(unsafe { offset_tagged.read() }, 20);
    /// assert_eq!(offset_tagged.tag.value(), 123);
    /// ```
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> Self {
        Self {
            ptr: self.ptr.wrapping_sub(count),
            tag: self.tag,
        }
    }
}

impl<T> fmt::Pointer for TaggedPtr<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

impl<T> PartialOrd for TaggedPtr<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for TaggedPtr<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.ptr.cmp(&other.ptr) {
            core::cmp::Ordering::Equal => self.tag.cmp(&other.tag),
            ord => ord,
        }
    }
}

impl<T> From<TaggedPtr<T>> for *const T {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        tagged.as_ptr()
    }
}

impl<T> From<TaggedPtr<T>> for *mut T {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        tagged.as_mut_ptr()
    }
}

impl<T> fmt::Debug for TaggedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaggedPtr")
            .field("ptr", &self.ptr)
            .field("tag", &self.tag)
            .finish()
    }
}

impl<T> From<(Ptr<T>, crate::Tag)> for TaggedPtr<T> {
    #[inline]
    fn from(tuple: (Ptr<T>, crate::Tag)) -> Self {
        Self {
            ptr: tuple.0,
            tag: tuple.1,
        }
    }
}

impl<T> From<(Option<NonNull<T>>, crate::Tag)> for TaggedPtr<T> {
    #[inline]
    fn from(tuple: (Option<NonNull<T>>, crate::Tag)) -> Self {
        Self {
            ptr: Ptr::new(tuple.0),
            tag: tuple.1,
        }
    }
}

impl<T> From<(NonNull<T>, crate::Tag)> for TaggedPtr<T> {
    #[inline]
    fn from(tuple: (NonNull<T>, crate::Tag)) -> Self {
        Self {
            ptr: Ptr::from(tuple.0),
            tag: tuple.1,
        }
    }
}

impl<T> From<(*const T, crate::Tag)> for TaggedPtr<T> {
    #[inline]
    fn from(tuple: (*const T, crate::Tag)) -> Self {
        Self {
            ptr: Ptr::from(tuple.0),
            tag: tuple.1,
        }
    }
}

impl<T> From<(*mut T, crate::Tag)> for TaggedPtr<T> {
    #[inline]
    fn from(tuple: (*mut T, crate::Tag)) -> Self {
        Self {
            ptr: Ptr::from(tuple.0),
            tag: tuple.1,
        }
    }
}

impl<T> From<TaggedPtr<T>> for (Ptr<T>, crate::Tag) {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        (tagged.ptr, tagged.tag)
    }
}

impl<T> From<TaggedPtr<T>> for Option<NonNull<T>> {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        tagged.ptr.inner
    }
}

impl<T> AsRef<Option<NonNull<T>>> for Ptr<T> {
    #[inline]
    fn as_ref(&self) -> &Option<NonNull<T>> {
        &self.inner
    }
}

impl<T> AsRef<Ptr<T>> for TaggedPtr<T> {
    #[inline]
    fn as_ref(&self) -> &Ptr<T> {
        &self.ptr
    }
}

impl<T> AsRef<crate::Tag> for TaggedPtr<T> {
    #[inline]
    fn as_ref(&self) -> &crate::Tag {
        &self.tag
    }
}

