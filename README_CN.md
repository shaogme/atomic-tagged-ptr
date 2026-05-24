# atomic-tagged-ptr

[![Crates.io](https://img.shields.io/crates/v/atomic-tagged-ptr.svg)](https://crates.io/crates/atomic-tagged-ptr)
[![Documentation](https://docs.rs/atomic-tagged-ptr/badge.svg)](https://docs.rs/atomic-tagged-ptr)
[![CI Status](https://github.com/shaogme/atomic-tagged-ptr/actions/workflows/ci.yml/badge.svg)](https://github.com/shaogme/atomic-tagged-ptr/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

简体中文 | [English](README.md)

一个高性能、零开销、平台自适应的 Rust 原子标记指针（Atomic Tagged Pointer）实现。

专为无锁侵入式数据结构（如 Treiber 栈）设计，提供坚固的 ABA 问题保护。支持 32 位和 64 位平台，并完美兼容 48 位、52 位以及 57 位虚拟地址空间布局（如 Intel 5 级分页），在确保高效率的同时绝不发生指针截断、来源丢失（Provenance Loss）或内存损坏。

---

## 目录

- [核心特性](#核心特性)
- [硬件现实与内存布局设计](#硬件现实与内存布局设计)
- [自适应探测与环境变量](#自适应探测与环境变量)
- [安装](#安装)
- [使用示例](#使用示例)
  - [并发 Treiber 栈实现](#并发-treiber-栈实现)
- [API 概览](#api-概览)
- [QEMU 虚拟机中的严苛-ci-测试](#qemu-虚拟机中的严苛-ci-测试)
- [开源许可](#开源许可)

---

## 核心特性

- **平台自适应内存布局**：在编译/运行时动态调整内存布局，完美适配 48 位、52 位和 57 位虚拟地址空间限制，在防止指针损坏的同时最大化标记（Tag）可用位数。
- **零额外开销**：直接映射到硬件级原子操作指令（x86_64 上的 `cmpxchg`，AArch64 上的 `ldrex/strex` 或 `cas`，x86-32 上的 `cmpxchg8b`，ARMv7-A 上的 `ldrd/strd`），无锁且高效。
- **遵循严格指针来源规范**：采用现代 Rust 标准库的 `core::ptr::with_exposed_provenance_mut` API，规避不安全的裸指针强转，完全符合 Rust 官方最新的指针来源（Strict Provenance）安全规范。
- **健壮的后备方案**：在缺少原生 64 位原子操作的平台上，自动且无缝地切换到基于 Mutex 的同步方案（需要启用 `std` 特性），保证 100% 的编译通过率与一致的 API 行为。若启用了 `parking_lot` 特性，将使用性能更好的 `parking_lot::Mutex` 代替 `std::sync::Mutex`。
- **支持 `#![no_std]`**：核心功能默认支持 `no_std` 环境（只有在不支持 64 位原子操作的平台上使用 Mutex 后备方案时才需要启用 `std`）。
- **完善的 CI 实机验证**：在 GitHub Actions 中启动真实的 QEMU 虚拟机，在真实的 57 位虚拟地址空间（Intel 5 级分页）内核下运行并通过全部测试用例。

---

## 硬件现实与内存布局设计

在无锁并发编程中，**ABA 问题**是一个经典的挑战。传统的解决方案是将物理指针与一个世代标记（Generation Tag）组合打包，并作为一个整体进行原子更新。然而，在 64 位指针上打包标记面临着严苛的硬件约束：

### 1. 64 位平台（标准 48 位虚拟地址空间）
在标准的 64 位架构（如 Apple M 系列芯片、使用 4 级页表的标准 x86_64）中，虚拟地址最多只占用低 48 位。我们把 **16 位的世代标记** 和 **48 位的物理指针** 打包进单个 64 位 `usize` 中。

```text
 63            48 47                                             0
+----------------+------------------------------------------------+
|  16 位标记 (Tag)|               48 位物理指针                     |
+----------------+------------------------------------------------+
```

### 2. 64 位平台（Intel 5 级分页 / 57 位虚拟地址空间 / ARMv8.2 52 位）
在支持 Intel 5 级分页（57 位虚拟地址空间）或 ARMv8.2（52 位虚拟地址空间）的现代高性能服务器上，假设 48 位指针上限会导致物理地址被截断，进而引发立即的内核恐慌（Kernel Panic）和崩溃。

为了解决这一问题，`atomic-tagged-ptr` 会自动平滑过渡到 **56 位物理指针** 与 **8 位世代标记** 的打包布局。由于 57 位系统下的用户空间地址全部位于正半区 `[0, 0x007f_ffff_ffff_ffff]`，因此 56 位空间可以完美容纳所有合法的用户空间指针而不会发生任何截断，同时保留 8 位用于 ABA 防护。

```text
 63        56 55                                                 0
+------------+----------------------------------------------------+
| 8位标记(Tag)|               56 位物理指针                        |
+------------+----------------------------------------------------+
```

### 3. 32 位平台（直接 64 位原子打包）
在 32 位系统上，物理指针大小刚好为 32 位。我们将它与一个 **32 位的世代标记** 配对，组成一个 64 位的双字（Double-Word）复合体，并调用原生的双字 64 位原子指令（如 x86 上的 `cmpxchg8b`，ARM 上的 `ldrd/strd`）实现零锁的高性能 CAS。

```text
 63                            32 31                             0
+--------------------------------+--------------------------------+
|          32 位标记 (Tag)        |         32 位物理指针           |
+--------------------------------+--------------------------------+
```

### 4. 健壮后备（Fallback）系统
在一些不支持原生 64 位原子操作的嵌入式芯片或超轻量微控制器上，`atomic-tagged-ptr` 会自动启用 Mutex 同步后端。此时标记和指针将分别占用完整的 `usize` 位宽（提供最大世代标记范围），并提供 100% 一致的 API。默认情况下使用 `std::sync::Mutex`（需要启用 `std` 特性），如果开启了 `parking_lot` 特性，则会使用 `parking_lot::Mutex` 作为同步后端。

---

## 自适应探测与环境变量

`build.rs` 脚本会在编译时智能分析目标平台的硬件能力：
- **Apple 平台**：默认静态配置为 **48 位布局**（16 位标记），因为 Apple Silicon 和 iOS 等系统有明确的 48 位虚拟地址上限。
- **本地原生编译**：在编译期对当前宿主系统进行运行时探测。在 Windows 上调用 `GetSystemInfo` API 检查当前操作系统激活的虚拟地址上限；在 Linux 上通过读取 `/proc/cpuinfo` 和执行高地址 `mmap` 探测当前内核是否启用了 5 级分页（LA57）。若未启用，则开启 48 位布局。
- **交叉编译 / 未知目标平台**：保守地默认使用 **57 位布局**（8 位标记），以确保在任何未知服务器环境下的物理指针绝对安全。

### 控制环境变量
- `ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR=48`：强制使用 48 位布局（16 位标记，可提供 256 倍更强的 ABA 世代防御力）。
- `ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR=57`：强制使用 57 位布局（8 位标记）。
- `ATOMIC_TAGGED_PTR_PRINT_AUTODETECT=true`：编译 Crate 时输出自适应探测的诊断日志警告。

---

## 安装

在你的 `Cargo.toml` 中添加以下依赖：

```toml
[dependencies]
atomic-tagged-ptr = "0.1.0"
```

如果需要在 `no_std` 环境下使用，请禁用默认特性：

```toml
[dependencies]
atomic-tagged-ptr = { version = "0.1.0", default-features = false }
```

---

## 使用示例

### 并发 Treiber 栈实现

下面是使用 `AtomicTaggedPtr` 构建的完整且无锁的并发侵入式 Treiber 栈实现，完美防御 ABA 问题：

```rust
use core::ptr::NonNull;
use core::sync::atomic::Ordering;
use atomic_tagged_ptr::{AtomicTaggedPtr, Tag};

/// 侵入式 Treiber 栈中的节点。
pub struct StackNode {
    pub value: usize,
    // 侵入式指向下一节点的原子标记指针
    next: AtomicTaggedPtr<StackNode>,
}

/// 并发无锁侵入式 Treiber 栈。
pub struct TreiberStack {
    // 栈顶指针，结合世代标记（Tag）以防御 ABA 问题
    head: AtomicTaggedPtr<StackNode>,
}

impl TreiberStack {
    pub fn new() -> Self {
        Self {
            head: AtomicTaggedPtr::new(None),
        }
    }

    /// 将节点压入栈顶。
    pub fn push(&self, node: &StackNode) {
        let node_ptr = NonNull::from(node);
        let mut bits = self.head.load(Ordering::Acquire);
        loop {
            // 将我们的节点链接到当前的栈顶指针上
            node.next.store(bits.0, bits.1, Ordering::Release);

            // 递增世代标记，以防御 ABA 问题
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

    /// 从栈顶弹出一个节点。
    ///
    /// # 安全性
    /// 被弹出的节点在并发访问中必须保持内存有效。此示例适用于静态分配节点或标准 GC 垃圾回收。
    pub unsafe fn pop(&self) -> Option<&StackNode> {
        let mut bits = self.head.load(Ordering::Acquire);
        loop {
            let head_ptr = bits.0.option()?;

            // 侵入式读取下一节点
            let next_state = head_ptr.as_ref().next.load(Ordering::Acquire);

            // 递增世代标记，以防御 ABA 问题
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

## API 概览

### `AtomicTaggedPtr<T>`
表示原子标记指针的核心结构体。
- `pub fn new<P>(ptr: P) -> Self where P: IntoOptionNonNull<T>`：创建一个新的原子标记指针，初始化为给定的指针和标记 0。支持传入 `NonNull<T>`、`Option<NonNull<T>>`、`*const T` 和 `*mut T`。
- `pub fn load(&self, order: Ordering) -> (Ptr<T>, Tag)`：原子地读取当前的物理指针封装 `Ptr<T>` 和世代标记。
- `pub fn store<P>(&self, ptr: P, tag: Tag, order: Ordering) where P: IntoOptionNonNull<T>`：原子地写入新的物理指针和世代标记。
- `pub fn compare_exchange<P1, P2>(&self, current: (P1, Tag), new: (P2, Tag), success: Ordering, failure: Ordering) -> TaggedPtrResult<T> where P1: IntoOptionNonNull<T>, P2: IntoOptionNonNull<T>`：原子地比较并交换指针与标记的值。支持混合传入不同的指针类型参数。
- `pub fn compare_exchange_weak<P1, P2>(&self, current: (P1, Tag), new: (P2, Tag), success: Ordering, failure: Ordering) -> TaggedPtrResult<T> where P1: IntoOptionNonNull<T>, P2: IntoOptionNonNull<T>`：具有较弱语义的 `compare_exchange` 变体，允许伪失败，在自旋锁或 LL/SC 架构（如 ARM）上效率更高。

### `Ptr<T>`
`AtomicTaggedPtr` 操作返回的指针封装结构体，便于进行裸指针和 Option 转换。
- `pub fn as_ptr(self) -> *const T`：转换成裸只读指针。若为空，返回空指针。
- `pub fn as_mut_ptr(self) -> *mut T`：转换成裸可写指针。若为空，返回空指针。
- `pub fn option(self) -> Option<NonNull<T>>`：获取底层的 `Option<NonNull<T>>`。
- `pub fn as_option(self) -> Option<NonNull<T>>`：获取底层的 `Option<NonNull<T>>`。
- `pub fn is_null(self) -> bool` / `pub fn is_some(self) -> bool` / `pub fn is_none(self) -> bool`：判断指针是否为空。
- 实现了 `PartialEq` 支持将 `Ptr<T>` 直接与 `NonNull<T>`、`Option<NonNull<T>>` 以及裸指针 `*const T`/`*mut T` 进行等值比较，确保完美的向前兼容性。

### `IntoOptionNonNull<T>`
用于在 API 接口中统一不同指针表示形态的 trait。
已经为 `NonNull<T>`、`Option<NonNull<T>>`、`*const T`、`*mut T` 以及 `Ptr<T>` 实现了此 trait。

### `Tag`
包裹平台专属世代计数值的类型安全包装器。
- `pub const fn new(value: usize) -> Self`：创建一个新的 `Tag`，超出当前平台布局上限的位会被掩码自动截断。
- `pub const fn value(self) -> usize`：获取原始的世代标记数值。
- `pub const fn wrapping_add(self, rhs: usize) -> Self`：在当前平台限制范围内进行回绕加法。
- `pub const fn max_value() -> Self`：返回当前平台布局下允许的最大标记值。

---

## QEMU 虚拟机中的严苛 CI 测试

为了确保我们的自适应探测逻辑和指针打包机制在现代极高地址（57 位虚拟空间）系统上完美无瑕，我们的 GitHub Actions 工作流进行了严苛的实机模拟测试：
1. 在 Ubuntu 环境下编译测试套件。
2. 动态打包生成一个极简的可引导 Linux initramfs 镜像。
3. 启动一个 **QEMU 虚拟机**，并传入 `-cpu max` 参数以彻底启用 Intel 5 级分页（LA57），模拟完整的 57 位虚拟地址硬件。
4. 通过 9p 共享目录挂载工作区，chroot 进入虚拟环境并执行全套测试用例。
5. 断言判定编译时探测逻辑已正确切换至 57 位模式，并测试高达 `0x007F_FFFF_FFFF_F000` 的物理指针打包，以证明在大地址空间服务器环境下的绝对安全。

---

## 开源许可

本项目遵循双重开源许可：
- **MIT 许可** ([LICENSE-MIT](LICENSE-MIT))
- **Apache 许可 2.0 版** ([LICENSE-APACHE](LICENSE-APACHE))

您可以根据个人喜好选择其中任意一种许可。
