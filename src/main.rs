#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

#[cfg(not(test))]
use alloc::vec::Vec;
use blog_os::println;
#[cfg(not(test))]
use blog_os::syscall::{self, SYS_DEBUG_HIERARCHY_DUMP, SYS_GET_NODE_WEIGHT, SYS_WRITE};
#[cfg(not(test))]
use blog_os::task::{Task, executor::Executor, shell};
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use blog_os::allocator;
    use blog_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    println!("Hello World{}", "!");
    // #region agent log
    #[cfg(test)]
    blog_os::serial_println!("[DBG c63425] bin::kernel_main entered (cfg(test))");
    // #endregion

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // #region agent log
    #[cfg(test)]
    blog_os::serial_println!("[DBG c63425] bin::phys_mem_offset ok");
    // #endregion
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // #region agent log
    #[cfg(test)]
    blog_os::serial_println!("[DBG c63425] bin::memory::init ok");
    // #endregion
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    // #region agent log
    #[cfg(test)]
    blog_os::serial_println!("[DBG c63425] bin::frame_allocator ok");
    // #endregion

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    // #region agent log
    #[cfg(test)]
    blog_os::serial_println!("[DBG c63425] bin::heap init ok; calling blog_os::init()");
    // #endregion
    #[cfg(test)]
    {
        blog_os::init_for_tests();
    }
    #[cfg(not(test))]
    {
        blog_os::init();
    }
    // #region agent log
    #[cfg(test)]
    blog_os::serial_println!("[DBG c63425] bin::blog_os::init done");
    // #endregion

    // TS: register initial subsystem nodes (kernel already at 1.0 from init())
    {
        use blog_os::ts::TS_REGISTRY;
        let mut reg = TS_REGISTRY.lock();
        reg.register_node("interrupt_manager", 0.95, Some("kernel"), alloc::vec![]);
        reg.register_node("memory_manager", 0.9, Some("kernel"), alloc::vec![]);
        reg.register_node("task_executor", 0.85, Some("kernel"), alloc::vec![]);
        reg.register_node("user_tasks", 0.5, Some("kernel"), alloc::vec![]);
        reg.register_node("shell", 0.6, Some("kernel"), alloc::vec![]);
    }

    #[cfg(not(test))]
    {
        // Phase 5+6 + 1.1: TS-weighted drivers; framebuffer from boot_display (VGA 0xa0000 now; UEFI/multiboot2 later).
        // TS RULE: display resource gated by gui_driver weight — kernel supremacy preserved.
        let fb_info = blog_os::boot_display::framebuffer_info(boot_info, phys_mem_offset);
        use blog_os::drivers::{
            FsDriver, GuiDriver, NetDriver, SimUart, TimerStub, VgaTextDriver,
            register_and_init_drivers,
        };
        let drivers: alloc::vec::Vec<alloc::boxed::Box<dyn blog_os::drivers::Driver>> = alloc::vec![
            alloc::boxed::Box::new(VgaTextDriver::new()),
            alloc::boxed::Box::new(TimerStub::new()),
            alloc::boxed::Box::new(SimUart::new()),
            alloc::boxed::Box::new(NetDriver::new()),
            alloc::boxed::Box::new(FsDriver::new()),
            alloc::boxed::Box::new(GuiDriver::new(fb_info)),
        ];
        register_and_init_drivers(drivers);
        // TS RULE: final polish maintains kernel supremacy — single hierarchy dump after all inits
        blog_os::ts::print_hierarchy_dump();
        println!("BoggersTheOS-Beta booted – kernel is alpha leader (weight 1.0)");
    }

    #[cfg(test)]
    {
        // #region agent log
        blog_os::serial_println!("[DBG c63425] bin::test_main about to run");
        // #endregion
        test_main();
        // #region agent log
        blog_os::serial_println!("[DBG c63425] bin::test_main returned; exiting qemu");
        // #endregion
        // Ensure QEMU exits so CI doesn't time out (test_runner also calls exit_qemu; this is fallback).
        blog_os::exit_qemu(blog_os::QemuExitCode::Success);
        blog_os::hlt_loop();
    }

    #[cfg(not(test))]
    {
        let mut executor = Executor::new();
        // Orchestrated order: high-weight demos first, then violation demos (weighted scheduling visible in TS logs)
        executor.spawn(Task::new_with_node(shell::shell_task(), "shell"));
        executor.spawn(Task::new_with_node(net_demo_task(), "task_executor"));
        executor.spawn(Task::new_with_node(fs_demo_task(), "gui_driver"));
        executor.spawn(Task::new_with_node(gui_demo_task(), "gui_driver"));
        executor.spawn(Task::new_with_node(
            driver_uart_demo_task(),
            "task_executor",
        ));
        executor.spawn(Task::new(example_task()));
        executor.spawn(Task::new(syscall_stub_task()));
        executor.spawn(Task::new(fs_user_violation_task()));
        executor.spawn(Task::new(net_violation_task()));
        executor.run();
    }
}

