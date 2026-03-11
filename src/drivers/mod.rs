//! TS-weighted driver model. Drivers register as nodes; init order = descending weight — kernel supremacy.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::Ordering;
use crate::fs::Ramdisk;
use crate::gui::{FrameBufferWriter, FramebufferInfo, Color as GuiColor};
use crate::net::NetStack;
use crate::ts::{self, TS_REGISTRY};
use crate::println;
use spin::Mutex;
use lazy_static::lazy_static;

/// TS RULE: drivers initialized by descending node weight — kernel supremacy.
pub trait Driver: Send + core::any::Any {
    fn name(&self) -> &'static str;
    fn node_id(&self) -> &'static str;
    fn weight(&self) -> f32;
    fn init(&mut self) -> Result<(), &'static str>;
    fn poll(&mut self) {}

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
}

// --- Fake drivers ---

pub struct VgaTextDriver;

impl VgaTextDriver {
    pub const fn new() -> Self {
        VgaTextDriver
    }
}

impl Driver for VgaTextDriver {
    fn name(&self) -> &'static str {
        "vga_text_driver"
    }
    fn node_id(&self) -> &'static str {
        "vga_driver"
    }
    fn weight(&self) -> f32 {
        0.8
    }
    fn init(&mut self) -> Result<(), &'static str> {
        println!("VGA driver initialized");
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

pub struct TimerStub {
    tick_count: u64,
}

impl TimerStub {
    pub const fn new() -> Self {
        TimerStub { tick_count: 0 }
    }
}

impl Driver for TimerStub {
    fn name(&self) -> &'static str {
        "timer_stub"
    }
    fn node_id(&self) -> &'static str {
        "timer_driver"
    }
    fn weight(&self) -> f32 {
        0.82
    }
    fn init(&mut self) -> Result<(), &'static str> {
        self.tick_count = 0;
        println!("timer_stub: initialized (stub tick={})", self.tick_count);
        Ok(())
    }
    fn poll(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

/// Simulated UART. write() gated by enforce_min_weight("uart write", 0.7).
pub struct SimUart {
    _initialized: bool,
}

impl SimUart {
    pub const fn new() -> Self {
        SimUart {
            _initialized: false,
        }
    }

    /// TS RULE: driver ops enforce weight — uart write requires min 0.7.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if ts::enforce_min_weight("uart write", 0.7).is_err() {
            return Err(());
        }
        for &b in bytes {
            crate::print!("{}", b as char);
        }
        Ok(())
    }
}

impl Driver for SimUart {
    fn name(&self) -> &'static str {
        "sim_uart"
    }
    fn node_id(&self) -> &'static str {
        "uart_driver"
    }
    fn weight(&self) -> f32 {
        0.75
    }
    fn init(&mut self) -> Result<(), &'static str> {
        println!("Sim UART ready");
        self._initialized = true;
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

/// GUI driver: framebuffer writer. node_id "gui_driver", weight 0.6. TS RULE: GUI ops prioritized by node weight.
pub struct GuiDriver {
    writer: Option<FrameBufferWriter>,
}

impl GuiDriver {
    /// If fb_info is None, init will log "No framebuffer provided" and skip.
    pub fn new(fb_info: Option<FramebufferInfo>) -> Self {
        GuiDriver {
            writer: fb_info.map(FrameBufferWriter::new),
        }
    }

    pub fn clear_screen(&mut self, color: GuiColor) -> Result<(), ()> {
        match &mut self.writer {
            Some(w) => w.clear_screen(color),
            None => Err(()),
        }
    }

    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: GuiColor) -> Result<(), ()> {
        match &mut self.writer {
            Some(writer) => writer.draw_rect(x, y, w, h, color),
            None => Err(()),
        }
    }

    pub fn draw_text(&mut self, x: u32, y: u32, s: &str, color: GuiColor) -> Result<(), ()> {
        match &mut self.writer {
            Some(w) => w.draw_text(x, y, s, color),
            None => Err(()),
        }
    }

    pub fn has_framebuffer(&self) -> bool {
        self.writer.is_some()
    }
}

impl Driver for GuiDriver {
    fn name(&self) -> &'static str {
        "gui_driver"
    }
    fn node_id(&self) -> &'static str {
        "gui_driver"
    }
    fn weight(&self) -> f32 {
        0.6
    }
    fn init(&mut self) -> Result<(), &'static str> {
        if self.writer.is_some() {
            println!("GUI framebuffer initialized");
        } else {
            println!("No framebuffer provided");
        }
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

/// Ramdisk FS driver. node_id "ramdisk_fs", weight 0.65. TS RULE: filesystem ops gated by node weight — kernel supremacy.
pub struct FsDriver {
    ramdisk: Ramdisk,
}

