//! Implementation of [`PageTableEntry`] and [`PageTable`].

use super::{frame_alloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
  /// page table entry flags
  pub struct PTEFlags: u8 {
    /// Valid: 有效位
    const V = 1 << 0;
    /// Readable: 可读位
    const R = 1 << 1;
    /// Writable: 可写位
    const W = 1 << 2;
    /// eXecutable: 可执行位
    const X = 1 << 3;
    /// User: 用户位
    const U = 1 << 4;
    /// Global: 全局位
    const G = 1 << 5;
    /// Accessed: 访问位
    const A = 1 << 6;
    /// Dirty: 修改位(脏位)
    const D = 1 << 7;
  }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// 页表项
pub struct PageTableEntry {
  /// bits of page table entry
  pub bits: usize,
}

impl PageTableEntry {
  /// Create a new page table entry
  /// 分配一个物理页号和一个 flags
  pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
    PageTableEntry {
      bits: ppn.0 << 10 | flags.bits as usize,
    }
  }

  /// 创建一个空页表
  pub fn empty() -> Self {
    PageTableEntry { bits: 0 }
  }

  /// 从页表项中获取相应的物理页号
  pub fn ppn(&self) -> PhysPageNum {
    (self.bits >> 10 & ((1usize << 44) - 1)).into()
  }

  /// Get the flags from the page table entry
  pub fn flags(&self) -> PTEFlags {
    PTEFlags::from_bits(self.bits as u8).unwrap()
  }

  /// The page pointered by page table entry is valid?
  pub fn is_valid(&self) -> bool {
    (self.flags() & PTEFlags::V) != PTEFlags::empty()
  }

  /// The page pointered by page table entry is readable?
  pub fn readable(&self) -> bool {
    (self.flags() & PTEFlags::R) != PTEFlags::empty()
  }

  /// The page pointered by page table entry is writable?
  pub fn writable(&self) -> bool {
    (self.flags() & PTEFlags::W) != PTEFlags::empty()
  }

  /// The page pointered by page table entry is executable?
  pub fn executable(&self) -> bool {
    (self.flags() & PTEFlags::X) != PTEFlags::empty()
  }
}

/// page table structure
pub struct PageTable {
  root_ppn: PhysPageNum,
  frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
  /// Create a new page table
  pub fn new() -> Self {
    let frame = frame_alloc().unwrap();
    PageTable {
      root_ppn: frame.ppn,
      frames: vec![frame],
    }
  }

  /// Temporarily used to get arguments from user space.
  /// 临时用于从用户空间获取参数，创建一个空的页表结构
  pub fn from_token(satp: usize) -> Self {
    Self {
      root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
      frames: Vec::new(),
    }
  }

