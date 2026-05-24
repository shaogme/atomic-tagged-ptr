use core::fmt;
use core::ptr::NonNull;

use super::TaggedPtr;

/// A transparent wrapper around `Option<NonNull<T>>` returned by `AtomicTaggedPtr` operations.
///
/// It provides convenient helper methods to convert into raw const/mutable pointers,
/// access the underlying `Option<NonNull<T>>`, and supports direct comparisons.
#[repr(transparent)]
pub struct Ptr<T> {
    pub(crate) inner: Option<NonNull<T>>,
}

impl<T> Default for Ptr<T> {
    #[inline]
    fn default() -> Self {
        Self { inner: None }
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> Clone for Ptr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T> Ptr<T> {
    /// Creates a new `Ptr` wrapper from an `Option<NonNull<T>>`.
    #[inline]
    pub const fn new(ptr: Option<NonNull<T>>) -> Self {
        Self { inner: ptr }
    }

    /// Creates a null `Ptr`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    ///
    /// let p: Ptr<i32> = Ptr::null();
    /// assert!(p.is_null());
    /// ```
    #[inline]
    pub const fn null() -> Self {
        Self { inner: None }
    }

    /// Creates a null `Ptr` (alias for `null`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    ///
    /// let p: Ptr<i32> = Ptr::none();
    /// assert!(p.is_null());
    /// ```
    #[inline]
    pub const fn none() -> Self {
        Self { inner: None }
    }

    /// Casts to a pointer of another type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42u8;
    /// let ptr = Ptr::new(NonNull::new(&val as *const u8 as *mut u8));
    /// let casted: Ptr<i8> = ptr.cast();
    /// ```
    #[inline]
    pub fn cast<U>(self) -> Ptr<U> {
        Ptr {
            inner: self.inner.map(|p| p.cast()),
        }
    }

    /// Converts the pointer into a raw mutable pointer `*mut T`.
    ///
    /// Returns a null pointer if the underlying value is `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 42;
    /// let ptr = Ptr::new(NonNull::new(&mut val as *mut i32));
    /// assert_eq!(unsafe { *ptr.as_mut_ptr() }, 42);
    ///
    /// let null_ptr: Ptr<i32> = Ptr::null();
    /// assert!(null_ptr.as_mut_ptr().is_null());
    /// ```
    #[inline]
    pub fn as_mut_ptr(self) -> *mut T {
        self.inner
            .map(|p| p.as_ptr())
            .unwrap_or(core::ptr::null_mut())
    }

    /// Converts the pointer into a raw const pointer `*const T`.
    ///
    /// Returns a null pointer if the underlying value is `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// assert_eq!(unsafe { *ptr.as_ptr() }, 42);
    ///
    /// let null_ptr: Ptr<i32> = Ptr::null();
    /// assert!(null_ptr.as_ptr().is_null());
    /// ```
    #[inline]
    pub fn as_ptr(self) -> *const T {
        self.inner
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null())
    }

    /// Obtains the underlying `Option<NonNull<T>>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let raw = NonNull::new(&val as *const i32 as *mut i32);
    /// let ptr = Ptr::new(raw);
    /// assert_eq!(ptr.option(), raw);
    /// ```
    #[inline]
    pub const fn option(self) -> Option<NonNull<T>> {
        self.inner
    }

    /// Obtains the underlying `Option<NonNull<T>>` (alias for `option`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let raw = NonNull::new(&val as *const i32 as *mut i32);
    /// let ptr = Ptr::new(raw);
    /// assert_eq!(ptr.as_option(), raw);
    /// ```
    #[inline]
    pub const fn as_option(self) -> Option<NonNull<T>> {
        self.inner
    }

    /// Returns `true` if the pointer is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    ///
    /// let p: Ptr<i32> = Ptr::null();
    /// assert!(p.is_null());
    /// ```
    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_none()
    }

    /// Returns `true` if the pointer is not null (is some).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let p = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// assert!(p.is_some());
    /// ```
    #[inline]
    pub fn is_some(self) -> bool {
        self.inner.is_some()
    }

    /// Returns `true` if the pointer is null (is none).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    ///
    /// let p: Ptr<i32> = Ptr::null();
    /// assert!(p.is_none());
    /// ```
    #[inline]
    pub fn is_none(self) -> bool {
        self.inner.is_none()
    }