impl FsDriver {
    pub fn new() -> Self {
        FsDriver {
            ramdisk: Ramdisk::new(),
        }
    }

    pub fn write_file(&mut self, filename: &str, data: &[u8]) -> Result<usize, ()> {
        self.ramdisk.write_file(filename, data)
    }

    pub fn read_file(&mut self, filename: &str) -> Option<alloc::vec::Vec<u8>> {
        self.ramdisk.read_file(filename)
    }

    pub fn list_files(&mut self) -> alloc::vec::Vec<alloc::string::String> {
        self.ramdisk.list_files()
    }
}

impl Driver for FsDriver {
    fn name(&self) -> &'static str {
        "ramdisk_fs_driver"
    }
    fn node_id(&self) -> &'static str {
        "ramdisk_fs"
    }
    fn weight(&self) -> f32 {
        0.65
    }
    fn init(&mut self) -> Result<(), &'static str> {
        self.ramdisk.init();
        println!("Ramdisk FS initialized (2 MiB)");
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

/// Network stub driver. node_id "network_stack", weight 0.75. TS RULE: network ops gated by node weight — kernel supremacy.
pub struct NetDriver {
    stack: NetStack,
}

impl NetDriver {
    pub fn new() -> Self {
        NetDriver {
            stack: NetStack::new(),
        }
    }

    pub fn send_packet(&mut self, data: &[u8]) -> Result<usize, ()> {
        self.stack.send_packet(data)
    }

    pub fn recv_packet(&mut self) -> Option<alloc::vec::Vec<u8>> {
        self.stack.recv_packet()
    }
}

impl Driver for NetDriver {
    fn name(&self) -> &'static str {
        "network_driver"
    }
    fn node_id(&self) -> &'static str {
        "network_stack"
    }
    fn weight(&self) -> f32 {
        0.75
    }
    fn init(&mut self) -> Result<(), &'static str> {
        println!("Networking stub initialized (loopback, weight 0.75)");
        Ok(())
    }
    fn poll(&mut self) {
        self.stack.poll();
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

// --- Driver manager: register nodes, sort by weight, init in order ---

lazy_static! {
    static ref DRIVERS: Mutex<Vec<Box<dyn Driver>>> = Mutex::new(Vec::new());
}

/// Register each driver as a TS node (parent "kernel"), then sort by weight descending and init.
/// Panics if a driver has invalid weight (TS RULE).
pub fn register_and_init_drivers(mut drivers: Vec<Box<dyn Driver>>) {
    // TS RULE: drivers initialized by descending node weight — kernel supremacy.
    {
        let mut reg = TS_REGISTRY.lock();
        for d in drivers.iter() {
            let w = d.weight();
            assert!(w >= 0.0 && w < 1.0, "TS: driver weight must be in [0, 1), got {}", w);
            reg.register_node(d.node_id(), w, Some("kernel"), alloc::vec![]);
        }
    }
    drivers.sort_by(|a, b| {
        b.weight()
            .partial_cmp(&a.weight())
            .unwrap_or(Ordering::Equal)
    });
    for d in drivers.iter_mut() {
        println!(
            "TS driver init: initializing {} (weight {:.2})",
            d.name(),
            d.weight()
        );
        d.init().expect("driver init failed");
    }
    *DRIVERS.lock() = drivers;
}

/// Call f with mutable reference to SimUart if present. For demo: uart_driver_write(bytes).
pub fn with_uart_driver<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut SimUart) -> R,
{
    let mut guard = DRIVERS.lock();
    for d in guard.iter_mut() {
        if let Some(u) = d.as_any_mut().downcast_mut::<SimUart>() {
            return Some(f(u));
        }
    }
    None
}

/// Call f with mutable reference to GuiDriver if present. For demo: clear_screen, draw_rect, draw_text.
pub fn with_gui_driver<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut GuiDriver) -> R,
{
    let mut guard = DRIVERS.lock();
    for d in guard.iter_mut() {
        if let Some(g) = d.as_any_mut().downcast_mut::<GuiDriver>() {
            return Some(f(g));
        }
    }
    None
}

/// Call f with mutable reference to FsDriver if present. For demo: write_file, read_file, list_files.
pub fn with_fs_driver<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut FsDriver) -> R,
{
    let mut guard = DRIVERS.lock();
    for d in guard.iter_mut() {
        if let Some(fs) = d.as_any_mut().downcast_mut::<FsDriver>() {
            return Some(f(fs));
        }
    }
    None
}

/// Call f with mutable reference to NetDriver if present. For demo: send_packet, recv_packet.
pub fn with_net_driver<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut NetDriver) -> R,
{
    let mut guard = DRIVERS.lock();
    for d in guard.iter_mut() {
        if let Some(net) = d.as_any_mut().downcast_mut::<NetDriver>() {
            return Some(f(net));
        }
    }
    None
}
