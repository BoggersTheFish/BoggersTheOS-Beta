# TS Hierarchy – Rules and Weights

BoggersTheOS-Beta resolves **all** conflicts by node weight. The kernel is the alpha (weight 1.0).

## Node list (boot default)

| id                 | weight | parent  | role                    |
|--------------------|--------|---------|-------------------------|
| kernel             | 1.0    | —       | Alpha; ties go to kernel |
| interrupt_manager  | 0.95   | kernel  | Interrupt handling      |
| memory_manager     | 0.9    | kernel  | Memory subsystem        |
| task_executor      | 0.85   | kernel  | Async task run queue    |
| vga_driver         | 0.8    | kernel  | VGA text                |
| timer_driver       | 0.82   | kernel  | Timer stub              |
| network_stack      | 0.75   | kernel  | Loopback net            |
| uart_driver        | 0.75   | kernel  | Sim UART                |
| ramdisk_fs         | 0.65   | kernel  | 2 MiB ramdisk FS        |
| gui_driver         | 0.6    | kernel  | Framebuffer GUI         |
| user_tasks         | 0.5    | kernel  | Default task node       |

## Rules

1. **Weights** must be in `[0.0, 1.0]`. Only `kernel` may have weight 1.0 → panic otherwise.
2. **Conflict resolution**: `resolve_conflict(a, b)` returns the node with higher weight; if either is kernel, or on tie, kernel wins.
3. **No bypass**: Scheduling, heap alloc, syscalls, driver ops, interrupts all check `enforce_min_weight(op, min)`; below min → deny and log (or panic where required).
4. **Driver init order**: Descending by weight (stronger nodes first).

## Conflict examples

- **Scheduling**: Ready tasks sorted by node weight desc; task from `task_executor` (0.85) is polled before task from `user_tasks` (0.5).
- **Heap alloc**: Large alloc needs weight ≥ 0.7; `user_tasks` (0.5) → violation.
- **FS write**: Min weight 0.6; `gui_driver` (0.6) allowed, `user_tasks` (0.5) denied.
- **Net send**: Min weight 0.7; `task_executor` (0.85) allowed, `user_tasks` (0.5) denied.
- **Syscall debug_hierarchy_dump**: Min 0.6; `user_tasks` (0.5) → denied.

## Tree dump

At end of boot, `print_hierarchy_dump()` prints an ASCII tree (children sorted by weight desc). Example:

```
[kernel] (weight: 1.0 (kernel))
  |- [interrupt_manager] (weight: 0.95)
  |- [memory_manager] (weight: 0.90)
  ...
```