/// This function is called on panic. TS RULE: show context (node/weight) for debugging.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let node =
        blog_os::ts::current_node_id().unwrap_or_else(|| alloc::string::String::from("kernel"));
    let w = blog_os::ts::current_node_weight();
    println!("[panic] node={} weight={:.2}", node, w);
    println!("{}", info);
    blog_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[cfg(not(test))]
async fn async_number() -> u32 {
    42
}

#[cfg(not(test))]
async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
    // Demo: low-weight task (user_tasks 0.5) attempts large alloc (>256 bytes needs weight 0.7) -> TS violation
    let _v: Vec<u8> = Vec::with_capacity(1024);
    println!("large alloc ok");
}

/// Minimal "userspace" stub: invokes syscalls via dispatch (same path as int 0x80). Demo TS enforcement.
#[cfg(not(test))]
async fn syscall_stub_task() {
    // SYS_GET_NODE_WEIGHT (min 0.3) — user_tasks 0.5 allowed
    let w = syscall::dispatch(SYS_GET_NODE_WEIGHT, 0, 0, 0, 0);
    if w != u64::MAX {
        let f = f32::from_bits(w as u32);
        println!("[stub] sys_get_node_weight -> {:.2}", f);
    }
    // SYS_WRITE (min 0.3) — allowed
    let msg = b"hello from syscall stub\n";
    syscall::dispatch(SYS_WRITE, msg.as_ptr() as u64, msg.len() as u64, 0, 0);
    // SYS_DEBUG_HIERARCHY_DUMP (min 0.6) — user_tasks 0.5 < 0.6 -> TS violation
    println!("[stub] attempting sys_debug_hierarchy_dump (min weight 0.6)...");
    let ret = syscall::dispatch(SYS_DEBUG_HIERARCHY_DUMP, 0, 0, 0, 0);
    if ret == u64::MAX {
        println!("[stub] sys_debug_hierarchy_dump denied (TS violation as expected)");
    }
}

/// Phase 5 demo: task_executor (weight 0.85) calls uart write — allowed (min 0.7).
#[cfg(not(test))]
async fn driver_uart_demo_task() {
    match blog_os::drivers::with_uart_driver(|uart| {
        uart.write(b"[uart] hello from task_executor (weight 0.85)\n")
    }) {
        Some(Ok(())) => blog_os::println!("[driver_demo] uart write succeeded (TS allowed)"),
        Some(Err(())) => blog_os::println!("[driver_demo] uart write denied (TS violation)"),
        None => blog_os::println!("[driver_demo] no uart driver"),
    }
}

/// Phase 6+9: gui_driver (0.6) draws framebuffer + persistent status (uptime, ramdisk file count).
#[cfg(not(test))]
async fn gui_demo_task() {
    use blog_os::gui::{
        COLOR_BLACK, COLOR_BLUE, COLOR_GREEN, COLOR_RED, COLOR_WHITE, COLOR_YELLOW,
    };
    let uptime_ticks = blog_os::uptime::ticks();
    let file_count = blog_os::drivers::with_fs_driver(|fs| fs.list_files().len()).unwrap_or(0);
    let result = blog_os::drivers::with_gui_driver(|gui| {
        if !gui.has_framebuffer() {
            return None;
        }
        gui.clear_screen(COLOR_BLACK).ok()?;
        gui.draw_rect(10, 10, 80, 40, COLOR_BLUE).ok()?;
        gui.draw_rect(100, 10, 80, 40, COLOR_GREEN).ok()?;
        gui.draw_rect(190, 10, 80, 40, COLOR_RED).ok()?;
        gui.draw_rect(10, 60, 300, 30, COLOR_YELLOW).ok()?;
        gui.draw_text(20, 70, "BoggersTheOS - kernel is alpha leader", COLOR_BLACK)
            .ok()?;
        gui.draw_text(20, 90, "BoggersTheOS Beta", COLOR_WHITE)
            .ok()?;
        gui.draw_text(
            20,
            100,
            "TS Hierarchy Active – kernel weight 1.0",
            COLOR_WHITE,
        )
        .ok()?;
        let uptime_str = alloc::format!("Uptime: {} ticks", uptime_ticks);
        gui.draw_text(20, 110, &uptime_str, COLOR_WHITE).ok()?;
        let files_str = alloc::format!("Files in ramdisk: {}", file_count);
        gui.draw_text(20, 120, &files_str, COLOR_WHITE).ok()?;
        Some(())
    });
    let node =
        blog_os::ts::current_node_id().unwrap_or_else(|| alloc::string::String::from("kernel"));
    match result {
        Some(Some(())) => {
            blog_os::println!("[{}] gui framebuffer + status drawn (TS allowed)", node)
        }
        Some(None) => blog_os::println!("[{}] gui draw failed or no framebuffer", node),
        None => blog_os::println!("[{}] no gui driver", node),
    }
}

