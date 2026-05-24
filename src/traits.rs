use core::ptr::NonNull;

/// A helper trait to unify various raw pointer types (`NonNull<T>`, `Option<NonNull<T>>`, `*const T`, `*mut T`, etc.)
/// and convert them into `Option<NonNull<T>>`.
///
/// This trait optimizes the developer experience when passing pointer arguments to `AtomicTaggedPtr`.
pub trait IntoOptionNonNull<T> {
    /// Converts `self` into `Option<NonNull<T>>`.
    fn into_option_non_null(self) -> Option<NonNull<T>>;
}

impl<T> IntoOptionNonNull<T> for Option<NonNull<T>> {
    #[inline]
    fn into_option_non_null(self) -> Option<NonNull<T>> {
        self
    }
}

impl<T> IntoOptionNonNull<T> for NonNull<T> {
    #[inline]
    fn into_option_non_null(self) -> Option<NonNull<T>> {
        Some(self)
    }
}

impl<T> IntoOptionNonNull<T> for *const T {
    #[inline]
    fn into_option_non_null(self) -> Option<NonNull<T>> {
        NonNull::new(self as *mut T)
    }
}

impl<T> IntoOptionNonNull<T> for *mut T {
    #[inline]
    fn into_option_non_null(self) -> Option<NonNull<T>> {
        NonNull::new(self)
    }
}
