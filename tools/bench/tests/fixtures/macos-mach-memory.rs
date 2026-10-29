//! Standalone P1 whole-process macOS memory ABI control, not runtime integration.
//! Apple XNU mach/task_info.h: MACH_TASK_BASIC_INFO (flavor 20).
//! No external crates or guest-specific memory attribution.
#![cfg(target_os = "macos")]

use std::hint::black_box;
use std::mem::{align_of, size_of};

const MACH_TASK_BASIC_INFO: u32 = 20;

#[repr(C, packed(4))]
#[derive(Default)]
struct MachTaskBasicInfo {
    virtual_size: u64,
    resident_size: u64,
    resident_size_max: u64,
    user_time: [i32; 2],
    system_time: [i32; 2],
    policy: i32,
    suspend_count: i32,
}

unsafe extern "C" {
    // mach_task_self() is a C macro reading the cached libSystem task port.
    // The self task port is borrowed; it must not be deallocated.
    static mut mach_task_self_: u32;
    fn task_info(
        task: u32,
        flavor: u32,
        info: *mut i32,
        count: *mut u32,
    ) -> i32;
}

fn observe() -> Result<(u64, u64), String> {
    // Apple declares the ABI under #pragma pack(push, 4).
    if size_of::<MachTaskBasicInfo>() != 48 || align_of::<MachTaskBasicInfo>() != 4 {
        return Err("unexpected mach_task_basic_info layout".into());
    }
    let mut info = MachTaskBasicInfo::default();
    let mut count = (size_of::<MachTaskBasicInfo>() / size_of::<i32>()) as u32;
    // SAFETY: mach_task_self_ is initialized by libSystem before main; the
    // initialized repr(C,packed(4)) output buffer is exactly 12 natural_t
    // slots (48 bytes). The self task port is borrowed, not deallocated.
    let status = unsafe {
        task_info(
            mach_task_self_,
            MACH_TASK_BASIC_INFO,
            (&mut info as *mut MachTaskBasicInfo).cast::<i32>(),
            &mut count,
        )
    };
    if status != 0 || count != 12 {
        return Err(format!("task_info(MACH_TASK_BASIC_INFO) failed: {status}/{count}"));
    }
    // Copy packed fields by value; never take a reference to a packed member.
    let resident = info.resident_size;
    let peak = info.resident_size_max;
    if resident == 0 || peak < resident {
        return Err("invalid Mach process resident/peak sizes".into());
    }
    Ok((resident, peak))
}

fn run() -> Result<(), String> {
    let before = observe()?;
    let mut allocation = vec![0u8; 4 * 1024 * 1024];
    for chunk in allocation.chunks_mut(4096) {
        chunk[0] = 7;
    }
    black_box(&allocation);
    let during = observe()?;
    drop(allocation);
    let after = observe()?;
    for (phase, (resident, peak)) in [
        ("before", before), ("during", during), ("after", after),
    ] {
        println!("{phase}\t{resident}\t{peak}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
