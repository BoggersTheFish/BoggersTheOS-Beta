#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use core::panic::PanicInfo;

pub mod allocator;
pub mod boot_display;
pub mod drivers;
pub mod elf_loader;
pub mod fs;
pub mod gdt;
pub mod gui;
pub mod interrupts;
pub mod memory;
pub mod net;
pub mod serial;
pub mod syscall;
pub mod task;
pub mod ts;
pub mod uptime;
pub mod vga_buffer;

/// Init subset safe for `cargo test` QEMU runs of other targets (e.g. `src/main.rs` unit tests).
/// Keeps interrupts disabled and skips TS init to avoid allocator/IRQ-related hangs during tests.
pub fn init_for_tests() {
    gdt::init();
    interrupts::init_idt();
}

pub fn init() {
    // #region agent log
    #[cfg(test)]
    serial_println!("[DBG c63425] init: start");
    // #endregion
    gdt::init();
    // #region agent log
    #[cfg(test)]
    serial_println!("[DBG c63425] init: gdt::init ok");
    // #endregion
    interrupts::init_idt();
    // #region agent log
    #[cfg(test)]
    serial_println!("[DBG c63425] init: interrupts::init_idt ok");
    // #endregion
    // TS RULE: kernel is alpha — register kernel node at 1.0 on boot.
    #[cfg(not(test))]
    {
        ts::init();
    }
    // #region agent log
    #[cfg(test)]
    serial_println!("[DBG c63425] init: ts::init skipped (test)");
    // #endregion

    // In test builds we keep interrupts disabled to avoid immediately-entered IRQ handlers
    // interfering with deterministic test execution.
    #[cfg(not(test))]
    {
        unsafe { interrupts::PICS.lock().initialize() };
        x86_64::instructions::interrupts::enable();
    }

    // #region agent log
    #[cfg(test)]
    serial_println!("[DBG c63425] init: end (interrupts left disabled for tests)");
    // #endregion
}
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        // #region agent log
        #[cfg(test)]
        serial_println!("[DBG c63425] exit_qemu write code={:?}", exit_code);
        // #endregion
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
use bootloader::{BootInfo, entry_point};

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test` (QEMU). Run tests then exit so CI doesn't time out.
/// Memory and heap must be set up before init(), since init() -> ts::init() uses the allocator.
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    use memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    // #region agent log
    serial_println!("[DBG c63425] lib::test_kernel_main entered");
    // #endregion

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // #region agent log
    serial_println!("[DBG c63425] lib::phys_mem_offset ok");
    // #endregion
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // #region agent log
    serial_println!("[DBG c63425] lib::memory::init ok");
    // #endregion
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    // #region agent log
    serial_println!("[DBG c63425] lib::frame_allocator ok");
    // #endregion
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    // #region agent log
    serial_println!("[DBG c63425] lib::heap init ok; calling init()");
    // #endregion

    init();
    // #region agent log
    serial_println!("[DBG c63425] lib::init done; calling test_main()");
    // #endregion
    test_main();
    // #region agent log
    serial_println!("[DBG c63425] lib::test_main returned; exiting qemu");
    // #endregion
    exit_qemu(QemuExitCode::Success);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
