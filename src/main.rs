extern crate some_macros;
use some_macros::{make_answer, wysi};

make_answer!();

#[wysi]
fn some_func(num: i32) -> i32 {
    num + 32
}

fn main() {
    println!("{}", some_func(5));
}
