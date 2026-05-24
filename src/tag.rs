use core::fmt;

use crate::TAG_MASK;

/// Represents a generation tag used for ABA protection in `AtomicTaggedPtr`.
///
/// `Tag` wraps a platform-specific generation count and ensures that any operations
/// (like wrapping addition or creation) respect the hardware platform's limits and bit-width.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(pub(crate) usize);

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

    /// Performs wrapping subtraction on the tag value.
    #[inline]
    pub const fn wrapping_sub(self, rhs: usize) -> Self {
        Self::new(self.0.wrapping_sub(rhs))
    }

    /// Returns the next tag value, wrapping around on overflow.
    #[inline]
    pub const fn next(self) -> Self {
        self.wrapping_add(1)
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

impl From<u8> for Tag {
    #[inline]
    fn from(value: u8) -> Self {
        Self::new(value as usize)
    }
}

impl From<u16> for Tag {
    #[inline]
    fn from(value: u16) -> Self {
        Self::new(value as usize)
    }
}

impl From<u32> for Tag {
    #[inline]
    fn from(value: u32) -> Self {
        Self::new(value as usize)
    }
}

impl From<Tag> for usize {
    #[inline]
    fn from(tag: Tag) -> usize {
        tag.0
    }
}

impl core::ops::Add<usize> for Tag {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        self.wrapping_add(rhs)
    }
}

impl core::ops::AddAssign<usize> for Tag {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl core::ops::Sub<usize> for Tag {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        self.wrapping_sub(rhs)
    }
}

impl core::ops::SubAssign<usize> for Tag {
    #[inline]
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

impl core::ops::BitAnd<usize> for Tag {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: usize) -> Self::Output {
        Self::new(self.0 & rhs)
    }
}

impl core::ops::BitAnd<Tag> for Tag {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Tag) -> Self::Output {
        Self::new(self.0 & rhs.0)
    }
}

impl core::ops::BitAndAssign<usize> for Tag {
    #[inline]
    fn bitand_assign(&mut self, rhs: usize) {
        *self = *self & rhs;
    }
}

impl core::ops::BitAndAssign<Tag> for Tag {
    #[inline]
    fn bitand_assign(&mut self, rhs: Tag) {
        *self = *self & rhs.0;
    }
}

impl core::ops::BitOr<usize> for Tag {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: usize) -> Self::Output {
        Self::new(self.0 | rhs)
    }
}

impl core::ops::BitOr<Tag> for Tag {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Tag) -> Self::Output {
        Self::new(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign<usize> for Tag {
    #[inline]
    fn bitor_assign(&mut self, rhs: usize) {
        *self = *self | rhs;
    }
}

impl core::ops::BitOrAssign<Tag> for Tag {
    #[inline]
    fn bitor_assign(&mut self, rhs: Tag) {
        *self = *self | rhs.0;
    }
}

impl core::ops::BitXor<usize> for Tag {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: usize) -> Self::Output {
        Self::new(self.0 ^ rhs)
    }
}

impl core::ops::BitXor<Tag> for Tag {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Tag) -> Self::Output {
        Self::new(self.0 ^ rhs.0)
    }
}

impl core::ops::BitXorAssign<usize> for Tag {
    #[inline]
    fn bitxor_assign(&mut self, rhs: usize) {
        *self = *self ^ rhs;
    }
}

impl core::ops::BitXorAssign<Tag> for Tag {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Tag) {
        *self = *self ^ rhs.0;
    }
}

impl core::ops::Not for Tag {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self::new(!self.0)
    }
}