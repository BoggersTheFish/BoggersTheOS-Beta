//! TS (Trust/Strength) Registry — central hierarchy for BoggersTheOS-Alpha.
//!
//! **Philosophy**: Every subsystem is a "node" with weight in [0.0, 1.0].
//! The kernel is always the strongest node at fixed weight 1.0. All conflicts
//! (scheduling, resources, syscalls, interrupts) are resolved by comparing
//! node weights: higher wins; kernel always wins ties. No bypass logic.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;
use spin::Mutex;

/// Kernel node id — the alpha. Fixed weight 1.0.
pub const KERNEL_NODE_ID: &str = "kernel";

/// Weight must be in [0.0, 1.0]. Kernel is always 1.0.
pub const KERNEL_WEIGHT: f32 = 1.0;

/// Status of a node in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsNodeStatus {
    Active,
    Suspended,
}

/// A single node in the TS hierarchy. Id is the string key (e.g. "kernel", "memory_manager").
#[derive(Debug, Clone)]
pub struct TsNode {
    pub id: String,
    pub weight: f32,
    pub parent: Option<String>,
    pub deps: Vec<String>,
    pub status: TsNodeStatus,
}

/// Central registry: all nodes keyed by id string. Singleton via lazy_static.
pub struct TsRegistry {
    nodes: BTreeMap<String, TsNode>,
}

impl TsRegistry {
    pub const fn new() -> Self {
        TsRegistry {
            nodes: BTreeMap::new(),
        }
    }

    /// Register a new node by string id.
    /// TS RULE: weight comparison ONLY — no bypass. Weights must be in [0.0, 1.0].
    /// TS RULE: only "kernel" may have weight 1.0; otherwise panic (kernel supremacy).
    pub fn register_node(&mut self, id: &str, weight: f32, parent: Option<&str>, deps: Vec<&str>) {
        assert!(
            weight >= 0.0 && weight <= 1.0,
            "TS: weight must be in [0.0, 1.0]"
        );
        if (weight - KERNEL_WEIGHT).abs() < 0.001 && id != KERNEL_NODE_ID {
            panic!("TS violation: only kernel may have weight 1.0");
        }
        if weight > KERNEL_WEIGHT {
            panic!("TS violation: weight > 1.0 not allowed");
        }
        let node = TsNode {
            id: String::from(id),
            weight,
            parent: parent.map(String::from),
            deps: deps.iter().map(|s| String::from(*s)).collect(),
            status: TsNodeStatus::Active,
        };
        self.nodes.insert(String::from(id), node);
    }

    /// Register the kernel node. Must be called exactly once at boot (weight 1.0).
    pub fn register_kernel(&mut self) {
        if self.nodes.contains_key(KERNEL_NODE_ID) {
            return; // idempotent
        }
        self.register_node(KERNEL_NODE_ID, KERNEL_WEIGHT, None, Vec::new());
    }

    /// Get weight of a node by string id. Kernel is always 1.0. Returns None if unknown.
    pub fn get_weight(&self, id: &str) -> Option<f32> {
        self.nodes.get(id).map(|n| n.weight)
    }

    /// TS RULE: weight comparison ONLY — no bypass. Kernel always wins ties/involvement.
    /// Returns the winning node id.
    pub fn resolve_conflict(&self, a: &str, b: &str) -> String {
        if a == KERNEL_NODE_ID || b == KERNEL_NODE_ID {
            return String::from(KERNEL_NODE_ID);
        }
        let wa = self.get_weight(a).unwrap_or(0.0);
        let wb = self.get_weight(b).unwrap_or(0.0);
        match wa.partial_cmp(&wb) {
            Some(Ordering::Greater) => String::from(a),
            Some(Ordering::Less) => String::from(b),
            Some(Ordering::Equal) | None => String::from(KERNEL_NODE_ID), // tie -> kernel wins
        }
    }

    /// Root nodes (no parent) for tree print. Kernel has no parent.
    pub fn roots(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter(|n| n.parent.is_none())
            .map(|n| n.id.clone())
            .collect::<Vec<_>>()
    }

