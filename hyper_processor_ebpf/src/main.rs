#![no_std]
#![no_main]

use aya_bpf::{
    bindings::path,
    helpers::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_probe_read_kernel_str_bytes},
    macros::{lsm, map},
    maps::HashMap,
    programs::LsmContext,
};
use aya_log_ebpf::info;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnauthorizedInfo {
    pub path: [u8; 256],
    pub timestamp: u64,
}

#[map(name = "unauthorized_libs")]
static mut UNAUTHORIZED_LIBS: HashMap<u64, UnauthorizedInfo> = HashMap::with_max_entries(1024, 0);

#[map(name = "whitelist")]
static mut WHITELIST: HashMap<[u8; 64], u8> = HashMap::with_max_entries(256, 0);

const S_IXUSR: u16 = 0o100;
const S_IXGRP: u16 = 0o010;
const S_IXOTH: u16 = 0o001;

#[lsm(name = "check_file_open")]
pub fn check_file_open(ctx: LsmContext) -> i32 {
    match unsafe { try_check_file_open(ctx) } {
        Ok(ret) => ret,
        Err(_) => 0, // Allow on error
    }
}

unsafe fn try_check_file_open(ctx: LsmContext) -> Result<i32, i64> {
    // Get the file path
    let file: *const aya_bpf::bindings::file = ctx.arg(0);
    if file.is_null() {
        return Ok(0);
    }

    // Check if this is an executable file
    let f_path = &(*file).f_path;
    let dentry = f_path.dentry;
    if dentry.is_null() {
        return Ok(0);
    }

    let inode = (*dentry).d_inode;
    if inode.is_null() {
        return Ok(0);
    }

    let mode = (*inode).i_mode;
    
    // Check if file has execute permissions
    if (mode & (S_IXUSR | S_IXGRP | S_IXOTH) as u16) == 0 {
        return Ok(0); // Not executable
    }

    // Get the filename
    let mut path_buf = [0u8; 256];
    let name_ptr = (*dentry).d_name.name;
    let len = bpf_probe_read_kernel_str_bytes(name_ptr as *const u8, &mut path_buf)
        .unwrap_or(0) as usize;

    if len == 0 {
        return Ok(0);
    }

    // Check if it's a .so file
    if !is_shared_library(&path_buf, len) {
        return Ok(0);
    }

    // Check whitelist
    let mut whitelist_key = [0u8; 64];
    let copy_len = core::cmp::min(len, 64);
    whitelist_key[..copy_len].copy_from_slice(&path_buf[..copy_len]);

    if WHITELIST.get(&whitelist_key).is_some() {
        return Ok(0); // Whitelisted
    }

    // Log unauthorized library
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u64;
    
    let info = UnauthorizedInfo {
        path: path_buf,
        timestamp: bpf_ktime_get_ns(),
    };

    let _ = UNAUTHORIZED_LIBS.insert(&pid, &info, 0);
    
    info!(&ctx, "Unauthorized library detected: pid={}", pid);

    // Return -EACCES to block
    Ok(-13)
}

#[lsm(name = "check_exec")]
pub fn check_exec(ctx: LsmContext) -> i32 {
    // This can be used to monitor process execution
    // For now, just allow all
    0
}

fn is_shared_library(path: &[u8], len: usize) -> bool {
    if len < 3 {
        return false;
    }

    // Check for .so extension
    for i in 0..len.saturating_sub(3) {
        if path[i] == b'.' && path[i + 1] == b's' && path[i + 2] == b'o' {
            // Check if it's .so or .so.X
            if i + 3 == len || path[i + 3] == b'.' {
                return true;
            }
        }
    }

    false
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
} 