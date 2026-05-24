# atomic-tagged-ptr

[![Crates.io](https://img.shields.io/crates/v/atomic-tagged-ptr.svg)](https://crates.io/crates/atomic-tagged-ptr)
[![Documentation](https://docs.rs/atomic-tagged-ptr/badge.svg)](https://docs.rs/atomic-tagged-ptr)
[![CI Status](https://github.com/shaogme/atomic-tagged-ptr/actions/workflows/ci.yml/badge.svg)](https://github.com/shaogme/atomic-tagged-ptr/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

[简体中文](README_CN.md) | English

A high-performance, zero-overhead, platform-adaptive atomic tagged pointer implementation in Rust. 

Specially tailored for lock-free intrusive data structures (such as Treiber Stack) with ABA protection, supporting 32-bit and 64-bit platforms, as well as 48-bit, 52-bit, and 57-bit virtual address spaces (such as Intel 5-level paging) without pointer truncation, provenance loss, or memory corruption.

---

## Table of Contents

- [Core Features](#core-features)
- [Hardware Realities \& Memory Layout Design](#hardware-realities--memory-layout-design)
- [Auto-Detection \& Environment Variables](#auto-detection--environment-variables)
- [Installation](#installation)
- [Usage Examples](#usage-examples)
  - [Concurrent Treiber Stack Implementation](#concurrent-treiber-stack-implementation)
- [API Overview](#api-overview)
- [Robust CI Testing in QEMU VM](#robust-ci-testing-in-qemu-vm)
- [License](#license)

---

## Core Features

- **Platform-Adaptive Memory Layout**: Dynamically adjusts layout to fit 48-bit, 52-bit, and 57-bit virtual address limits at build time, optimizing tag bits while preventing pointer corruption.
- **Zero Overhead**: Direct hardware mapping to atomic operations (`cmpxchg` on x86_64, `ldrex/strex` or `cas` on AArch64, `cmpxchg8b` on x86-32, `ldrd/strd` on ARMv7-A) without locking.
- **Preserves Strict Provenance**: Utilizes modern `core::ptr::with_exposed_provenance_mut` to avoid unsafe pointer hacks, aligning with the latest Rust strict provenance guidelines.
- **Robust Fallback**: Automatically falls back to standard Mutex-based synchronization on platforms lacking native 64-bit atomics (requires the `std` feature), ensuring 100% compilation and API consistency.
- **`#![no_std]` Support**: Core functions work in `no_std` environments by default (Mutex fallback requires `std`).
- **Comprehensive CI Verification**: Verified via real QEMU virtual machine emulation on GitHub Actions to test and assert correctness under actual 57-bit virtual address spaces (Intel 5-level paging).

---

## Hardware Realities & Memory Layout Design

In lock-free concurrent programming, the **ABA problem** frequently arises. The traditional mitigation involves pairing the physical pointer with a generation tag, updating both atomically. However, 64-bit pointer packing faces strict hardware constraints:

### 1. 64-bit Platforms (Standard 48-bit Virtual Address Space)
For standard 64-bit architectures (like Apple M-series, x86_64 with 4-level paging) where virtual addresses use at most 48 bits, we pack a **16-bit generation tag** and a **48-bit pointer** into a single 64-bit `usize` word.

```text
 63            48 47                                             0
+----------------+------------------------------------------------+
|  16-Bit Tag    |               48-Bit Pointer                   |
+----------------+------------------------------------------------+
```

### 2. 64-bit Platforms (Intel 5-level Paging / 57-bit Address Space / ARMv8.2 52-bit)
On modern servers with Intel 5-level paging (supporting up to 57-bit address spaces) or ARMv8.2 with 52-bit virtual addresses, assuming a 48-bit limit causes immediate truncation and kernel crashes. 

To resolve this, `atomic-tagged-ptr` automatically transitions to a **56-bit pointer** and **8-bit tag** layout. Since user-space addresses in 57-bit systems reside within the positive half `[0, 0x007f_ffff_ffff_ffff]`, the pointer fits perfectly within 56 bits without any truncation, while providing an 8-bit tag for ABA protection.

```text
 63        56 55                                                 0
+------------+----------------------------------------------------+
| 8-Bit Tag  |               56-Bit Pointer                       |
+------------+----------------------------------------------------+
```

### 3. 32-bit Platforms (Direct 64-bit Atomic Packing)
On 32-bit systems, the pointer fits entirely in 32 bits. We pair it with a **32-bit generation tag** to form a double-word 64-bit integer, and utilize native double-word 64-bit atomic operations (`cmpxchg8b` on x86, `ldrd/strd` on ARM) to perform lock-free CAS.

```text
 63                            32 31                             0
+--------------------------------+--------------------------------+
|          32-Bit Tag            |         32-Bit Pointer         |
+--------------------------------+--------------------------------+
```

### 4. Fallback Systems
On platforms where native 64-bit atomics are unavailable (or forced fallback is requested), `atomic-tagged-ptr` automatically switches to Mutex-based wrapping. The tag and pointer occupy full `usize` widths (providing maximum generation range) while offering identical API signatures.

---

## Auto-Detection & Environment Variables

The `build.rs` script automatically identifies target capabilities:
- **Apple Targets**: Statically defaults to the **48-bit layout** (16-bit tag), since Apple Silicon and iOS utilize 48-bit virtual address limits.
- **Local Native Compilation**: Runs run-time checks on the host OS. On Windows, it queries `GetSystemInfo` to determine virtual address space boundaries. On Linux, it probes `/proc/cpuinfo` and performs high-address `mmap` checks to see if 5-level paging is active.
- **Cross-Compilation / Unknown Targets**: Conservatively defaults to the **57-bit layout** (8-bit tag) to ensure absolute pointer safety.

### Control Environment Variables
- `ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR=48`: Override detection and force the 48-bit layout (16-bit tag, 256x stronger ABA protection).
- `ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR=57`: Override detection and force the 57-bit layout (8-bit tag).
- `ATOMIC_TAGGED_PTR_PRINT_AUTODETECT=true`: Output auto-detection diagnostic logs during crate compilation.

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
atomic-tagged-ptr = "0.1.0"
```

To use in `no_std` environments, disable the default features:

```toml
[dependencies]
atomic-tagged-ptr = { version = "0.1.0", default-features = false }
```

---

## Usage Examples

### Concurrent Treiber Stack Implementation

Below is a complete, concurrent lock-free Treiber Stack implementation using `AtomicTaggedPtr` to defend against the ABA problem:

```rust
use core::ptr::NonNull;
use core::sync::atomic::Ordering;
use atomic_tagged_ptr::{AtomicTaggedPtr, Tag};

/// A node in the intrusive Treiber Stack.
pub struct StackNode {
    pub value: usize,
    // Intrusive next pointer slot
    next: AtomicTaggedPtr<StackNode>,
}

/// A lock-free intrusive Treiber Stack.
pub struct TreiberStack {
    // Head pointer along with generation tag for ABA defense
    head: AtomicTaggedPtr<StackNode>,
}

impl TreiberStack {
    pub fn new() -> Self {
        Self {
            head: AtomicTaggedPtr::new(None),
        }
    }

    /// Pushes a node onto the stack.
    pub fn push(&self, node: &StackNode) {
        let node_ptr = NonNull::from(node);
        let mut bits = self.head.load(Ordering::Acquire);
        loop {
            // Link our node to the current head pointer
            node.next.store(bits.0, bits.1, Ordering::Release);

            // Increment the generation tag to defend against ABA
            let next_tag = bits.1.wrapping_add(1);

            match self.head.compare_exchange_weak(
                bits,
                (Some(node_ptr), next_tag),
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => bits = actual,
            }
        }
    }

    /// Pops a node from the stack.
    ///
    /// # Safety
    /// Popped nodes must remain valid. This example assumes static nodes or standard GC.
    pub unsafe fn pop(&self) -> Option<&StackNode> {
        let mut bits = self.head.load(Ordering::Acquire);
        loop {
            let head_ptr = bits.0.option()?;

            // Read the next node intrusively
            let next_state = head_ptr.as_ref().next.load(Ordering::Acquire);

            // Increment the generation tag to defend against ABA
            let next_tag = bits.1.wrapping_add(1);

            match self.head.compare_exchange_weak(
                bits,
                (next_state.0, next_tag),
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(head_ptr.as_ref()),
                Err(actual) => bits = actual,
            }
        }
    }
}
```

---

## API Overview

### `AtomicTaggedPtr<T>`
The core struct representing an atomic tagged pointer.
- `pub fn new<P>(ptr: P) -> Self where P: IntoOptionNonNull<T>`: Creates a new atomic tagged pointer initialized with the given pointer and tag 0. Supports `NonNull<T>`, `Option<NonNull<T>>`, `*const T`, and `*mut T`.
- `pub fn load(&self, order: Ordering) -> (Ptr<T>, Tag)`: Loads the pointer wrapper `Ptr<T>` and tag atomically.
- `pub fn store<P>(&self, ptr: P, tag: Tag, order: Ordering) where P: IntoOptionNonNull<T>`: Stores a new pointer and tag atomically.
- `pub fn compare_exchange<P1, P2>(&self, current: (P1, Tag), new: (P2, Tag), success: Ordering, failure: Ordering) -> TaggedPtrResult<T> where P1: IntoOptionNonNull<T>, P2: IntoOptionNonNull<T>`: Compares and exchanges the pointer and tag values. Supports mixing different pointer types.
- `pub fn compare_exchange_weak<P1, P2>(&self, current: (P1, Tag), new: (P2, Tag), success: Ordering, failure: Ordering) -> TaggedPtrResult<T> where P1: IntoOptionNonNull<T>, P2: IntoOptionNonNull<T>`: Weaker, more efficient variant of `compare_exchange` suitable for spin-loops.

### `Ptr<T>`
A pointer wrapper returned by `AtomicTaggedPtr` operations to facilitate raw pointer and `Option` conversions.
- `pub fn as_ptr(self) -> *const T`: Converts into a raw const pointer `*const T` (returns a null pointer if empty).
- `pub fn as_mut_ptr(self) -> *mut T`: Converts into a raw mutable pointer `*mut T` (returns a null pointer if empty).
- `pub fn option(self) -> Option<NonNull<T>>`: Extracts the underlying `Option<NonNull<T>>`.
- `pub fn as_option(self) -> Option<NonNull<T>>`: Extracts the underlying `Option<NonNull<T>>`.
- `pub fn is_null(self) -> bool` / `pub fn is_some(self) -> bool` / `pub fn is_none(self) -> bool`: Query the pointer status.
- Implements `PartialEq` allowing direct comparisons between `Ptr<T>` and `NonNull<T>`, `Option<NonNull<T>>`, or raw `*const T`/`*mut T` pointers.

### `IntoOptionNonNull<T>`
A trait implemented by types that can be unified into `Option<NonNull<T>>`. 
It is implemented for `NonNull<T>`, `Option<NonNull<T>>`, `*const T`, `*mut T`, and `Ptr<T>`.

### `Tag`
A wrapper around the platform-specific generation count.
- `pub const fn new(value: usize) -> Self`: Creates a new `Tag` with values masked to the platform limit.
- `pub const fn value(self) -> usize`: Unwraps the raw tag value.
- `pub const fn wrapping_add(self, rhs: usize) -> Self`: Performs wrapping addition within platform limits.
- `pub const fn max_value() -> Self`: Returns the maximum tag value allowed on the current platform layout.

---

## Robust CI Testing in QEMU VM

To ensure the auto-detection logic and bit-packing mechanism are bulletproof on modern high-address systems, our GitHub Actions workflow performs real-world emulation testing:
1. Compiles the test suite for x86_64.
2. Builds a lightweight bootable Linux initramfs.
3. Launches a **QEMU virtual machine** with `-cpu max` to enable full Intel 5-level paging (LA57) supporting 57-bit virtual addresses.
4. Mounts the workspace via 9p share, chroots into it, and executes the entire test suite.
5. Asserts that the compilation correctly detects 57-bit paging support and tests boundary pointers up to `0x007F_FFFF_FFFF_F000` to guarantee absolute safety under large virtual address spaces.

---

## License

This project is dual-licensed under:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

You may choose either license at your option.
