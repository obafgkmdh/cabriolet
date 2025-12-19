extern crate some_macros;

use std::time::Instant;

use some_macros::labeled_block;

use secrets_structs::{LabelNonIdem, LabelTimely, Labeled};

fn main() {
    let x = 1;

    let before = Instant::now();
    let mut y: Labeled<i32, LabelTimely<10>> = Labeled::new(Box::new(move || {
        let now = Instant::now();
        let elapsed = now - before;

        elapsed.as_millis() as i32
    }) as Box<dyn Fn() -> i32>);

    let mut z = labeled_block!(LabelNonIdem {
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let sigma = x + unwrap_labeled(y);

        wrap_labeled(sigma)
    });

    println!("result: {:?}", z.endorse_idempotent());
}