/// Phase 7 demo: gui_driver (0.6) — fs write/read/list allowed (write min 0.6, read/list min 0.5).
#[cfg(not(test))]
async fn fs_demo_task() {
    let node =
        blog_os::ts::current_node_id().unwrap_or_else(|| alloc::string::String::from("kernel"));
    let msg = b"TS is alpha supremacy";
    let wrote = blog_os::drivers::with_fs_driver(|fs| fs.write_file("hello.txt", msg));
    match wrote {
        Some(Ok(n)) => blog_os::println!("[{}] fs_demo wrote hello.txt ({} bytes)", node, n),
        Some(Err(())) => blog_os::println!("[{}] fs_demo write denied (TS violation)", node),
        None => blog_os::println!("[{}] fs_demo no fs driver", node),
    }
    let read = blog_os::drivers::with_fs_driver(|fs| fs.read_file("hello.txt"));
    if let Some(Some(data)) = read {
        let s = core::str::from_utf8(&data).unwrap_or("");
        blog_os::println!("[{}] fs_demo read back: {}", node, s);
    }
    let list = blog_os::drivers::with_fs_driver(|fs| fs.list_files());
    if let Some(files) = list {
        blog_os::println!("[{}] fs_demo list_files: {:?}", node, files);
    }
}

/// Phase 7: user_tasks (0.5) attempts fs write — min 0.6 required → TS violation.
#[cfg(not(test))]
async fn fs_user_violation_task() {
    blog_os::println!("[fs_violation] user_tasks (0.5) attempting fs write (min 0.6)...");
    let wrote = blog_os::drivers::with_fs_driver(|fs| fs.write_file("denied.txt", b"x"));
    if let Some(Err(())) = wrote {
        blog_os::println!("[fs_violation] fs write denied as expected (TS violation)");
    }
}

/// Phase 8 demo: task_executor (0.85) — net send (min 0.7) and recv (min 0.65) allowed; loopback.
#[cfg(not(test))]
async fn net_demo_task() {
    let node =
        blog_os::ts::current_node_id().unwrap_or_else(|| alloc::string::String::from("kernel"));
    let payload = b"TS alpha packet supremacy";
    let sent = blog_os::drivers::with_net_driver(|net| net.send_packet(payload));
    if let Some(Ok(n)) = sent {
        blog_os::println!("[{}] net_demo sent {} bytes (loopback)", node, n);
    }
    let recv = blog_os::drivers::with_net_driver(|net| net.recv_packet());
    if let Some(Some(packet)) = recv {
        let s = core::str::from_utf8(&packet).unwrap_or("");
        blog_os::println!("[{}] net_demo received: {}", node, s);
        blog_os::println!("[{}] sent & received loopback packet", node);
    } else {
        blog_os::println!("[{}] net_demo recv failed or no driver", node);
    }
}

/// Phase 8: user_tasks (0.5) attempts net send — min 0.7 required → TS violation.
#[cfg(not(test))]
async fn net_violation_task() {
    blog_os::println!("[net_violation] user_tasks (0.5) attempting net send (min 0.7)...");
    let sent = blog_os::drivers::with_net_driver(|net| net.send_packet(b"denied"));
    if let Some(Err(())) = sent {
        blog_os::println!("[net_violation] net send denied as expected (TS violation)");
    }
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
