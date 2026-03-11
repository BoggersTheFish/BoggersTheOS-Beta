//! Syscall interface for BoggersTheOS-Beta. TS RULE: every syscall checks caller weight — kernel supremacy.
//!
//! Invoked via int 0x80 (rax = number, rdi/rsi/rdx = args). Also callable from kernel stub task.

use crate::ts::{self, current_node_weight};
use crate::println;

/// Syscall numbers (match userspace / int 0x80 convention).
pub const SYS_WRITE: u64 = 0;
pub const SYS_EXIT: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_GET_NODE_WEIGHT: u64 = 3;
pub const SYS_DEBUG_HIERARCHY_DUMP: u64 = 4;
pub const SYS_FS_READ: u64 = 5;
pub const SYS_FS_WRITE: u64 = 6;
pub const SYS_FS_LIST: u64 = 7;
pub const SYS_NET_SEND: u64 = 8;
pub const SYS_NET_RECV: u64 = 9;

/// Min weight for "write" (low privilege).
const MIN_WEIGHT_WRITE: f32 = 0.3;
/// Min weight for exit/yield (basic control).
const MIN_WEIGHT_EXIT_YIELD: f32 = 0.3;
/// Min weight for get_node_weight (read own weight).
const MIN_WEIGHT_GET_NODE_WEIGHT: f32 = 0.3;
/// Min weight for debug hierarchy dump (sensitive).
const MIN_WEIGHT_DEBUG_DUMP: f32 = 0.6;
/// Min weight for fs read/list.
const MIN_WEIGHT_FS_READ_LIST: f32 = 0.5;
/// Min weight for fs write.
const MIN_WEIGHT_FS_WRITE: f32 = 0.6;
/// Min weight for net send.
const MIN_WEIGHT_NET_SEND: f32 = 0.7;
/// Min weight for net recv.
const MIN_WEIGHT_NET_RECV: f32 = 0.65;

/// Max bytes per SYS_WRITE to avoid abuse.
const MAX_WRITE_LEN: usize = 2048;
const MAX_FS_FILENAME_LEN: usize = 256;
const MAX_FS_DATA_LEN: usize = 64 * 1024;
const MAX_NET_PACKET_LEN: usize = 2048;

/// TS RULE: Syscall entry — check caller node weight vs syscall min; deny low-weight dangerous calls.
/// Returns value for rax (e.g. 0 = ok, negative-ish = error); for SYS_GET_NODE_WEIGHT returns weight bits.
/// d is 4th arg (r10) for FS syscalls.
pub fn dispatch(num: u64, a: u64, b: u64, c: u64, d: u64) -> u64 {
    let ret = match num {
        SYS_WRITE => {
            if ts::enforce_min_weight("sys_write", MIN_WEIGHT_WRITE).is_err() {
                return u64::MAX; // -1 as error
            }
            sys_write(a, b)
        }
        SYS_EXIT => {
            if ts::enforce_min_weight("sys_exit", MIN_WEIGHT_EXIT_YIELD).is_err() {
                return u64::MAX;
            }
            sys_exit(a)
        }
        SYS_YIELD => {
            if ts::enforce_min_weight("sys_yield", MIN_WEIGHT_EXIT_YIELD).is_err() {
                return u64::MAX;
            }
            sys_yield()
        }
        SYS_GET_NODE_WEIGHT => {
            if ts::enforce_min_weight("sys_get_node_weight", MIN_WEIGHT_GET_NODE_WEIGHT).is_err() {
                return u64::MAX;
            }
            sys_get_node_weight()
        }
        SYS_DEBUG_HIERARCHY_DUMP => {
            if ts::enforce_min_weight("sys_debug_hierarchy_dump", MIN_WEIGHT_DEBUG_DUMP).is_err() {
                return u64::MAX;
            }
            sys_debug_hierarchy_dump()
        }
        SYS_FS_READ => {
            if ts::enforce_min_weight("sys_fs_read", MIN_WEIGHT_FS_READ_LIST).is_err() {
                return u64::MAX;
            }
            sys_fs_read(a, b, c, d)
        }
        SYS_FS_WRITE => {
            if ts::enforce_min_weight("sys_fs_write", MIN_WEIGHT_FS_WRITE).is_err() {
                return u64::MAX;
            }
            sys_fs_write(a, b, c, d)
        }
        SYS_FS_LIST => {
            if ts::enforce_min_weight("sys_fs_list", MIN_WEIGHT_FS_READ_LIST).is_err() {
                return u64::MAX;
            }
            sys_fs_list()
        }
        SYS_NET_SEND => {
            if ts::enforce_min_weight("sys_net_send", MIN_WEIGHT_NET_SEND).is_err() {
                return u64::MAX;
            }
            sys_net_send(a, b)
        }
        SYS_NET_RECV => {
            if ts::enforce_min_weight("sys_net_recv", MIN_WEIGHT_NET_RECV).is_err() {
                return u64::MAX;
            }
            sys_net_recv(a, b)
        }
        _ => {
            println!("syscall unknown: {}", num);
            u64::MAX
        }
    };
    ret
}

