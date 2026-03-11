//! Minimal console shell. Phase 1.5: line buffer, dump/run/exit. TS RULE: shell node 0.6 — kernel supremacy.

use crate::{drivers, println, ts};
use alloc::vec::Vec;
use core::str;
use futures_util::StreamExt;
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};

use super::keyboard::ScancodeStream;

const MAX_LINE: usize = 128;

/// Run shell: read keys, on Enter parse "dump" | "run <name>" | "exit". Shell has weight 0.6 for dump.
pub async fn shell_task() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );
    let mut line: Vec<u8> = Vec::with_capacity(MAX_LINE);

    println!("BoggersTheOS-Beta shell (dump | run <name> | exit)");
    loop {
        let key = match scancodes.next().await {
            Some(sc) => {
                if let Ok(Some(ev)) = keyboard.add_byte(sc) {
                    keyboard.process_keyevent(ev)
                } else {
                    None
                }
            }
            None => continue,
        };

        let Some(decoded) = key else { continue };

        match decoded {
            DecodedKey::Unicode('\r') | DecodedKey::Unicode('\n') => {
                if line.is_empty() {
                    continue;
                }
                let s = str::from_utf8(&line).unwrap_or("");
                let s = s.trim();
                // TS RULE: shell (0.6) can call hierarchy_dump (min 0.6) — kernel supremacy.
                if s == "dump" {
                    ts::print_hierarchy_dump();
                } else if s == "exit" {
                    println!("shell: exit (halt not implemented)");
                } else if s.starts_with("run ") {
                    let name = s[4..].trim();
                    run_cmd(name).await;
                } else {
                    println!("unknown: {}", s);
                }
                line.clear();
            }
            DecodedKey::Unicode(c) => {
                if line.len() < MAX_LINE && c.is_ascii() && !c.is_control() {
                    line.push(c as u8);
                    crate::print!("{}", c);
                }
            }
            DecodedKey::RawKey(_) => {}
        }
    }
}

async fn run_cmd(name: &str) {
    let data = drivers::with_fs_driver(|fs| fs.read_file(name));
    match data {
        Some(Some(bytes)) => {
            if let Some(elf) = crate::elf_loader::parse_elf64(&bytes) {
                println!(
                    "run {}: ELF64 entry 0x{:x} (load to user space not yet implemented)",
                    name, elf.entry_point
                );
            } else {
                println!("run {}: not a valid ELF64", name);
            }
        }
        Some(None) => println!("run {}: file not found", name),
        None => println!("run: no fs driver"),
    }
}
