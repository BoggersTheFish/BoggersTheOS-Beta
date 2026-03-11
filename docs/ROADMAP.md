# BoggersTheOS-Beta – Full Roadmap to Minimal Usable OS

**Motto:** *im the alpha im the leader im the one to trust.*

**Constraint:** All work MUST preserve TS philosophy: kernel fixed weight 1.0, weights [0, 1), conflict resolution by weight only, no bypasses/fallbacks. Every new subsystem gets a node and weight; every privileged op is gated by `enforce_min_weight`.

---

## Current State (Phase 9 + Horizon 1 Stubs Complete)

- **Boot:** bootloader crate (BIOS/MBR), physical memory map, heap; **1.1** `boot_display::framebuffer_info()` centralizes VGA 320×200 (UEFI/multiboot2-ready).
- **TS:** TsRegistry, hierarchy_dump (single at boot end), weighted scheduler, syscalls min_weight, drivers weight-desc init; **1.2** timer-driven preemption points (reschedule by weight every N ticks); **1.3** GDT user segments, TSS RSP0, int 0x80 DPL 3; **1.4** `elf_loader::parse_elf64()` stub; **1.5** shell task (dump | run \<name\> | exit), node "shell" 0.6.
- **No:** full UEFI boot, per-process user page tables, actual ELF load to ring 3, persistent storage, real NIC.

---

# Horizon 1: Minimal Usable Kernel (3–12 months solo)

**Goal:** Boot on real x86_64 hardware, preemptive TS-weighted scheduler, ring-3 userspace with ELF loader, simple console/shell. Still in-memory FS and loopback net for demos.

---

## Phase 1.1 – UEFI Boot + Framebuffer Probing

| Item | Detail |
|------|--------|
| **Goal** | Replace BIOS bootloader with UEFI (uefi-rs / uefi-utils) or keep bootloader with VESA/multiboot2; get real framebuffer (resolution, pitch) from firmware for real HW. |
| **Key technical work** | (A) Add UEFI stub or multiboot2 support to existing boot flow; (B) Parse framebuffer tag (width, height, bpp, stride, physical base); (C) Pass `FramebufferInfo` to kernel via boot_info or handoff; (D) GUI driver uses probed values instead of hardcoded 320×200. |
| **Effort** | 4–8 weeks |
| **Dependencies** | `uefi` crate (no_std), or multiboot2 in bootloader; UEFI spec; QEMU -kernel + real HW test. |
| **TS integration** | No change to weights; kernel still 1.0, gui_driver 0.6, draw min 0.55. Add comment: `// TS RULE: framebuffer access gated by driver weight — kernel supremacy preserved`. |
| **Milestone demo** | QEMU: same GUI + hierarchy; real HW (or QEMU UEFI): kernel gets correct resolution and draws "BoggersTheOS Beta" + TS status. |

---

## Phase 1.2 – Preemptive Multitasking (TS-Weighted Slices)

| Item | Detail |
|------|--------|
| **Goal** | Timer interrupt triggers context switch; time slices proportional to node weight (higher weight → larger quantum); cooperative yield still available. |
| **Key technical work** | (A) Per-task kernel stacks + save/restore state in timer handler; (B) Scheduler state (current task index, ready list); (C) Quantum = base × weight (e.g. 10ms × weight); (D) On tick: decrement quantum or yield; (E) TS: pick next task by desc weight (same as current executor). |
| **Effort** | 6–10 weeks |
| **Dependencies** | Existing timer_stub → real PIT/APIC; task/executor refactor; optional `x86_64` TSS for kernel stack per task. |
| **TS integration** | Scheduler still sorts by node weight; log "TS schedule: preempt, picking task X from node Y (weight Z)". No task may exceed kernel; panic if weight ≥ 1.0. |
| **Milestone demo** | Multiple tasks run; TS schedule logs show preemption and weight-ordered scheduling; uptime still increments. |

---

## Phase 1.3 – Ring-3 Userspace (GDT/TSS + Privilege Drop)