fn sys_write(ptr: u64, len: u64) -> u64 {
    let len = len as usize;
    if len > MAX_WRITE_LEN {
        return u64::MAX;
    }
    if len == 0 {
        return 0;
    }
    let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
    for &b in slice {
        crate::print!("{}", b as char);
    }
    len as u64
}

fn sys_exit(_code: u64) -> u64 {
    // Minimal: just log. Full OS would mark task as exited.
    println!("sys_exit called (stub)");
    0
}

fn sys_yield() -> u64 {
    // Return from handler; executor will run other tasks when we're cooperative.
    0
}

fn sys_get_node_weight() -> u64 {
    let w = current_node_weight();
    w.to_bits() as u64
}

fn sys_debug_hierarchy_dump() -> u64 {
    ts::print_hierarchy_dump();
    0
}

/// a=filename_ptr, b=filename_len, c=buffer_ptr, d=buffer_len. Returns bytes read or u64::MAX.
fn sys_fs_read(filename_ptr: u64, filename_len: u64, buffer_ptr: u64, buffer_len: u64) -> u64 {
    let fn_len = filename_len as usize;
    let buf_len = buffer_len as usize;
    if fn_len > MAX_FS_FILENAME_LEN || buf_len > MAX_FS_DATA_LEN {
        return u64::MAX;
    }
    if fn_len == 0 {
        return u64::MAX;
    }
    let filename = unsafe { core::slice::from_raw_parts(filename_ptr as *const u8, fn_len) };
    let filename_str = core::str::from_utf8(filename).unwrap_or("");
    let result = crate::drivers::with_fs_driver(|fs| fs.read_file(filename_str));
    match result {
        Some(Some(data)) => {
            let copy_len = data.len().min(buf_len);
            let buf = unsafe { core::slice::from_raw_parts_mut(buffer_ptr as *mut u8, buf_len) };
            buf[..copy_len].copy_from_slice(&data[..copy_len]);
            copy_len as u64
        }
        _ => u64::MAX,
    }
}

/// a=filename_ptr, b=filename_len, c=data_ptr, d=data_len. Returns bytes written or u64::MAX.
fn sys_fs_write(filename_ptr: u64, filename_len: u64, data_ptr: u64, data_len: u64) -> u64 {
    let fn_len = filename_len as usize;
    let data_len_usize = data_len as usize;
    if fn_len > MAX_FS_FILENAME_LEN || data_len_usize > MAX_FS_DATA_LEN {
        return u64::MAX;
    }
    if fn_len == 0 {
        return u64::MAX;
    }
    let filename = unsafe { core::slice::from_raw_parts(filename_ptr as *const u8, fn_len) };
    let filename_str = core::str::from_utf8(filename).unwrap_or("");
    let data = unsafe { core::slice::from_raw_parts(data_ptr as *const u8, data_len_usize) };
    let result = crate::drivers::with_fs_driver(|fs| fs.write_file(filename_str, data));
    result.and_then(|r| r.ok()).map(|n| n as u64).unwrap_or(u64::MAX)
}

/// Returns number of files in ramdisk.
fn sys_fs_list() -> u64 {
    let result = crate::drivers::with_fs_driver(|fs| fs.list_files().len());
    result.map(|n| n as u64).unwrap_or(u64::MAX)
}

/// a=data_ptr, b=data_len. Returns bytes sent or u64::MAX.
fn sys_net_send(data_ptr: u64, data_len: u64) -> u64 {
    let len = data_len as usize;
    if len > MAX_NET_PACKET_LEN {
        return u64::MAX;
    }
    if len == 0 {
        return 0;
    }
    let data = unsafe { core::slice::from_raw_parts(data_ptr as *const u8, len) };
    let result = crate::drivers::with_net_driver(|net| net.send_packet(data));
    result.and_then(|r| r.ok()).map(|n| n as u64).unwrap_or(u64::MAX)
}

/// a=buffer_ptr, b=buffer_len. Copies received packet into buffer, returns bytes copied or u64::MAX.
fn sys_net_recv(buffer_ptr: u64, buffer_len: u64) -> u64 {
    let len = buffer_len as usize;
    if len > MAX_NET_PACKET_LEN {
        return u64::MAX;
    }
    let result = crate::drivers::with_net_driver(|net| net.recv_packet());
    match result {
        Some(Some(packet)) => {
            let copy_len = packet.len().min(len);
            let buf = unsafe { core::slice::from_raw_parts_mut(buffer_ptr as *mut u8, len) };
            buf[..copy_len].copy_from_slice(&packet[..copy_len]);
            copy_len as u64
        }
        _ => u64::MAX,
    }
}
