//! Types related to task management

use super::TaskContext;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
  /// The task status in it's lifecycle
  pub task_status: TaskStatus,
  /// The task context
  pub task_cx: TaskContext,
  /// The syscalls counter
  pub syscalls_cnt: [u32; 512],
}

impl TaskControlBlock {
  /// Increment the syscall counter for the given syscall id.
  pub fn syscall_cnt_index(&self, syscall_id: usize) -> u32 {
    self.syscalls_cnt[syscall_id]
  }

  /// Increment the syscall counter for the given syscall id.
  pub fn syscall_cnt_plus(&mut self, syscall_id: usize) {
    self.syscalls_cnt[syscall_id] += 1;
  }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
  /// uninitialized
  UnInit,
  /// ready to run
  Ready,
  /// running
  Running,
  /// exited
  Exited,
}
