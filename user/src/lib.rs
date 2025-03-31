#![feature(linkage)]

use core::panic;
use syscall::{sys_exit, sys_write};
mod syscall;

pub fn write(fd: usize, buf: &[u8]) -> isize {
  sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
  sys_exit(exit_code)
}

/// clear BSS segment
fn clear_bss() {
  extern "C" {
    fn sbss();
    fn ebss();
  }
  unsafe {
    core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
      .fill(0);
  }
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
  clear_bss();
  exit(main());
  panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
  panic!("Cannot find main!");
}