| Item | Detail |
|------|--------|
| **Goal** | Run one or more user processes in ring 3; syscalls via int 0x80 (already present); kernel enforces TS weights for calling task. |
| **Key technical work** | (A) GDT: kernel code/data, user code/data, TSS; (B) TSS.esp0/ss0 for kernel stack on syscall; (C) Enter user: iret with user CS/RIP/RFLAGS/SS/RSP; (D) Syscall path: int 0x80 → kernel handler, look up current process’s node_id, enforce_min_weight per syscall, then dispatch. |
| **Effort** | 4–8 weeks |
| **Dependencies** | x86_64 GDT/TSS; per-process (or per-task) node_id for syscall gating. |
| **TS integration** | User processes have a node (e.g. "user_tasks" 0.5 or per-process node). Syscall handler uses that node for enforce_min_weight; violations return u64::MAX. No bypass. |
| **Milestone demo** | One hand-crafted user binary (or inline asm) in ring 3 that issues SYS_WRITE; kernel logs and output appear; violation (e.g. SYS_DEBUG_HIERARCHY_DUMP) denied. |

---

## Phase 1.4 – ELF Loader Stub (User Binaries)

| Item | Detail |
|------|--------|
| **Goal** | Load and run position-independent or fixed-address ELF executables (statically linked, no dynamic linker) from ramdisk or later from FS. |
| **Key technical work** | (A) Parse ELF header + program headers; (B) Allocate user pages (or use pre-mapped region), copy segments; (C) Set entry point; (D) Create "process" with its own node_id (e.g. inherit "user_tasks" or config); (E) Optional: syscall SYS_EXEC or kernel API to load and run. |
| **Effort** | 4–6 weeks |
| **Dependencies** | `elf` or minimal ELF parsing (no_std); page allocator for user space; ramdisk or FS read. |
| **TS integration** | Loader runs in kernel context (weight 1.0); loaded process gets a node (e.g. 0.5); all syscalls from that process gated by that node. |
| **Milestone demo** | Kernel loads one ELF from ramdisk, jumps to ring 3; program runs and can syscall (write, exit). |

---

## Phase 1.5 – Simple Console / Shell

| Item | Detail |
|------|--------|
| **Goal** | Line-oriented console: read keyboard (or UART), echo, parse simple commands (e.g. run ELF by name, hierarchy_dump, exit), run as ring-3 or kernel task. |
| **Key technical work** | (A) Keyboard → line buffer; (B) Parser: "run <name>", "dump", "exit"; (C) "run": look up ELF in ramdisk/FS, load via 1.4, run in ring 3; (D) Shell task has node_id (e.g. "shell" 0.6) so it can call hierarchy_dump (min 0.6). |
| **Effort** | 3–6 weeks |
| **Dependencies** | Existing keyboard + VGA/UART; ELF loader; ramdisk/FS list. |
| **TS integration** | Shell runs at weight high enough for allowed commands; user-typed "run" programs inherit user_tasks 0.5 unless elevated. enforce_min_weight on every syscall. |
| **Milestone demo** | Boot → "BoggersTheOS-Beta" banner → prompt; type "run hello" → user program runs; type "dump" → hierarchy dump; violations still denied. |

---

## Horizon 1 Summary

| Phase | Goal | Effort (est.) |
|-------|------|----------------|
| 1.1 | UEFI / better framebuffer | 4–8 wks |
| 1.2 | Preemptive TS-weighted scheduler | 6–10 wks |
| 1.3 | Ring-3 + syscall path | 4–8 wks |
| 1.4 | ELF loader stub | 4–6 wks |
| 1.5 | Console/shell | 3–6 wks |

**Total Horizon 1:** ~6–12 months solo, depending on depth and real HW testing.

---

# Horizon 2: Persistent Storage & Real Networking (1–3 years part-time)

**Goal:** Real block device (virtio-blk), on-disk FS (ext2 or FAT32), smoltcp + virtio-net, basic PCI/USB HID, minimal init flow. TS weights on all drivers and syscalls unchanged.

---

## Phase 2.1 – Virtio-Blk + Block Cache

