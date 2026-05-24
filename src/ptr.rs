use crate::traits::IntoOptionNonNull;
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

// Enable Ptr<T> to be directly passed into AtomicTaggedPtr APIs
impl<T> IntoOptionNonNull<T> for Ptr<T> {
    #[inline]
    fn into_option_non_null(self) -> Option<NonNull<T>> {
        self.inner
    }
}

/// A packaged representation of a pointer and a generation tag.
/// Used for atomic operations with `AtomicTaggedPtr`.
#[derive(PartialEq, Eq, Hash)]
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
        P: IntoOptionNonNull<T>,
    {
        Self {
            ptr: Ptr::new(ptr.into_option_non_null()),
            tag,
        }
    }

    /// Deconstructs the `TaggedPtr` into a tuple of `(Ptr<T>, Tag)`.
    #[inline]
    pub fn decompose(self) -> (Ptr<T>, crate::Tag) {
        (self.ptr, self.tag)
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

impl<T> From<TaggedPtr<T>> for (Ptr<T>, crate::Tag) {
    #[inline]
    fn from(tagged: TaggedPtr<T>) -> Self {
        (tagged.ptr, tagged.tag)
    }
}

impl<T> IntoOptionNonNull<T> for TaggedPtr<T> {
    #[inline]
    fn into_option_non_null(self) -> Option<NonNull<T>> {
        self.ptr.into_option_non_null()
    }
}
