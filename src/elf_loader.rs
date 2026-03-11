//! Minimal ELF64 loader stub. Phase 1.4: parse header and entry point; full load (user pages) later.
//! TS RULE: loader runs in kernel context; loaded process gets node (e.g. user_tasks) — kernel supremacy.

#![allow(dead_code)]

/// ELF64 magic.
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;

/// Parsed ELF64 info (stub: entry point and segment count).
pub struct Elf64Info {
    pub entry_point: u64,
    pub phoff: u64,
    pub phnum: u16,
    pub is_valid: bool,
}

/// Parse ELF64 header from raw bytes. No alloc. Returns None if not a valid ELF64 executable.
pub fn parse_elf64(data: &[u8]) -> Option<Elf64Info> {
    // Minimal size: e_ident (16) + rest of Ehdr up to e_entry (24 bytes more) = 40 bytes
    if data.len() < 64 {
        return None;
    }
    if data[0..4] != ELF_MAGIC {
        return None;
    }
    if data[4] != ELFCLASS64 {
        return None;
    }
    if data[5] != ELFDATA2LSB {
        return None;
    }
    if data[6] != EV_CURRENT {
        return None;
    }
    // e_type at offset 16
    let e_type = u16::from_le_bytes([data[16], data[17]]);
    if e_type != ET_EXEC {
        return None;
    }
    let e_machine = u16::from_le_bytes([data[18], data[19]]);
    if e_machine != EM_X86_64 {
        return None;
    }
    // e_entry at offset 24 (u64)
    let entry_point = u64::from_le_bytes([
        data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
    ]);
    // e_phoff at 32, e_phnum at 56
    let phoff = u64::from_le_bytes([
        data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
    ]);
    let phnum = u16::from_le_bytes([data[56], data[57]]);

    Some(Elf64Info {
        entry_point,
        phoff,
        phnum,
        is_valid: true,
    })
}
