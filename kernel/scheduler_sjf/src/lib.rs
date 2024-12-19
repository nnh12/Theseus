extern crate alloc;

use alloc::collections::VecDeque;
use task::TaskRef;

pub struct Scheduler {
    idle_task: TaskRef,
    queue: VecDeque<TaskRef>,
}

impl Scheduler {
    pub const fn new(idle_task: TaskRef) -> Self {
        Self {
            idle_task,
            queue: VecDeque::new(),
        }
    }

    /// Compares the burst time of the idle task with another task.
    /// Returns `true` if the idle task's burst time is less than the other task's burst time.
    pub fn compare_idle_task_burst_time(&self, other_task: &TaskRef) -> bool {
        self.idle_task.burst_time() < other_task.burst_time()
    }
}

impl task::scheduler::Scheduler for FCFSScheduler {
    fn next(&mut self) -> TaskRef {
        if let Some(task) = self.queue.pop_front() {
            task
        } else {
            // Return an idle task if no other task is available
            self.idle_task.clone()
        }
    }

    fn add(&mut self, task: TaskRef) {
        self.queue.push_back(task);
    }

    fn busyness(&self) -> usize {
        self.queue.len()
    }

    fn remove(&mut self, task: &TaskRef) -> bool {
        if let Some(pos) = self.queue.iter().position(|t| t == task) {
            self.queue.remove(pos);
            true
        } else {
            false
        }
    }

    fn as_priority_scheduler(&mut self) -> Option<&mut dyn task::scheduler::PriorityScheduler> {
        None
    }

    fn drain(&mut self) -> Box<dyn Iterator<Item = TaskRef> + '_> {
        Box::new(self.queue.drain(..))
    }

    fn tasks(&self) -> Vec<TaskRef> {
        self.queue.clone().into()
    }
}
