//! Process management syscalls

use core::{mem::size_of, ptr::addr_of};

use crate::{
  mm::{copy_to_user, translated_byte_buffer, VirtAddr},
  task::{
    change_program_brk, current_user_token, debug_mem_view, exit_current_and_run_next, map_mem_area, suspend_current_and_run_next, trace_syscall, unmap_mem_area
  },
  timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
  pub sec: usize,
  pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
  trace!("kernel: sys_exit");
  exit_current_and_run_next();
  panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
  trace!("kernel: sys_yield");
  suspend_current_and_run_next();
  0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
  trace!("kernel: sys_get_time");
  // debug_mem_view();

  let us = get_time_us();
  let time_val = TimeVal {
    sec: us / 1_000_000,
    usec: us % 1_000_000,
  };

  copy_to_user(
    current_user_token(),
    ts as *mut u8,
    addr_of!(time_val) as *const u8,
    size_of::<TimeVal>(),
  );
  0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// 1. 传入的地址超出虚拟内存范围
/// 2. 读取和写入时，用户权限不足
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
  trace!("kernel: sys_trace");
  let token = current_user_token();

  match trace_request {
    0 => {
      // 要求访问的地址存在
      if id & !((1 << 39) - 1) != 0 {
        return  -1;
      };
      // 返回第一页的第一个字节
      if let Some(res) = translated_byte_buffer(token, id as *const u8, 1) {
        info!("res[0][0]: {:?}", res[0][0]);
        res[0][0] as isize
      } else {
        warn!("sys_trace error");
        -1
      }
    }
    1 => {
      // 要求访问的地址存在
      if id & !((1 << 39) - 1) != 0 {
        return  -1;
      };
      // 小端序机器中，地址指向的是低字节
      let input = data as u8;
      if copy_to_user(token, id as *mut u8, addr_of!(input) as *const u8, 1) {
        0
      } else {
        -1
      }
    }
    2 => trace_syscall(id) as isize,
    _ => -1,
  }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
  trace!("kernel: sys_mmap");
  debug_mem_view();

  // 1. start 必须按页对齐
  // 2. prot 除前 3 位外，其余位必须为 0
  // 3. prot 至少在可读，可写，可执行中占一个
  if (start & 0xFFF) != 0 || (prot & !0x7) != 0 || (prot & 0x7) == 0 {
    return -1;
  }

  // 4. 计算映射区域
  let start_va = VirtAddr(start);
  let end_va = VirtAddr(start + len);
  
  // 5. 进行映射
  let flag = map_mem_area(start_va, end_va, prot as u8);
  debug_mem_view();

  if !flag {
    warn!("map failed");
    return -1;
  }
  0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
  trace!("kernel: sys_munmap");
  debug_mem_view();

  // 1. start 必须按页对齐
  if (start & 0xFFF) != 0 {
    warn!("start: 0x{start} is not aligned");
    return -1;
  }
  
  // 2. 计算映射区域
  let start_va = VirtAddr(start);
  let end_va = VirtAddr(start + len);
  let flag = unmap_mem_area(start_va, end_va);

  debug_mem_view();

  if !flag {
    warn!("unmap failed");
    return -1;
  }
  0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
  trace!("kernel: sys_sbrk");
  if let Some(old_brk) = change_program_brk(size) {
    old_brk as isize
  } else {
    -1
  }
}
