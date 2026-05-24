use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR");
    println!("cargo::rerun-if-env-changed=ATOMIC_TAGGED_PTR_PRINT_AUTODETECT");
    println!("cargo::rerun-if-env-changed=ATOMIC_TAGGED_PTR_TEST_EXPECT_48");
    println!("cargo::rerun-if-env-changed=ATOMIC_TAGGED_PTR_TEST_EXPECT_57");

    println!("cargo::rustc-check-cfg=cfg(virt_addr_48, values(none()))");
    println!("cargo::rustc-check-cfg=cfg(atomic_fallback, values(none()))");

    let target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    let has_atomic64 = env::var("CARGO_CFG_TARGET_HAS_ATOMIC")
        .map(|v| v.split(',').any(|s| s.trim() == "64"))
        .unwrap_or(false);
    let force_fallback = env::var("CARGO_FEATURE_FULL_POINTER_LOCKING").is_ok();

    let use_fallback = force_fallback
        || (target_pointer_width != "32" && target_pointer_width != "64")
        || !has_atomic64;

    if use_fallback {
        println!("cargo::rustc-cfg=atomic_fallback");
        println!(
            "cargo::warning=[atomic-tagged-ptr] Native 64-bit atomics are unsupported or forced fallback is set. Enabling Mutex-based fallback implementation."
        );
        return;
    }

    let mut is_48 = false;

    if target_pointer_width == "64" {
        'detect: {
            let print_autodetect = env::var("ATOMIC_TAGGED_PTR_PRINT_AUTODETECT")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

            if let Ok(force_val) = env::var("ATOMIC_TAGGED_PTR_FORCE_VIRT_ADDR") {
                if force_val == "48" {
                    is_48 = true;
                    println!("cargo::rustc-cfg=virt_addr_48");
                    println!(
                        "cargo::warning=[atomic-tagged-ptr] Environment override: Enabling 48-bit virtual address layout (16-bit tag, 256x stronger ABA protection)."
                    );
                    break 'detect;
                } else if force_val == "57" {
                    println!(
                        "cargo::warning=[atomic-tagged-ptr] Environment override: Enabling 57-bit virtual address layout (8-bit tag)."
                    );
                    break 'detect;
                }
            }

            let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

            if target_os == "macos"
                || target_os == "ios"
                || target_os == "watchos"
                || target_os == "tvos"
            {
                is_48 = true;
                println!("cargo::rustc-cfg=virt_addr_48");
                if print_autodetect {
                    println!(
                        "cargo::warning=[atomic-tagged-ptr] Auto-detection: Apple platform detected, enabling 48-bit virtual address layout (16-bit tag)."
                    );
                }
                break 'detect;
            }

            let host_os = std::env::consts::OS;
            let host_arch = std::env::consts::ARCH;
            let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

            // 只有本地编译时才进行运行时探测
            if host_os == target_os && host_arch == target_arch {
                if target_os == "windows" {
                    if !windows_la57_enabled() {
                        is_48 = true;
                        println!("cargo::rustc-cfg=virt_addr_48");
                        if print_autodetect {
                            println!(
                                "cargo::warning=[atomic-tagged-ptr] Auto-detection: Windows 5-level paging (LA57) is NOT active in the current OS, enabling 48-bit virtual address layout (16-bit tag)."
                            );
                        }
                        break 'detect;
                    }
                } else if target_os == "linux" && !linux_la57_enabled() {
                    is_48 = true;
                    println!("cargo::rustc-cfg=virt_addr_48");
                    if print_autodetect {
                        println!(
                            "cargo::warning=[atomic-tagged-ptr] Auto-detection: Linux 5-level paging is NOT active in the current kernel, enabling 48-bit virtual address layout (16-bit tag)."
                        );
                    }
                    break 'detect;
                }
            }

            if print_autodetect {
                println!(
                    "cargo::warning=[atomic-tagged-ptr] Auto-detection: Unable to determine virtual address space limits safely, defaulting to conservative 57-bit layout (8-bit tag)."
                );
            }
            break 'detect;
        }
    }

    let expect_48 = env::var("ATOMIC_TAGGED_PTR_TEST_EXPECT_48").is_ok();
    let expect_57 = env::var("ATOMIC_TAGGED_PTR_TEST_EXPECT_57").is_ok();

    if expect_48 {
        assert!(
            is_48,
            "[atomic-tagged-ptr] Compile-time assertion failed: Expected 48-bit virtual address layout, but detected 57-bit layout!"
        );
    }
    if expect_57 {
        assert!(
            !is_48,
            "[atomic-tagged-ptr] Compile-time assertion failed: Expected 57-bit virtual address layout, but detected 48-bit layout!"
        );
    }
}

fn windows_la57_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct SYSTEM_INFO {
            w_processor_architecture: u16,
            w_reserved: u16,
            dw_page_size: u32,
            lp_minimum_application_address: *mut std::ffi::c_void,
            lp_maximum_application_address: *mut std::ffi::c_void,
            dw_active_processor_mask: usize,
            dw_number_of_processors: u32,
            dw_processor_type: u32,
            dw_allocation_granularity: u32,
            w_processor_level: u16,
            w_processor_revision: u16,
        }

        unsafe extern "system" {
            fn GetSystemInfo(lpSystemInfo: *mut SYSTEM_INFO);
        }

        let mut sys_info = std::mem::MaybeUninit::<SYSTEM_INFO>::uninit();
        unsafe {
            GetSystemInfo(sys_info.as_mut_ptr());
            let sys_info = sys_info.assume_init();
            let max_addr = sys_info.lp_maximum_application_address as usize;
            max_addr > 0x0000_8000_0000_0000
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn linux_la57_enabled() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            if cpuinfo.contains(" la57 ") {
                return true;
            }
            for line in cpuinfo.lines() {
                if line.starts_with("address sizes") && line.contains("57 bits virtual") {
                    return true;
                }
            }
        }

        unsafe extern "C" {
            fn mmap(
                addr: *mut std::ffi::c_void,
                len: usize,
                prot: std::ffi::c_int,
                flags: std::ffi::c_int,
                fd: std::ffi::c_int,
                offset: isize,
            ) -> *mut std::ffi::c_void;
            fn munmap(addr: *mut std::ffi::c_void, len: usize) -> std::ffi::c_int;
        }
        const MAP_ANONYMOUS: std::ffi::c_int = 0x20;
        const MAP_PRIVATE: std::ffi::c_int = 0x02;
        const PROT_READ: std::ffi::c_int = 0x01;
        const PROT_WRITE: std::ffi::c_int = 0x02;

        let hint_addr = 0x0010_0000_0000_0000 as *mut std::ffi::c_void;
        let map_len = 4096;
        unsafe {
            let ret = mmap(
                hint_addr,
                map_len,
                PROT_READ | PROT_WRITE,
                MAP_ANONYMOUS | MAP_PRIVATE,
                -1,
                0,
            );
            if ret != -1isize as *mut std::ffi::c_void {
                let addr_val = ret as usize;
                munmap(ret, map_len);
                addr_val > 0x0000_7FFF_FFFF_FFFF
            } else {
                false
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
