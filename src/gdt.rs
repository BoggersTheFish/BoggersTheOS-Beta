//! GDT and TSS. Phase 1.3: user code/data segments and RSP0 for ring-3 syscall entry.
//! TS RULE: privilege levels enforce kernel supremacy; syscall gate remains weight-gated.

use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Kernel stack used when CPU switches from ring 3 to ring 0 (e.g. int 0x80). Phase 1.3.
const RING0_STACK_SIZE: usize = 4096 * 2;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE
        };
        // TS RULE: RSP0 used on syscall from ring 3 — kernel stack for handler; kernel supremacy preserved.
        tss.privilege_stack_table[0] = {
            static mut RING0_STACK: [u8; RING0_STACK_SIZE] = [0; RING0_STACK_SIZE];
            let start = VirtAddr::from_ptr(&raw const RING0_STACK);
            start + RING0_STACK_SIZE
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                user_code_selector,
                user_data_selector,
            },
        )
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
}

/// User code and data selectors for iretq to ring 3. Phase 1.3.
pub fn user_code_selector() -> SegmentSelector {
    GDT.1.user_code_selector
}

pub fn user_data_selector() -> SegmentSelector {
    GDT.1.user_data_selector
}

pub fn init() {
    use x86_64::instructions::segmentation::{CS, Segment};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