    /// Returns a shared reference to the value if the pointer is not null.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// unsafe {
    ///     assert_eq!(ptr.as_ref(), Some(&42));
    /// }
    /// ```
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        self.inner.map(|p| unsafe { p.as_ref() })
    }

    /// Returns a mutable reference to the value if the pointer is not null.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 42;
    /// let ptr = Ptr::new(NonNull::new(&mut val as *mut i32));
    /// unsafe {
    ///     let r = ptr.as_mut();
    ///     assert_eq!(r, Some(&mut 42));
    ///     *r.unwrap() = 100;
    /// }
    /// assert_eq!(val, 100);
    /// ```
    #[inline]
    pub unsafe fn as_mut<'a>(mut self) -> Option<&'a mut T> {
        self.inner.as_mut().map(|p| unsafe { p.as_mut() })
    }

    /// Unwraps the inner `NonNull<T>`, panicking with the given message if it is `None`.
    ///
    /// # Panics
    ///
    /// Panics if the pointer is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// let non_null = ptr.expect("should not be null");
    /// assert_eq!(unsafe { *non_null.as_ptr() }, 42);
    /// ```
    #[inline]
    pub fn expect(self, msg: &str) -> NonNull<T> {
        self.inner.expect(msg)
    }

    /// Unwraps the inner `NonNull<T>`, panicking if it is `None`.
    ///
    /// # Panics
    ///
    /// Panics if the pointer is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// let non_null = ptr.unwrap();
    /// assert_eq!(unsafe { *non_null.as_ptr() }, 42);
    /// ```
    #[inline]
    pub fn unwrap(self) -> NonNull<T> {
        self.inner
            .expect("called `Ptr::unwrap()` on a null pointer")
    }

    /// Returns the contained `NonNull<T>` or a default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val1 = 42;
    /// let val2 = 84;
    /// let non_null1 = NonNull::new(&val1 as *const i32 as *mut i32).unwrap();
    /// let non_null2 = NonNull::new(&val2 as *const i32 as *mut i32).unwrap();
    ///
    /// let ptr = Ptr::new(Some(non_null1));
    /// assert_eq!(ptr.unwrap_or(non_null2), non_null1);
    ///
    /// let null_ptr = Ptr::null();
    /// assert_eq!(null_ptr.unwrap_or(non_null2), non_null2);
    /// ```
    #[inline]
    pub fn unwrap_or(self, default: NonNull<T>) -> NonNull<T> {
        self.inner.unwrap_or(default)
    }

    /// Maps the inner `NonNull<T>` pointer to a new pointer of another type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42u8;
    /// let ptr = Ptr::new(NonNull::new(&val as *const u8 as *mut u8));
    /// let mapped = ptr.map(|p| p.cast::<i8>());
    /// assert_eq!(unsafe { *mapped.as_ptr() }, 42);
    /// ```
    #[inline]
    pub fn map<U, F>(self, f: F) -> Ptr<U>
    where
        F: FnOnce(NonNull<T>) -> NonNull<U>,
    {
        Ptr::new(self.inner.map(f))
    }

    /// Maps the inner `NonNull<T>` pointer to a value, or returns a default value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// let res = ptr.map_or(0, |p| unsafe { *p.as_ptr() });
    /// assert_eq!(res, 42);
    ///
    /// let null_ptr = Ptr::null();
    /// let res = null_ptr.map_or(0, |p| unsafe { *p.as_ptr() });
    /// assert_eq!(res, 0);
    /// ```
    #[inline]
    pub fn map_or<U, F>(self, default: U, f: F) -> U
    where
        F: FnOnce(NonNull<T>) -> U,
    {
        self.inner.map_or(default, f)
    }

    /// Maps the inner `NonNull<T>` pointer to a value, or evaluates a default closure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// let res = ptr.map_or_else(|| 0, |p| unsafe { *p.as_ptr() });
    /// assert_eq!(res, 42);
    ///
    /// let null_ptr = Ptr::null();
    /// let res = null_ptr.map_or_else(|| 0, |p| unsafe { *p.as_ptr() });
    /// assert_eq!(res, 0);
    /// ```
    #[inline]
    pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: FnOnce() -> U,
        F: FnOnce(NonNull<T>) -> U,
    {
        self.inner.map_or_else(default, f)
    }

    /// Reads the value from `self` without moving it. This leaves the memory in `self` unchanged.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// unsafe {
    ///     assert_eq!(ptr.read(), 42);
    /// }
    /// ```
    #[inline]
    pub unsafe fn read(self) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::read(self.as_ptr()) }
    }

    /// Performs a volatile read of the value from `self` without moving it.
    ///
    /// Volatile operations are intended for acting on I/O memory, and are never coalesced or
    /// eliminated by the compiler.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// unsafe {
    ///     assert_eq!(ptr.read_volatile(), 42);
    /// }
    /// ```
    #[inline]
    pub unsafe fn read_volatile(self) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::read_volatile(self.as_ptr()) }
    }

    /// Reads the value from `self` without moving it, without requiring alignment.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let val = 42;
    /// let ptr = Ptr::new(NonNull::new(&val as *const i32 as *mut i32));
    /// unsafe {
    ///     assert_eq!(ptr.read_unaligned(), 42);
    /// }
    /// ```
    #[inline]
    pub unsafe fn read_unaligned(self) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::read_unaligned(self.as_ptr()) }
    }

    /// Overwrites a memory location with the given value without reading or dropping the old value.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for writes (correctly aligned, etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 0;
    /// let ptr = Ptr::new(NonNull::new(&mut val as *mut i32));
    /// unsafe {
    ///     ptr.write(42);
    /// }
    /// assert_eq!(val, 42);
    /// ```
    #[inline]
    pub unsafe fn write(self, val: T) {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::write(self.as_mut_ptr(), val) }
    }

    /// Performs a volatile write of a memory location with the given value.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for writes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 0;
    /// let ptr = Ptr::new(NonNull::new(&mut val as *mut i32));
    /// unsafe {
    ///     ptr.write_volatile(42);
    /// }
    /// assert_eq!(val, 42);
    /// ```
    #[inline]
    pub unsafe fn write_volatile(self, val: T) {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::write_volatile(self.as_mut_ptr(), val) }
    }

    /// Overwrites a memory location with the given value without requiring alignment.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for writes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 0;
    /// let ptr = Ptr::new(NonNull::new(&mut val as *mut i32));
    /// unsafe {
    ///     ptr.write_unaligned(42);
    /// }
    /// assert_eq!(val, 42);
    /// ```
    #[inline]
    pub unsafe fn write_unaligned(self, val: T) {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::write_unaligned(self.as_mut_ptr(), val) }
    }

    /// Replaces the value at `self` with `val`, returning the old value.
    ///
    /// # Safety
    ///
    /// * The pointer must be non-null.
    /// * The pointer must be valid for reads and writes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val = 10;
    /// let ptr = Ptr::new(NonNull::new(&mut val as *mut i32));
    /// unsafe {
    ///     let old = ptr.replace(20);
    ///     assert_eq!(old, 10);
    /// }
    /// assert_eq!(val, 20);
    /// ```
    #[inline]
    pub unsafe fn replace(self, val: T) -> T {
        // Safety: The caller guarantees the pointer is valid.
        unsafe { core::ptr::replace(self.as_mut_ptr(), val) }
    }

    /// Swaps the values at `self` and `with`.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut val1 = 10;
    /// let mut val2 = 20;
    /// let ptr1 = Ptr::new(NonNull::new(&mut val1 as *mut i32));
    /// let ptr2 = Ptr::new(NonNull::new(&mut val2 as *mut i32));
    /// unsafe {
    ///     ptr1.swap(ptr2);
    /// }
    /// assert_eq!(val1, 20);
    /// assert_eq!(val2, 10);
    /// ```
    #[inline]
    pub unsafe fn swap(self, with: Ptr<T>) {
        // Safety: The caller guarantees both pointers are valid.
        unsafe { core::ptr::swap(self.as_mut_ptr(), with.as_mut_ptr()) }
    }

    /// Copies `count` items from `self` to `dest`. The source and destination may overlap.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [1, 2, 3];
    /// let ptr1 = Ptr::new(NonNull::new(&mut arr[0] as *mut i32));
    /// let ptr2 = Ptr::new(NonNull::new(&mut arr[1] as *mut i32));
    /// unsafe {
    ///     ptr1.copy_to(ptr2, 2);
    /// }
    /// assert_eq!(arr, [1, 1, 2]);
    /// ```
    #[inline]
    pub unsafe fn copy_to(self, dest: Ptr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { core::ptr::copy(self.as_ptr(), dest.as_mut_ptr(), count) }
    }

    /// Copies `count` items from `self` to `dest`. The source and destination must not overlap.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr1 = [1, 2, 3];
    /// let mut arr2 = [0, 0, 0];
    /// let ptr1 = Ptr::new(NonNull::new(&mut arr1[0] as *mut i32));
    /// let ptr2 = Ptr::new(NonNull::new(&mut arr2[0] as *mut i32));
    /// unsafe {
    ///     ptr1.copy_to_nonoverlapping(ptr2, 3);
    /// }
    /// assert_eq!(arr2, [1, 2, 3]);
    /// ```
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: Ptr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { core::ptr::copy_nonoverlapping(self.as_ptr(), dest.as_mut_ptr(), count) }
    }

    /// Copies `count` items from `src` to `self`. The source and destination may overlap.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [1, 2, 3];
    /// let ptr1 = Ptr::new(NonNull::new(&mut arr[1] as *mut i32));
    /// let ptr2 = Ptr::new(NonNull::new(&mut arr[0] as *mut i32));
    /// unsafe {
    ///     ptr1.copy_from(ptr2, 2);
    /// }
    /// assert_eq!(arr, [1, 1, 2]);
    /// ```
    #[inline]
    pub unsafe fn copy_from(self, src: Ptr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { core::ptr::copy(src.as_ptr(), self.as_mut_ptr(), count) }
    }

    /// Copies `count` items from `src` to `self`. The source and destination must not overlap.
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
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr1 = [1, 2, 3];
    /// let mut arr2 = [0, 0, 0];
    /// let ptr1 = Ptr::new(NonNull::new(&mut arr2[0] as *mut i32));
    /// let ptr2 = Ptr::new(NonNull::new(&mut arr1[0] as *mut i32));
    /// unsafe {
    ///     ptr1.copy_from_nonoverlapping(ptr2, 3);
    /// }
    /// assert_eq!(arr2, [1, 2, 3]);
    /// ```
    #[inline]
    pub unsafe fn copy_from_nonoverlapping(self, src: Ptr<T>, count: usize) {
        // Safety: The caller guarantees the pointers are valid.
        unsafe { core::ptr::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr(), count) }
    }

    /// Calculates the offset from a pointer.
    ///
    /// If the pointer is null, this returns a null pointer.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one byte past the end of the same allocated object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = Ptr::new(NonNull::new(&mut arr[0] as *mut i32));
    /// unsafe {
    ///     let offset_ptr = ptr.offset(1);
    ///     assert_eq!(offset_ptr.read(), 20);
    /// }
    /// ```
    #[inline]
    pub unsafe fn offset(self, count: isize) -> Self {
        Self {
            inner: self.inner.map(|p| unsafe { NonNull::new_unchecked(p.as_ptr().offset(count)) }),
        }
    }

    /// Calculates the offset from a pointer (positive offset).
    ///
    /// If the pointer is null, this returns a null pointer.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one byte past the end of the same allocated object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = Ptr::new(NonNull::new(&mut arr[0] as *mut i32));
    /// unsafe {
    ///     let offset_ptr = ptr.add(2);
    ///     assert_eq!(offset_ptr.read(), 30);
    /// }
    /// ```
    #[inline]
    pub unsafe fn add(self, count: usize) -> Self {
        Self {
            inner: self.inner.map(|p| unsafe { NonNull::new_unchecked(p.as_ptr().add(count)) }),
        }
    }

    /// Calculates the offset from a pointer (negative offset).
    ///
    /// If the pointer is null, this returns a null pointer.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one byte past the end of the same allocated object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = Ptr::new(NonNull::new(&mut arr[2] as *mut i32));
    /// unsafe {
    ///     let offset_ptr = ptr.sub(1);
    ///     assert_eq!(offset_ptr.read(), 20);
    /// }
    /// ```
    #[inline]
    pub unsafe fn sub(self, count: usize) -> Self {
        Self {
            inner: self.inner.map(|p| unsafe { NonNull::new_unchecked(p.as_ptr().sub(count)) }),
        }
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    ///
    /// If the pointer is null, this returns a null pointer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = Ptr::new(NonNull::new(&mut arr[0] as *mut i32));
    /// let offset_ptr = ptr.wrapping_offset(1);
    /// assert_eq!(unsafe { offset_ptr.read() }, 20);
    /// ```
    #[inline]
    pub fn wrapping_offset(self, count: isize) -> Self {
        Self {
            inner: self.inner.map(|p| unsafe { NonNull::new_unchecked(p.as_ptr().wrapping_offset(count)) }),
        }
    }

    /// Calculates the offset from a pointer using wrapping arithmetic (positive offset).
    ///
    /// If the pointer is null, this returns a null pointer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = Ptr::new(NonNull::new(&mut arr[0] as *mut i32));
    /// let offset_ptr = ptr.wrapping_add(2);
    /// assert_eq!(unsafe { offset_ptr.read() }, 30);
    /// ```
    #[inline]
    pub fn wrapping_add(self, count: usize) -> Self {
        Self {
            inner: self.inner.map(|p| unsafe { NonNull::new_unchecked(p.as_ptr().wrapping_add(count)) }),
        }
    }

    /// Calculates the offset from a pointer using wrapping arithmetic (negative offset).
    ///
    /// If the pointer is null, this returns a null pointer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atomic_tagged_ptr::Ptr;
    /// use std::ptr::NonNull;
    ///
    /// let mut arr = [10, 20, 30];
    /// let ptr = Ptr::new(NonNull::new(&mut arr[2] as *mut i32));
    /// let offset_ptr = ptr.wrapping_sub(1);
    /// assert_eq!(unsafe { offset_ptr.read() }, 20);
    /// ```
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> Self {
        Self {
            inner: self.inner.map(|p| unsafe { NonNull::new_unchecked(p.as_ptr().wrapping_sub(count)) }),
        }
    }
}

