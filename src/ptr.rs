use core::fmt;
use core::ptr::NonNull;

/// A transparent wrapper around `Option<NonNull<T>>` returned by `AtomicTaggedPtr` operations.
///
/// It provides convenient helper methods to convert into raw const/mutable pointers,
/// access the underlying `Option<NonNull<T>>`, and supports direct comparisons.
#[repr(transparent)]
pub struct Ptr<T> {
    inner: Option<NonNull<T>>,
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
    /// Converts the pointer into a raw mutable pointer `*mut T`.
    /// Returns a null pointer if the underlying value is `None`.
    #[inline]
    pub fn as_mut_ptr(self) -> *mut T {
        self.inner
            .map(|p| p.as_ptr())
            .unwrap_or(core::ptr::null_mut())
    }

    /// Converts the pointer into a raw const pointer `*const T`.
    /// Returns a null pointer if the underlying value is `None`.
    #[inline]
    pub fn as_ptr(self) -> *const T {
        self.inner
            .map(|p| p.as_ptr() as *const T)
            .unwrap_or(core::ptr::null())
    }

    /// Obtains the underlying `Option<NonNull<T>>`.
    #[inline]
    pub const fn option(self) -> Option<NonNull<T>> {
        self.inner
    }

    /// Obtains the underlying `Option<NonNull<T>>`.
    #[inline]
    pub const fn as_option(self) -> Option<NonNull<T>> {
        self.inner
    }

    /// Returns `true` if the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_none()
    }

    /// Returns `true` if the pointer is not null (is some).
    #[inline]
    pub fn is_some(self) -> bool {
        self.inner.is_some()
    }

    /// Returns `true` if the pointer is null (is none).
    #[inline]
    pub fn is_none(self) -> bool {
        self.inner.is_none()
    }

    /// Returns a shared reference to the value if the pointer is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid, aligned, points to a valid initialized value,
    /// and respects the aliasing rules of Rust references.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        self.inner.map(|p| unsafe { p.as_ref() })
    }

    /// Returns a mutable reference to the value if the pointer is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid, aligned, points to a valid initialized value,
    /// and respects the aliasing rules of Rust references.
    #[inline]
    pub unsafe fn as_mut<'a>(mut self) -> Option<&'a mut T> {
        self.inner.as_mut().map(|p| unsafe { p.as_mut() })
    }

    /// Unwraps the inner `NonNull<T>`, panicking with the given message if it is `None`.
    #[inline]
    pub fn expect(self, msg: &str) -> NonNull<T> {
        self.inner.expect(msg)
    }

    /// Unwraps the inner `NonNull<T>`, panicking if it is `None`.
    #[inline]
    pub fn unwrap(self) -> NonNull<T> {
        self.inner
            .expect("called `Ptr::unwrap()` on a null pointer")
    }

    /// Returns the contained `NonNull<T>` or a default.
    #[inline]
    pub fn unwrap_or(self, default: NonNull<T>) -> NonNull<T> {
        self.inner.unwrap_or(default)
    }

    /// Maps the inner `NonNull<T>` pointer to a new pointer of another type.
    #[inline]
    pub fn map<U, F>(self, f: F) -> Ptr<U>
    where
        F: FnOnce(NonNull<T>) -> NonNull<U>,
    {
        Ptr::new(self.inner.map(f))
    }

    /// Maps the inner `NonNull<T>` pointer to a value, or returns a default value.
    #[inline]
    pub fn map_or<U, F>(self, default: U, f: F) -> U
    where
        F: FnOnce(NonNull<T>) -> U,
    {
        self.inner.map_or(default, f)
    }

    /// Maps the inner `NonNull<T>` pointer to a value, or evaluates a default closure.
    #[inline]
    pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: FnOnce() -> U,
        F: FnOnce(NonNull<T>) -> U,
    {
        self.inner.map_or_else(default, f)
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

impl<T> From<TaggedPtr<T>> for Option<NonNull<T>> {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        tagged.ptr.inner
    }
}

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

    /// Deconstructs the `TaggedPtr` into a tuple of `(Ptr<T>, Tag)`.
    #[inline]
    pub fn decompose(self) -> (Ptr<T>, crate::Tag) {
        (self.ptr, self.tag)
    }

    /// Converts the pointer into a raw const pointer `*const T`.
    /// Returns a null pointer if the underlying value is `None`.
    #[inline]
    pub fn as_ptr(self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Converts the pointer into a raw mutable pointer `*mut T`.
    /// Returns a null pointer if the underlying value is `None`.
    #[inline]
    pub fn as_mut_ptr(self) -> *mut T {
        self.ptr.as_mut_ptr()
    }

    /// Returns `true` if the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
        self.ptr.is_null()
    }

    /// Returns `true` if the pointer is not null (is some).
    #[inline]
    pub fn is_some(self) -> bool {
        self.ptr.is_some()
    }

    /// Returns `true` if the pointer is null (is none).
    #[inline]
    pub fn is_none(self) -> bool {
        self.ptr.is_none()
    }

    /// Returns a shared reference to the value if the pointer is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid, aligned, points to a valid initialized value,
    /// and respects the aliasing rules of Rust references.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        unsafe { self.ptr.as_ref() }
    }

    /// Returns a mutable reference to the value if the pointer is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid, aligned, points to a valid initialized value,
    /// and respects the aliasing rules of Rust references.
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        unsafe { self.ptr.as_mut() }
    }

    /// Returns a new `TaggedPtr` with a different pointer but the same tag.
    #[inline]
    pub fn with_ptr<U>(self, ptr: impl Into<Ptr<U>>) -> TaggedPtr<U> {
        TaggedPtr {
            ptr: ptr.into(),
            tag: self.tag,
        }
    }

    /// Returns a new `TaggedPtr` with a different tag but the same pointer.
    #[inline]
    pub fn with_tag(self, tag: crate::Tag) -> Self {
        Self { ptr: self.ptr, tag }
    }

    /// Maps the pointer part of the `TaggedPtr` using the given closure.
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
