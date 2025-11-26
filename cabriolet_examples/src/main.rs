extern crate some_macros;
use some_macros::{labeled_block};

use secrets_structs::{LabelNonIdem, LabelTimely, Labeled};

fn main() {
    let x = 1;

    let y: Labeled<i32, LabelTimely> = Labeled::new(5);

    let z = labeled_block!(LabelNonIdem {
        let sigma = x + unwrap_labeled(y);

        wrap_labeled(sigma)
    });

    println!("result: {:?}", z);
}
