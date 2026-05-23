//! `atomic_tagged_ptr`
//!
//! A high-performance, zero-overhead, platform-adaptive atomic tagged pointer implementation.
//! Specially tailored for lock-free intrusive data structures (such as Treiber Stack)
//! with ABA protection, supporting 32-bit and 64-bit platforms, as well as 52/57-bit high
//! virtual address (such as Intel 5-level paging) without truncation or pointer corruption.

#![no_std]

// Standard library is required for Mutex downgrade fallback or testing
extern crate std;

pub use r#impl::AtomicTaggedPtr;
pub use r#impl::TAG_MASK;

pub mod r#impl;
