//! `atomic_tagged_ptr`
//!
//! A high-performance, zero-overhead, platform-adaptive atomic tagged pointer implementation.
//! Specially tailored for lock-free intrusive data structures (such as Treiber Stack)
//! with ABA protection, supporting 32-bit and 64-bit platforms, as well as 52/57-bit high
//! virtual address (such as Intel 5-level paging) without truncation or pointer corruption.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub use r#impl::AtomicTaggedPtr;
pub use r#impl::TAG_MASK;
pub use r#impl::Tag;
pub use ptr::Ptr;
pub use traits::IntoOptionNonNull;

mod r#impl;
mod ptr;
mod traits;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctests {}

#[cfg(doctest)]
#[doc = include_str!("../README_CN.md")]
mod readme_cn_doctests {}

