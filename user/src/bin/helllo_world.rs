use usr_lib::write;

#[macro_use]
extern crate user_lib;

fn main() -> i32 {
  let buf = "Hello world!";
  write(1, buf.as_bytes())
}
