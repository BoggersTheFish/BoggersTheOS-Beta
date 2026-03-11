use alloc::boxed::Box;
use alloc::string::String;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

pub mod executor;
pub mod keyboard;
pub mod simple_executor;

/// Default node for unassigned user-level tasks. TS RULE: scheduling prioritizes higher node weight — kernel supremacy.
pub const DEFAULT_TASK_NODE_ID: &str = "user_tasks";

pub struct Task {
    pub id: TaskId,
    /// Node this task belongs to; used for TS-weighted scheduling. Default "user_tasks".
    pub node_id: String,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Spawn a task in the default "user_tasks" node.
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task::new_with_node(future, DEFAULT_TASK_NODE_ID)
    }

    /// Spawn a task in the given TS node. Node must be registered (e.g. "task_executor", "interrupt_manager").
    pub fn new_with_node(future: impl Future<Output = ()> + 'static, node_id: &str) -> Task {
        Task {
            id: TaskId::new(),
            node_id: String::from(node_id),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}
