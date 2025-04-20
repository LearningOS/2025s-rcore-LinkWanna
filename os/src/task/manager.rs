//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::binary_heap::BinaryHeap;
use alloc::sync::Arc;
use lazy_static::*;

// ///A array of `TaskControlBlock` that is thread-safe
// pub struct TaskManager {
//     ready_queue: VecDeque<Arc<TaskControlBlock>>,
// }

// /// A simple FIFO scheduler.
// impl TaskManager {
//     ///Creat an empty TaskManager
//     pub fn new() -> Self {
//         Self {
//             ready_queue: VecDeque::new(),
//         }
//     }
//     /// Add process back to ready queue
//     pub fn add(&mut self, task: Arc<TaskControlBlock>) {
//         self.ready_queue.push_back(task);
//     }
//     /// Take a process out of the ready queue
//     pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
//         self.ready_queue.pop_front()
//     }
// }

// lazy_static! {
//     /// TASK_MANAGER instance through lazy_static!
//     pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
//         unsafe { UPSafeCell::new(TaskManager::new()) };
// }

// /// Add process to ready queue
// pub fn add_task(task: Arc<TaskControlBlock>) {
//     //trace!("kernel: TaskManager::add_task");
//     TASK_MANAGER.exclusive_access().add(task);
// }

// /// Take a process out of the ready queue
// pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
//     //trace!("kernel: TaskManager::fetch_task");
//     TASK_MANAGER.exclusive_access().fetch()
// }

///
pub struct StrideTaskManager {
    // stride队列
    stride_queue: BinaryHeap<Arc<TaskControlBlock>>,
}

impl StrideTaskManager {
    /// Create an empty StrideTaskManager
    pub fn new() -> Self {
        Self {
            stride_queue: BinaryHeap::new(),
        }
    }

    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.stride_queue.push(task);
    }

    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(task) = self.stride_queue.pop() {
            task.inner_exclusive_access().stride_step();
            Some(task)
        } else {
            None
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<StrideTaskManager> =
        unsafe { UPSafeCell::new(StrideTaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
