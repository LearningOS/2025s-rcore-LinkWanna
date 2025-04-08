//! Types related to task management

use alloc::collections::btree_map::BTreeMap;

use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::mm::{
  kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::trap::{trap_handler, TrapContext};

/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
  /// Save task context
  pub task_cx: TaskContext,

  /// Maintain the execution status of the current process
  pub task_status: TaskStatus,

  /// Application address space
  /// 任务地址空间
  pub memory_set: MemorySet,

  /// The phys page number of trap context
  pub trap_cx_ppn: PhysPageNum,

  /// The size(top addr) of program which is loaded from elf file
  /// 程序大小
  pub base_size: usize,

  /// Heap bottom
  /// 堆底
  pub heap_bottom: usize,

  /// Program break
  pub program_brk: usize,

  /// trace record
  pub trace_record: BTreeMap<usize, usize>,
}

impl TaskControlBlock {
  /// get the trap context
  pub fn get_trap_cx(&self) -> &'static mut TrapContext {
    self.trap_cx_ppn.get_mut()
  }

  /// get the user token
  pub fn get_user_token(&self) -> usize {
    self.memory_set.token()
  }

  /// Based on the elf info in program, build the contents of task in a new address space
  /// 基于 elf 文件信息，构建任务在新地址空间中的内容
  pub fn new(elf_data: &[u8], app_id: usize) -> Self {
    // memory_set with elf program headers/trampoline/trap context/user stack
    let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

    let trap_cx_ppn = memory_set
      .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
      .unwrap()
      .ppn();
    let task_status = TaskStatus::Ready;

    // map a kernel-stack in kernel space
    // 将内核栈映射到内核空间
    let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
    KERNEL_SPACE.exclusive_access().insert_framed_area(
      kernel_stack_bottom.into(),
      kernel_stack_top.into(),
      MapPermission::R | MapPermission::W,
    );

    let task_control_block = Self {
      task_status,
      task_cx: TaskContext::goto_trap_return(kernel_stack_top),
      memory_set,
      trap_cx_ppn,
      base_size: user_sp,
      heap_bottom: user_sp, 
      program_brk: user_sp,
      trace_record: BTreeMap::new(),
    };

    // prepare TrapContext in user space
    let trap_cx = task_control_block.get_trap_cx();
    *trap_cx = TrapContext::app_init_context(
      entry_point,
      user_sp,
      KERNEL_SPACE.exclusive_access().token(),
      kernel_stack_top,
      trap_handler as usize,
    );

    task_control_block
  }

  /// change the location of the program break. return None if failed.
  /// 修改程序堆的大小，返回旧的堆大小
  pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
    let old_break = self.program_brk;
    let new_brk = self.program_brk as isize + size as isize;
    if new_brk < self.heap_bottom as isize {
      return None;
    }

    let result = if size < 0 {
      self
        .memory_set
        .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
    } else {
      self
        .memory_set
        .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
    };

    if result {
      self.program_brk = new_brk as usize;
      Some(old_break)
    } else {
      None
    }
  }

  /// 进行区域映射
  pub fn map_mem_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: u8) -> bool {
    self
      .memory_set
      .insert_framed_area(start_va, end_va, MapPermission::from_bits(permission << 1).unwrap() | MapPermission::U)
  }

  /// 进行区域取消映射
  pub fn unmap_mem_area(&mut self, start_va: VirtAddr, end_va: VirtAddr) -> bool {
    self
      .memory_set
      .remove_framed_area(start_va, end_va)
  }

  /// 获取指定系统调用的次数
  pub fn trace_syscall(&mut self, syscall_id: usize) -> usize {
    self.trace_record.entry(syscall_id).or_insert(0).clone()
  }

  /// 递增系统调用的次数
  pub fn trace_syscall_step(&mut self, syscall_id: usize) {
    let count = self.trace_record.entry(syscall_id).or_insert(0);
    *count += 1;
  }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
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
