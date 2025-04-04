//! Memory management implementation
//! 内存管理实现
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{copy_to_user, translated_byte_buffer, PageTableEntry};
pub use page_table::{PTEFlags, PageTable};

/// initiate heap allocator, frame allocator and kernel space
/// 初始化堆分配器，帧分配器和内核空间
pub fn init() {
  heap_allocator::init_heap();
  frame_allocator::init_frame_allocator();
  KERNEL_SPACE.exclusive_access().activate();
}

/// debug view of the page table
pub fn debug_view(root_ppn: PhysPageNum, level: i32) {
  if level > 2 {
    return;
  }

  println!("Page Table:");
  for pte in root_ppn.get_pte_array() {
    if pte.is_valid() {
      let ppn = pte.ppn();
      println!("valid ppn_{} {:?}", level, ppn);
      debug_view(ppn, level + 1);
    }
  }
}