impl<T> fmt::Pointer for Ptr<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

impl<T> PartialOrd for Ptr<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Ptr<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl<T> From<Ptr<T>> for *const T {
    #[inline]
    fn from(ptr: Ptr<T>) -> Self {
        ptr.as_ptr()
    }
}

impl<T> From<Ptr<T>> for *mut T {
    #[inline]
    fn from(ptr: Ptr<T>) -> Self {
        ptr.as_mut_ptr()
    }
}

impl<T> From<Ptr<T>> for Option<*const T> {
    #[inline]
    fn from(ptr: Ptr<T>) -> Self {
        ptr.inner.map(|p| p.as_ptr() as *const T)
    }
}

impl<T> From<Ptr<T>> for Option<*mut T> {
    #[inline]
    fn from(ptr: Ptr<T>) -> Self {
        ptr.inner.map(|p| p.as_ptr())
    }
}

// --- PartialEq implementations to ensure seamless forward compatibility ---

impl<T> PartialEq for Ptr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for Ptr<T> {}

impl<T> core::hash::Hash for Ptr<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> PartialEq<Option<NonNull<T>>> for Ptr<T> {
    #[inline]
    fn eq(&self, other: &Option<NonNull<T>>) -> bool {
        self.inner == *other
    }
}

impl<T> PartialEq<NonNull<T>> for Ptr<T> {
    #[inline]
    fn eq(&self, other: &NonNull<T>) -> bool {
        self.inner == Some(*other)
    }
}

