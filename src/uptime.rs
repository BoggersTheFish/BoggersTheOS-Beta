//! Simple uptime tick counter. TS RULE: final polish maintains kernel supremacy.
//! Incremented by timer interrupt; read by GUI/status.

use core::sync::atomic::{AtomicU64, Ordering};

static UPTIME_TICKS: AtomicU64 = AtomicU64::new(0);

/// Call from timer interrupt handler each tick.
pub fn tick() {
    UPTIME_TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Current uptime in ticks (for status display).
pub fn ticks() -> u64 {
    UPTIME_TICKS.load(Ordering::Relaxed)
}
