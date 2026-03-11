# BoggersTheOS-Beta – Master Plan (All 9 Phases)

Fork of phil-opp/blog_os (post-12 async-await). TS-driven strongest-node hierarchy throughout.

## Phases completed

| Phase | Name                         | Summary |
|-------|------------------------------|--------|
| 1     | TS Foundation                | `ts.rs`: TsNode, TsRegistry, register_node, get_weight, resolve_conflict; kernel 1.0; initial nodes; hierarchy_dump() |
| 2     | TS-Weighted Async Scheduler  | Task.node_id; executor sorts by weight; TS schedule logs; current_task tracking |
| 3     | TS Security & Resource       | current_node_weight, enforce_min_weight; heap alloc (0.5/0.7); interrupt handlers set node; violation demos |
| 4     | Basic Syscalls               | int 0x80; SYS_WRITE, EXIT, YIELD, GET_NODE_WEIGHT, DEBUG_HIERARCHY_DUMP; min weights; syscall_stub_task |
| 5     | Driver Model & Fake Devices  | Driver trait; VgaTextDriver, TimerStub, SimUart; register_and_init_drivers (weight order); with_*_driver helpers |
| 6     | Simple GUI / Framebuffer      | gui.rs FrameBufferWriter; GuiDriver 0.6; draw_rect, draw_text, clear_screen (min 0.55); 320×200 VGA |
| 7     | Minimal FS & Storage         | fs/mod.rs Ramdisk 2 MiB; FsDriver 0.65; SYS_FS_READ/WRITE/LIST; fs_demo_task; fs_user_violation_task |
| 8     | Networking Stub              | net/mod.rs loopback VecDeque; NetDriver 0.75; SYS_NET_SEND/RECV; net_demo_task; net_violation_task |
| 9     | Polish & Documentation       | Single hierarchy dump; boot log; uptime ticks; GUI status (uptime, file count); spawn order; README; docs/TS-HIERARCHY.md, MASTER-PLAN.md; panic shows node/weight |

## Next ideas

- **smoltcp** full stack (TCP/UDP) with loopback or real NIC
- **Virtio** drivers (block, net)
- **Preemptive** scheduling (still TS-weighted)
- **Real userspace** (ring 3) + syscall ABI
- **More nodes** (e.g. device classes, namespaces) with dynamic weight updates

---

*Kernel is alpha. Weight only. No fallbacks.*
