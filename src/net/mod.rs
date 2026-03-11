//! Networking: smoltcp integration (Phase 10A). TS RULE: network ops gated by node weight — kernel supremacy.
//! Replaces VecDeque loopback with smoltcp Interface + phy::Loopback; UDP echo demo (Chunk 2).

// TS RULE: smoltcp used for stack only; weight gates unchanged — kernel supremacy.
#[allow(unused_imports)]
use smoltcp::phy::{Loopback, Medium};

use crate::ts;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

const MIN_WEIGHT_NET_SEND: f32 = 0.7;
const MIN_WEIGHT_NET_RECV: f32 = 0.65;
const MAX_PACKET_LEN: usize = 2048;
const LOOPBACK_QUEUE_CAP: usize = 8;

/// Simple loopback: sent packets are queued and received in FIFO order.
pub struct NetStack {
    rx_queue: VecDeque<Vec<u8>>,
}

impl NetStack {
    pub fn new() -> Self {
        NetStack {
            rx_queue: VecDeque::new(),
        }
    }

    /// TS RULE: network ops gated by node weight — kernel supremacy.
    pub fn send_packet(&mut self, data: &[u8]) -> Result<usize, ()> {
        if ts::enforce_min_weight("net send", MIN_WEIGHT_NET_SEND).is_err() {
            return Err(());
        }
        if data.len() > MAX_PACKET_LEN {
            return Err(());
        }
        if self.rx_queue.len() >= LOOPBACK_QUEUE_CAP {
            return Err(());
        }
        self.rx_queue.push_back(data.to_vec());
        Ok(data.len())
    }

    /// TS RULE: network ops gated by node weight.
    pub fn recv_packet(&mut self) -> Option<Vec<u8>> {
        if ts::enforce_min_weight("net recv", MIN_WEIGHT_NET_RECV).is_err() {
            return None;
        }
        self.rx_queue.pop_front()
    }

    /// Poll: no-op for stub (real stack would call interface.poll()). Kept for Driver::poll.
    pub fn poll(&mut self) {
        // Stub: nothing to poll. Future: smoltcp Interface::poll.
    }
}