| Item | Detail |
|------|--------|
| **Goal** | virtio-blk driver; read/write blocks; optional in-kernel block cache (weight-gated); ramdisk remains for early boot, then root from virtio. |
| **Key technical work** | (A) PCI discovery, virtio MMIO or PCI; (B) Virtio queue setup, descriptors, used/avail; (C) Block read/write; (D) Cache layer with TS gate (e.g. cache node 0.78); (E) Driver node e.g. 0.77. |
| **Effort** | 2–4 months |
| **Dependencies** | virtio spec; `volatile`; PCI config space (optional crate). |
| **TS integration** | Block driver node < kernel; FS driver (or new "block_cache" node) gates access; no raw block from user_tasks. |
| **Milestone demo** | QEMU with -drive if=virtio; kernel reads/writes blocks; ramdisk + virtio coexist; hierarchy shows new node. |

---

## Phase 2.2 – Real FS (ext2 or FAT32)

| Item | Detail |
|------|--------|
| **Goal** | Mount ext2 or FAT32 from virtio-blk; replace or complement ramdisk for root; same FS syscalls (read/write/list) with min_weight. |
| **Key technical work** | (A) ext2: superblock, block groups, inodes, directory entries; (B) Or FAT32: BPB, FAT, clusters; (C) VFS-like layer or direct FS driver; (D) Path resolution, open/read/write/close semantics. |
| **Effort** | 3–6 months |
| **Dependencies** | FS spec; existing block layer; allocator for buffers. |
| **TS integration** | FS driver node (e.g. 0.65); SYS_FS_READ/WRITE/LIST unchanged; enforce_min_weight per op; no root bypass. |
| **Milestone demo** | Boot from virtio disk with ext2/FAT image; shell "run" loads ELF from disk; files persist across reboot. |

---

## Phase 2.3 – Smoltcp + Virtio-Net

| Item | Detail |
|------|--------|
| **Goal** | Replace loopback VecDeque with smoltcp Interface + virtio-net device; UDP/TCP; same NetDriver API and TS gates (send 0.7, recv 0.65). |
| **Key technical work** | (A) Virtio-net queues, RX/TX; (B) smoltcp Device trait impl over virtio-net; (C) Interface + SocketSet, poll in driver; (D) Optional: loopback fallback for no-NIC (still weight-gated). |
| **Effort** | 2–4 months |
| **Dependencies** | smoltcp (already in tree); virtio-net spec. |
| **TS integration** | Net driver 0.75; send/recv min weights unchanged; no unprivileged send. |
| **Milestone demo** | QEMU -device virtio-net; kernel sends/receives real packets; optional UDP echo or DHCP. |

---

## Phase 2.4 – PCI & Device Enumeration

| Item | Detail |
|------|--------|
| **Goal** | Enumerate PCI buses, attach virtio-blk/virtio-net by vendor/device ID; optional ACPI for future. |
| **Key technical work** | (A) PCI config space read (0xCF8/0xCFC or MMIO); (B) Bus walk, BARs; (C) Register "pci" or "bus" node; drivers claim devices by ID. |
| **Effort** | 1–2 months |
| **Dependencies** | PCI spec; existing drivers. |
| **TS integration** | PCI discovery at high weight (e.g. 0.88); driver init order still by weight. |
| **Milestone demo** | Log "virtio-blk at ...", "virtio-net at ..."; drivers bind and work. |

---

## Phase 2.5 – USB HID (Keyboard) Stub

| Item | Detail |
|------|--------|
| **Goal** | Basic USB host (xHCI or UHCI) + HID keyboard for real HW where PS/2 is absent. |
| **Key technical work** | (A) xHCI or UHCI init, port reset; (B) HID descriptor parse, interrupt in endpoint; (C) Key events feed into existing keyboard task. |
| **Effort** | 2–4 months |
| **Dependencies** | USB spec; xHCI doc. |
| **TS integration** | USB driver node (e.g. 0.74); input events gated for shell/tasks. |
| **Milestone demo** | Real laptop/desktop: keyboard input works without PS/2. |

