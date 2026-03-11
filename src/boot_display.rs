//! Boot-time display/framebuffer info. Phase 1.1: centralize framebuffer probing for future UEFI/multiboot2.
//! TS RULE: display is a kernel resource; access gated by gui_driver weight — kernel supremacy preserved.

use bootloader::BootInfo;
use x86_64::VirtAddr;

use crate::gui::FramebufferInfo;

/// VGA mode 13h (320×200, 256 colors): physical base when bootloader uses `vga_320x200` feature.
const VGA_FB_PHYS: u64 = 0xa0000;
const VGA_MODE13_WIDTH: u32 = 320;
const VGA_MODE13_HEIGHT: u32 = 200;
const VGA_MODE13_BPP: u32 = 1;
const VGA_MODE13_STRIDE: u32 = 320;

/// Returns framebuffer info for the kernel. With bootloader 0.9 + vga_320x200, uses VGA 0xa0000.
/// Future: parse BootInfo.framebuffer (when present) or multiboot2/UEFI GOP for real resolution.
pub fn framebuffer_info(
    boot_info: &BootInfo,
    phys_mem_offset: VirtAddr,
) -> Option<FramebufferInfo> {
    let _ = boot_info; // reserved for future BootInfo.framebuffer or UEFI handoff
    let base_vaddr = phys_mem_offset + VGA_FB_PHYS;
    Some(FramebufferInfo {
        base: base_vaddr.as_u64() as *mut u8,
        width: VGA_MODE13_WIDTH,
        height: VGA_MODE13_HEIGHT,
        bytes_per_pixel: VGA_MODE13_BPP,
        stride: VGA_MODE13_STRIDE,
    })
}
