//! Standalone Windows P1 process working-set control, not the Grunkalunka runtime.
//! Compiled with the native rustc selected by canonical Windows CI.
//! Microsoft PSAPI: K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS.
//! No third-party dependencies, performance assertions or guest allocation claim.
use std::ffi::c_void;
use std::hint::black_box;
use std::mem::size_of;

#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn K32GetProcessMemoryInfo(
        process: *mut c_void,
        counters: *mut ProcessMemoryCounters,
        bytes: u32,
    ) -> i32;
}

fn observe() -> Result<(usize, usize), &'static str> {
    let mut counters = ProcessMemoryCounters {
        cb: size_of::<ProcessMemoryCounters>() as u32,
        ..Default::default()
    };
    // SAFETY: GetCurrentProcess returns a pseudo-handle for this process;
    // counters is a live, writable, correctly sized C-compatible structure.
    let bytes = counters.cb;
    let ok = unsafe {
        K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, bytes)
    };
    if ok == 0 {
        return Err("K32GetProcessMemoryInfo failed");
    }
    if counters.peak_working_set_size < counters.working_set_size {
        return Err("peak working set smaller than current working set");
    }
    Ok((counters.working_set_size, counters.peak_working_set_size))
}

fn run() -> Result<(), &'static str> {
    if size_of::<usize>() != 8 || size_of::<ProcessMemoryCounters>() != 72 {
        return Err("unexpected Windows x64 PROCESS_MEMORY_COUNTERS layout");
    }
    let before = observe()?;
    let mut allocation = vec![0u8; 4 * 1024 * 1024];
    for chunk in allocation.chunks_mut(4096) {
        chunk[0] = 7; // Touch each page; do not assume physical RSS must rise.
    }
    black_box(&allocation);
    let during = observe()?;
    drop(allocation);
    let after = observe()?;
    for (phase, (rss, peak)) in [
        ("before", before), ("during", during), ("after", after),
    ] {
        println!("{phase}\t{rss}\t{peak}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
