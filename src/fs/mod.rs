//! Minimal in-memory filesystem (ramdisk). TS RULE: filesystem ops gated by node weight — kernel supremacy.

use crate::ts;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

const RAMDISK_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
const MIN_WEIGHT_FS_WRITE: f32 = 0.6;
const MIN_WEIGHT_FS_READ: f32 = 0.5;
const MIN_WEIGHT_FS_LIST: f32 = 0.5;

/// Single file entry: start offset and length in the buffer.
#[derive(Clone, Copy)]
struct FileEntry {
    offset: usize,
    len: usize,
}

/// Ramdisk: fixed-size buffer + index of (filename -> offset, len).
pub struct Ramdisk {
    buffer: Vec<u8>,
    index: BTreeMap<String, FileEntry>,
    next_offset: usize,
}

impl Ramdisk {
    pub fn new() -> Self {
        Ramdisk {
            buffer: Vec::with_capacity(RAMDISK_SIZE),
            index: BTreeMap::new(),
            next_offset: 0,
        }
    }

    /// Must be called once after creation to zero-initialize the buffer.
    pub fn init(&mut self) {
        self.buffer.resize(RAMDISK_SIZE, 0);
        self.next_offset = 0;
        self.index.clear();
    }

    /// TS RULE: filesystem ops gated by node weight — kernel supremacy.
    pub fn write_file(&mut self, filename: &str, data: &[u8]) -> Result<usize, ()> {
        if ts::enforce_min_weight("fs write", MIN_WEIGHT_FS_WRITE).is_err() {
            return Err(());
        }
        let len = data.len();
        if self.next_offset + len > self.buffer.len() {
            return Err(());
        }
        let offset = self.next_offset;
        self.buffer[offset..offset + len].copy_from_slice(data);
        self.next_offset += len;
        self.index
            .insert(String::from(filename), FileEntry { offset, len });
        Ok(len)
    }

    /// TS RULE: filesystem ops gated by node weight.
    pub fn read_file(&mut self, filename: &str) -> Option<Vec<u8>> {
        if ts::enforce_min_weight("fs read", MIN_WEIGHT_FS_READ).is_err() {
            return None;
        }
        let entry = self.index.get(filename)?;
        Some(self.buffer[entry.offset..entry.offset + entry.len].to_vec())
    }

    /// TS RULE: filesystem ops gated by node weight.
    pub fn list_files(&self) -> Vec<String> {
        if ts::enforce_min_weight("fs list", MIN_WEIGHT_FS_LIST).is_err() {
            return Vec::new();
        }
        self.index.keys().cloned().collect()
    }
}
