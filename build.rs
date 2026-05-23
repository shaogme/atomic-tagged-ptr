use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR");
    println!("cargo:rustc-check-cfg=cfg(virt_addr_48)");
    println!("cargo:rustc-check-cfg=cfg(atomic_fallback)");

    let target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    let has_atomic64 = env::var("CARGO_CFG_TARGET_HAS_ATOMIC")
        .map(|v| v.split(',').any(|s| s.trim() == "64"))
        .unwrap_or(false);
    let force_fallback = env::var("CARGO_FEATURE_FULL_POINTER_LOCKING").is_ok();

    // 1. Check if we need to use the Mutex-based fallback lock implementation
    let use_fallback = force_fallback
        || (target_pointer_width != "32" && target_pointer_width != "64")
        || !has_atomic64;

    if use_fallback {
        println!("cargo:rustc-cfg=atomic_fallback");
        println!("cargo:warning=[atomic-tagged-ptr] Native 64-bit atomics are unsupported or forced fallback is set. Enabling Mutex-based fallback implementation.");
        return;
    }

    // 2. Automatically detect virtual address space size for 64-bit platforms
    if target_pointer_width == "64" {
        // Support overriding via environment variable
        if let Ok(force_val) = env::var("ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR") {
            if force_val == "48" {
                println!("cargo:rustc-cfg=virt_addr_48");
                println!("cargo:warning=[atomic-tagged-ptr] Environment override: Enabling 48-bit virtual address layout (16-bit tag, 256x stronger ABA protection).");
                return;
            } else if force_val == "57" {
                println!("cargo:warning=[atomic-tagged-ptr] Environment override: Enabling 57-bit virtual address layout (8-bit tag).");
                return;
            }
        }

        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        
        // Automatically enable optimized layout for OSes known to have <= 48-bit virtual address spaces
        if target_os == "macos" || target_os == "ios" {
            println!("cargo:rustc-cfg=virt_addr_48");
            println!("cargo:warning=[atomic-tagged-ptr] Auto-detection: Apple platform detected, enabling 48-bit virtual address layout (16-bit tag).");
            return;
        }

        // Probe for Linux native compilation
        if target_os == "linux" {
            // Read cpuinfo only when compiling on Linux and target arch matches host arch
            if std::env::consts::OS == "linux" && env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == std::env::consts::ARCH {
                if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                    if !cpuinfo.contains("la57") {
                        // If la57 is absent, the CPU does not support 5-level paging, so virtual address space is <= 48-bit
                        println!("cargo:rustc-cfg=virt_addr_48");
                        println!("cargo:warning=[atomic-tagged-ptr] Auto-detection: Linux CPU does not support la57 (5-level paging), enabling 48-bit virtual address layout (16-bit tag).");
                        return;
                    }
                }
            }
        }

        // Default to conservative 57-bit virtual address layout (8-bit tag) for safety if unable to determine
        println!("cargo:warning=[atomic-tagged-ptr] Auto-detection: Unable to determine virtual address space limits, defaulting to conservative 57-bit layout (8-bit tag).");
    }
}