---

## Phase 2.6 – Init System (First Userspace)

| Item | Detail |
|------|--------|
| **Goal** | Kernel runs one "init" ELF (e.g. shell or minimal init); init can fork/exec (if implemented) or just run shell; clean shutdown/reboot syscall. |
| **Key technical work** | (A) Load init from disk (or ramdisk); (B) SYS_REBOOT / SYS_SHUTDOWN; (C) Optional: SYS_FORK, SYS_WAIT for simple process tree. |
| **Effort** | 1–2 months |
| **Dependencies** | ELF loader; FS; shell. |
| **TS integration** | Init runs as designated node (e.g. 0.55); child processes inherit or get user_tasks 0.5. |
| **Milestone demo** | Boot → init → shell; reboot from shell works. |

---

## Horizon 2 Summary

| Phase | Goal | Effort (est.) |
|-------|------|----------------|
| 2.1 | Virtio-blk + block cache | 2–4 mo |
| 2.2 | ext2 or FAT32 on disk | 3–6 mo |
| 2.3 | Smoltcp + virtio-net | 2–4 mo |
| 2.4 | PCI enumeration | 1–2 mo |
| 2.5 | USB HID keyboard | 2–4 mo |
| 2.6 | Init + reboot/shutdown | 1–2 mo |

**Total Horizon 2:** 1–3 years part-time.

---

# Horizon 3: General-Purpose (5+ years / team)

**Goal:** GUI compositor, package management, stronger security model, broad hardware support. TS remains: kernel 1.0, all resources weight-gated.

---

## Phase 3.1 – GUI Compositor (Windowing)

| Item | Detail |
|------|--------|
| **Goal** | Multiple "windows" (buffers), compositor task, input routing; optional GPU acceleration later. |
| **Effort** | 1–2 years |
| **TS integration** | Compositor node (e.g. 0.62); draw/flip gated; apps at lower weight. |

---

## Phase 3.2 – Package Manager & Base System

| Item | Detail |
|------|--------|
| **Goal** | Repo of ELF packages, install/upgrade from disk or net; dependency resolution; signed updates. |
| **Effort** | 1–2 years |
| **TS integration** | Package install/update requires elevated node; no user_tasks direct install. |

---

## Phase 3.3 – Security Model (Capabilities / MAC)

| Item | Detail |
|------|--------|
| **Goal** | Capability-like weight inheritance, optional MAC (e.g. per-process labels); no weakening of TS (kernel still 1.0, no bypass). |
| **Effort** | 1–3 years |
| **TS integration** | Weights and nodes remain; add optional labels/caps that map to allowed ops. |

---

## Phase 3.4 – Broad Hardware Support

| Item | Detail |
|------|--------|
| **Goal** | More GPUs, NICs, storage; ACPI full; power management; multi-core. |
| **Effort** | Ongoing |
| **TS integration** | Each driver family gets node; init order by weight. |

---

# Implementation Order (After Plan Approval)

- **Horizon 1 implemented (stubs):**
  - **1.1** `src/boot_display.rs`: `framebuffer_info(boot_info, phys_mem_offset)` → VGA 0xa0000; main uses it for GUI.
  - **1.2** `src/uptime.rs`: `PREEMPT_REQUESTED` every 20 ticks; executor re-queues and reschedules by weight.
  - **1.3** `src/gdt.rs`: user_code/user_data segments, TSS.privilege_stack_table[0]; `src/interrupts.rs`: int 0x80 DPL 3.
  - **1.4** `src/elf_loader.rs`: `parse_elf64()` → entry_point, phoff, phnum (no user mapping yet).
  - **1.5** `src/task/shell.rs`: shell_task(); "dump" (hierarchy_dump), "run \<name\>" (ELF parse from ramdisk), "exit"; node "shell" 0.6; spawn instead of keyboard.
- Each phase: `// TS RULE: ...` comments added; no_std + safe Rust preserved.
- Next: real iretq-to-user stub, ELF load into user pages, then Horizon 2.

---

*Kernel is alpha. Weight only. No fallbacks.*
