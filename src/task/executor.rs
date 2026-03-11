use super::{Task, TaskId};
use crate::ts::{TS_REGISTRY, set_current_task_node};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake, vec::Vec};
use core::cmp::Ordering;
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    /// TS RULE: scheduling prioritizes higher node weight — kernel supremacy.
    /// Drain ready queue, sort by owning node's weight (desc), then poll in that order.
    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        // Drain task_queue into a vec so we can sort by weight
        let mut ready: Vec<TaskId> = Vec::new();
        while let Some(tid) = task_queue.pop() {
            ready.push(tid);
        }

        if ready.is_empty() {
            return;
        }

        // Get weight for each task from TS registry (by task's node_id)
        let reg = TS_REGISTRY.lock();
        let mut with_weights: Vec<(TaskId, f32)> = ready
            .iter()
            .filter_map(|tid| {
                tasks.get(tid).map(|t| {
                    let w = reg.get_weight(t.node_id.as_str()).unwrap_or(0.0);
                    (*tid, w)
                })
            })
            .collect();
        drop(reg);

        // Sort descending by weight (higher weight first); stable sort keeps order within same weight
        with_weights.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        // Phase 1.2: TS RULE: preemption point — when timer requested, re-queue all and reschedule by weight next cycle.
        if crate::uptime::take_preempt_requested() {
            crate::println!("TS schedule: preempt, rescheduling by weight");
            for (tid, _) in &with_weights {
                let _ = task_queue.push(*tid);
            }
            return;
        }

        for (task_id, weight) in with_weights {
            let task = match tasks.get_mut(&task_id) {
                Some(t) => t,
                None => continue,
            };
            let node_id = task.node_id.clone();
            let _ = task;
            crate::println!(
                "TS schedule: picking task '{}' from node '{}' (weight {:.2})",
                task_id.0,
                node_id,
                weight
            );
            let task = match tasks.get_mut(&task_id) {
                Some(t) => t,
                None => continue,
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);
            set_current_task_node(Some(&task.node_id));
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
            set_current_task_node(None);
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
