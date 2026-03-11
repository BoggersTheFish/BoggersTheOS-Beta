# BoggersTheOS-Beta – TS-Driven Strongest-Node OS

**Motto:** *im the alpha im the leader im the one to trust*

A minimal x86_64 OS in Rust (fork of [phil-opp/blog_os](https://github.com/phil-opp/blog_os) async-await / post-12). Every subsystem is a **node** with a weight; the **kernel is always the strongest node** at fixed weight 1.0. All conflicts are resolved by weight only — no bypasses.

## Philosophy

- **Kernel** = fixed weight **1.0**. Nothing can equal or override it.
- Every major component (interrupts, memory, drivers, tasks) is a **node** with weight in `[0.0, 1.0]`.
- Nodes self-organize in a hierarchy (parent/children); **conflicts** (scheduling, resource allocation, syscalls, interrupts) are resolved **purely by comparing node weights**: higher wins, kernel always wins ties.
- **No fallback or override logic** that ignores weights — enforced with checks and panics.

## Features (example weights)

| Node               | Weight | Role                          |
|--------------------|--------|-------------------------------|
| kernel             | 1.0    | Alpha; all ties go to kernel  |
| interrupt_manager  | 0.95   | Interrupt handling            |
| memory_manager     | 0.9    | Memory subsystem              |
| task_executor      | 0.85   | Async task scheduling         |
| vga_driver         | 0.8    | VGA text                      |
| timer_driver       | 0.82   | Timer stub                    |
| network_stack      | 0.75   | Loopback net stub             |
| uart_driver        | 0.75   | Sim UART                      |
| ramdisk_fs         | 0.65   | 2 MiB in-memory FS            |
| gui_driver         | 0.6    | 320×200 framebuffer           |
| user_tasks         | 0.5    | Default for user-level tasks  |

- **TS-weighted scheduler**: tasks ordered by node weight (higher polled first); logs e.g. `TS schedule: picking task 'X' from node 'Y' (weight Z)`.
- **Syscalls** (int 0x80): write, exit, yield, get_node_weight, debug_hierarchy_dump, fs read/write/list, net send/recv — each gated by min weight.
- **Drivers**: VGA, timer, UART, network (loopback), ramdisk FS, GUI; init order = descending weight.
- **Demos**: FS write/read, net loopback send/recv, GUI rects + status text (uptime, file count); violation demos show denied ops for low-weight tasks.

## Build

Requires **nightly Rust** and (for bootable image) **bootimage**.

```bash
# If toolchain is missing rust-src:
rustup component add rust-src --toolchain nightly-x86_64-pc-windows-msvc

cargo build
cargo bootimage
```

## Run (QEMU)

With **graphics** (for GUI framebuffer):

```bash
cargo run
```

Or run the built image in QEMU with a display. Use `-display none` only for headless/testing.

## Example output

- **Hierarchy dump** (once at end of boot):

```
=== TS Hierarchy (kernel = alpha, weight 1.0) ===
[kernel] (weight: 1.0 (kernel))
  |- [interrupt_manager] (weight: 0.95)
  |- [memory_manager] (weight: 0.90)
  ...
  |- [gui_driver] (weight: 0.60)
  |- [user_tasks] (weight: 0.50)
=== end TS Hierarchy ===
BoggersTheOS-Beta booted – kernel is alpha leader (weight 1.0)
```

- **Scheduling**: `TS schedule: picking task 'N' from node 'task_executor' (weight 0.85)` (higher-weight tasks first).
- **Violations**: `TS violation: fs write denied - node 'user_tasks' weight 0.50 < 0.60` (and similar for net send, heap alloc, etc.).
- **GUI**: 320×200 with colored rects, “BoggersTheOS - kernel is alpha leader”, “TS Hierarchy Active – kernel weight 1.0”, “Uptime: X ticks”, “Files in ramdisk: Y”.

## Testing

```bash
cargo xtest
```

## License

Dual-licensed under **MIT** or **Apache-2.0** (see [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).

## Future

- Full smoltcp TCP/UDP stack
- Virtio / real NIC drivers
- Preemptive scheduling (still TS-weighted)
- Real userspace (ring 3) with syscall ABI

---

*Kernel supremacy. Weight only. No fallbacks.*