    /// Children of a given node (parent == id).
    pub fn children(&self, id: &str) -> Vec<String> {
        self.nodes
            .values()
            .filter(|n| n.parent.as_deref() == Some(id))
            .map(|n| n.id.clone())
            .collect::<Vec<_>>()
    }

    pub fn get(&self, id: &str) -> Option<&TsNode> {
        self.nodes.get(id)
    }
}

use lazy_static::lazy_static;

lazy_static! {
    /// Global TS registry. Kernel node is registered on first init.
    pub static ref TS_REGISTRY: Mutex<TsRegistry> = Mutex::new(TsRegistry::new());

    /// Currently executing task's node id (set by executor around poll). None = kernel context.
    static ref CURRENT_TASK_NODE_ID: Mutex<Option<String>> = Mutex::new(None);
}

/// Set the current execution context's node (for TS weight checks). None = kernel.
/// Executor sets this before polling a task and clears after.
pub fn set_current_task_node(node_id: Option<&str>) {
    *CURRENT_TASK_NODE_ID.lock() = node_id.map(String::from);
}

/// Get the current execution context's node id. None = no task (kernel context).
pub fn current_node_id() -> Option<String> {
    CURRENT_TASK_NODE_ID.lock().clone()
}

/// Weight of the current context. Falls back to kernel (1.0) if no task.
pub fn current_node_weight() -> f32 {
    let id = CURRENT_TASK_NODE_ID
        .lock()
        .clone()
        .unwrap_or_else(|| String::from(KERNEL_NODE_ID));
    let reg = TS_REGISTRY.lock();
    reg.get_weight(&id).unwrap_or(KERNEL_WEIGHT)
}

/// TS RULE: enforce weight check before resource use — kernel supremacy.
/// Returns Err(()) if current context weight < min_weight (logs violation); Ok(()) otherwise.
pub fn enforce_min_weight(op: &str, min_weight: f32) -> Result<(), ()> {
    let node = current_node_id();
    let node_str = node.as_deref().unwrap_or(KERNEL_NODE_ID);
    let weight = current_node_weight();
    if weight < min_weight {
        crate::println!(
            "TS violation: {} denied - node '{}' weight {:.2} < {:.2}",
            op,
            node_str,
            weight,
            min_weight
        );
        return Err(());
    }
    Ok(())
}

/// Initialize TS: register kernel at 1.0. Call once during early boot.
pub fn init() {
    let mut reg = TS_REGISTRY.lock();
    reg.register_kernel();
}

/// Print ASCII tree of all nodes (kernel at root). Call after init and optional fake nodes.
pub fn print_hierarchy_dump() {
    use crate::println;
    let reg = TS_REGISTRY.lock();
    println!("=== TS Hierarchy (kernel = alpha, weight 1.0) ===");
    let mut roots = reg.roots();
    roots.sort_by(|a, b| {
        // kernel first
        if a.as_str() == KERNEL_NODE_ID {
            return Ordering::Less;
        }
        if b.as_str() == KERNEL_NODE_ID {
            return Ordering::Greater;
        }
        a.cmp(b)
    });
    for r in &roots {
        print_node_tree(&reg, r, 0);
    }
    println!("=== end TS Hierarchy ===");
}

fn print_node_tree(reg: &TsRegistry, id: &str, depth: usize) {
    use crate::println;
    let indent = "  ".repeat(depth);
    let node = match reg.get(id) {
        Some(n) => n,
        None => return,
    };
    let weight_display = if (node.weight - KERNEL_WEIGHT).abs() < 0.001 {
        String::from("1.0 (kernel)")
    } else {
        alloc::format!("{:.2}", node.weight)
    };
    let prefix = if depth == 0 { "" } else { "|- " };
    println!(
        "{}{}[{}] (weight: {})",
        indent, prefix, node.id, weight_display
    );
    let children = reg.children(id);
    let mut sorted_children = children;
    sorted_children.sort_by(|a, b| {
        let wa = reg.get_weight(a).unwrap_or(0.0);
        let wb = reg.get_weight(b).unwrap_or(0.0);
        wb.partial_cmp(&wa).unwrap_or(Ordering::Equal) // desc weight
    });
    for c in &sorted_children {
        print_node_tree(reg, c, depth + 1);
    }
}
