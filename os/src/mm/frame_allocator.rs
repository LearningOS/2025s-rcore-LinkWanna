//! Implementation of [`FrameAllocator`] which
//! controls all the frames in the operating system.

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// tracker for physical page frame allocation and deallocation
/// 物理页跟踪器，用于自动释放物理页
pub struct FrameTracker {
  /// physical page number
  pub ppn: PhysPageNum,
}

impl FrameTracker {
  /// Create a new FrameTracker
  pub fn new(ppn: PhysPageNum) -> Self {
    // 获取对应页号的物理地址数组
    let bytes_array = ppn.get_bytes_array();
    // 清空当前页
    for i in bytes_array {
      *i = 0;
    }
    Self { ppn }
  }
}

impl Debug for FrameTracker {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
  }
}

impl Drop for FrameTracker {
  fn drop(&mut self) {
    frame_dealloc(self.ppn);
  }
}

trait FrameAllocator {
  fn new() -> Self;
  fn alloc(&mut self) -> Option<PhysPageNum>;
  fn dealloc(&mut self, ppn: PhysPageNum);
}

/// an implementation for frame allocator
/// 基于栈的分配器实现
pub struct StackFrameAllocator {
  current: usize,       // 当前页号
  end: usize,           // 结束页号
  recycled: Vec<usize>, // 回收的页号
}

impl StackFrameAllocator {
  pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
    self.current = l.0;
    self.end = r.0;
    // trace!("last {} Physical Frames.", self.end - self.current);
  }
}

impl FrameAllocator for StackFrameAllocator {
  fn new() -> Self {
    Self {
      current: 0,
      end: 0,
      recycled: Vec::new(),
    }
  }

  fn alloc(&mut self) -> Option<PhysPageNum> {
    if let Some(ppn) = self.recycled.pop() {
      // 优先分配回收的页
      Some(ppn.into())
    } else if self.current == self.end {
      None
    } else {
      // 单调递增分配
      self.current += 1;
      Some((self.current - 1).into())
    }
  }

  fn dealloc(&mut self, ppn: PhysPageNum) {
    let ppn = ppn.0;
    // validity check
    // 有效性检查：确保页号在范围内且未被回收
    if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
      panic!("Frame ppn={:#x} has not been allocated!", ppn);
    }
    // recycle
    self.recycled.push(ppn);
  }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// frame allocator instance through lazy_static!
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// initiate the frame allocator using `ekernel` and `MEMORY_END`
pub fn init_frame_allocator() {
  extern "C" {
    fn ekernel();
  }

  FRAME_ALLOCATOR.exclusive_access().init(
    PhysAddr::from(ekernel as usize).ceil(), // 物理页起始位置
    PhysAddr::from(MEMORY_END).floor(),      // 物理页结束位置
  );
}

/// Allocate a physical page frame in FrameTracker style
pub fn frame_alloc() -> Option<FrameTracker> {
  FRAME_ALLOCATOR
    .exclusive_access()
    .alloc()
    .map(FrameTracker::new)
}

/// Deallocate a physical page frame with a given ppn
pub fn frame_dealloc(ppn: PhysPageNum) {
  FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
/// a simple test for frame allocator
pub fn frame_allocator_test() {
  let mut v: Vec<FrameTracker> = Vec::new();
  for i in 0..5 {
    let frame = frame_alloc().unwrap();
    println!("{:?}", frame);
    v.push(frame);
  }
  v.clear();
  for i in 0..5 {
    let frame = frame_alloc().unwrap();
    println!("{:?}", frame);
    v.push(frame);
  }
  drop(v);
  println!("frame_allocator_test passed!");
}