impl<T> PartialEq<*const T> for Ptr<T> {
    #[inline]
    fn eq(&self, other: &*const T) -> bool {
        self.as_ptr() == *other
    }
}

impl<T> PartialEq<*mut T> for Ptr<T> {
    #[inline]
    fn eq(&self, other: &*mut T) -> bool {
        self.as_mut_ptr() == *other
    }
}

// --- From / Into conversion implementations for Ptr<T> ---

impl<T> From<Option<NonNull<T>>> for Ptr<T> {
    #[inline]
    fn from(ptr: Option<NonNull<T>>) -> Self {
        Self { inner: ptr }
    }
}

impl<T> From<NonNull<T>> for Ptr<T> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self { inner: Some(ptr) }
    }
}

impl<T> From<*const T> for Ptr<T> {
    #[inline]
    fn from(ptr: *const T) -> Self {
        Self {
            inner: NonNull::new(ptr as *mut T),
        }
    }
}

impl<T> From<*mut T> for Ptr<T> {
    #[inline]
    fn from(ptr: *mut T) -> Self {
        Self {
            inner: NonNull::new(ptr),
        }
    }
}

impl<T> From<TaggedPtr<T>> for Ptr<T> {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        tagged.ptr
    }
}

impl<T> From<Ptr<T>> for Option<NonNull<T>> {
    #[inline]
    fn from(ptr: Ptr<T>) -> Self {
        ptr.inner
    }
}