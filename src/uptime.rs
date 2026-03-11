//! Simple uptime tick counter. TS RULE: final polish maintains kernel supremacy.
//! Incremented by timer interrupt; read by GUI/status.
//! Phase 1.2: preemption request flag for TS-weighted time-slicing.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static UPTIME_TICKS: AtomicU64 = AtomicU64::new(0);

/// Every this many ticks we request a scheduler preemption (TS-weighted reschedule).
const PREEMPT_INTERVAL: u64 = 20;

/// Set by timer when it's time to yield and re-pick task by weight. Cleared by executor.
static PREEMPT_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Call from timer interrupt handler each tick.
pub fn tick() {
    let t = UPTIME_TICKS.fetch_add(1, Ordering::Relaxed);
    // TS RULE: periodic preemption preserves fair time-slicing; kernel supremacy unchanged.
    if t > 0 && t % PREEMPT_INTERVAL == 0 {
        PREEMPT_REQUESTED.store(true, Ordering::Relaxed);
    }
}

/// Check and clear preemption request. Returns true if preemption was requested.
pub fn take_preempt_requested() -> bool {
    PREEMPT_REQUESTED.swap(false, Ordering::Relaxed)
}

/// Current uptime in ticks (for status display).
pub fn ticks() -> u64 {
    UPTIME_TICKS.load(Ordering::Relaxed)
}