  /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
  /// 通过虚拟页号查找页表项，如果不存在则创建一个4KB的页表并分配一个物理帧
  fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
    let idxs = vpn.indexes();
    let mut ppn = self.root_ppn;
    let mut result: Option<&mut PageTableEntry> = None;
    for (i, idx) in idxs.iter().enumerate() {
      let pte = &mut ppn.get_pte_array()[*idx]; // 访问内存
      if i == 2 {
        result = Some(pte);
        break;
      }
      // 如果页表项无效，分配一个物理帧并更新页表项
      if !pte.is_valid() {
        let frame = frame_alloc().unwrap();
        *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
        self.frames.push(frame);
      }
      ppn = pte.ppn();
    }
    result
  }

  /// Find PageTableEntry by VirtPageNum
  /// 通过虚拟页号查找页表项，如果不存在则返回None
  fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
    let idxs = vpn.indexes();
    let mut ppn = self.root_ppn;
    let mut result: Option<&mut PageTableEntry> = None;
    for (i, idx) in idxs.iter().enumerate() {
      let pte = &mut ppn.get_pte_array()[*idx];
      // 如果是最后一级页表，直接返回
      if i == 2 {
        result = Some(pte);
        break;
      }
      if !pte.is_valid() {
        return None;
      }
      ppn = pte.ppn();
    }
    result
  }

  #[allow(unused)]
  /// 设置虚拟页号和物理页号的映射关系
  pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
    let pte = self.find_pte_create(vpn).unwrap();
    assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
    *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
  }

  #[allow(unused)]
  /// 移除虚拟页号和物理页号的映射关系
  pub fn unmap(&mut self, vpn: VirtPageNum) -> bool {
    if let Some(pte) = self.find_pte(vpn) {
      *pte = PageTableEntry::empty();
      true
    } else {
      debug!("vpn {:?} is invalid before unmapping", vpn);
      false
    }
    // assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
  }

  /// get the page table entry from the virtual page number
  pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
    self.find_pte(vpn).map(|pte| *pte)
  }

  /// get the token from the page table
  /// 使用 RV39 页表，参考《RISC-V 开源架构之道》 10.6 页式虚拟内存
  pub fn token(&self) -> usize {
    8usize << 60 | self.root_ppn.0
  }
}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
/// 将 U 态字节数组复制到 S 态中
pub fn translated_byte_buffer(
  token: usize,
  ptr: *const u8,
  len: usize,
) -> Option<Vec<&'static mut [u8]>> {
  let page_table = PageTable::from_token(token);
  let mut start = ptr as usize;
  let end = start + len;
  let mut v = Vec::new();
  // debug!("from {start} to {end}");

  while start < end {
    let start_va = VirtAddr::from(start);
    let mut vpn = start_va.floor();
    // 获取虚拟页号对应的物理页号
    // 若虚拟地址不存在，则直接报错
    
    let ppn = if let Some(pte) = page_table.translate(vpn) {
      if pte.is_valid() && pte.readable() {
        pte.ppn()
      } else {
        warn!("vpn: {:?} is invalid", vpn);
        return None;
      }
    } else {
      warn!("vpn: {:?} is invalid", vpn);
      // 不存在直接返回
      return None;
    };
    // let ppn = page_table.translate(vpn)?.ppn();
    // debug!("vpn: {}, ppn: {}", vpn.0, ppn.0);
    
    vpn.step();
    let mut end_va: VirtAddr = vpn.into();
    let a = VirtAddr::from(end);
    end_va = end_va.min(a);

    // 一页一页地读取
    if end_va.page_offset() == 0 {
      v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
    } else {
      v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
    }
    start = end_va.into();
  }
  Some(v)
}

/// 将 S 态字节数组复制到 U 态中
pub fn copy_to_user(token: usize, user_ptr: *mut u8, kernel_ptr: *const u8, len: usize) -> bool {
  let page_table = PageTable::from_token(token);
  let mut start = user_ptr as usize;
  let mut k_start = kernel_ptr as usize;
  let end = start + len;
  // debug!("from S:{start} to U:{end}");

  while start < end {
    let start_va = VirtAddr::from(start);
    let mut vpn = start_va.floor();

    let ppn = if let Some(pte) = page_table.translate(vpn) {
      // 不可写直接返回
      if !pte.writable() {
        return false;
      };
      pte.ppn()
    } else {
      return false;
    };

    // 获取结尾地址
    vpn.step();
    let mut end_va: VirtAddr = vpn.into();
    end_va = end_va.min(VirtAddr::from(end));

    // 获取目标地址指针
    let pa_offset: PhysAddr = PhysAddr((ppn.0 << 12) | start_va.page_offset());
    let dst: *mut u8 = pa_offset.0 as *mut u8;

    let cp_len = if end_va.page_offset() == 0 {
      4096 - start_va.page_offset()
    } else {
      end_va.page_offset() - start_va.page_offset()
    };

    // 不在同一个内存空间，可以保证不会重叠
    unsafe { dst.copy_from_nonoverlapping(k_start as *const u8, cp_len) };

    start += cp_len;
    k_start += cp_len;
  }
  true
}
