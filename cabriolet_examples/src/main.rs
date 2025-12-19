extern crate some_macros;

use std::time::Instant;

use futures::{FutureExt, future::{BoxFuture, ready}};
use some_macros::labeled_block;

use secrets_structs::{LabelNonIdem, LabelTimely, Labeled};
use toy_async::executor::spawn_executor_thread;

async fn foo() {
    let x = 1;

    let before = Instant::now();
    // TODO: it would be nice if we could write it like this, without the ready() thing
    //let m: Box<dyn Fn() -> BoxFuture<'static, i32> + Send> = Box::new(async move || {
    //    let now = Instant::now();
    //    let elapsed = now - before;
    //    elapsed.as_millis() as i32
    //});
    let m: Box<dyn Fn() -> BoxFuture<'static, i32> + Send> = Box::new(move || {
        let now = Instant::now();
        let elapsed = now - before;

        ready(elapsed.as_millis() as i32).boxed()
    });
    let mut y: Labeled<i32, LabelTimely<10>> = Labeled::new(m);

    let mut z = labeled_block!(LabelNonIdem {
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let sigma = x + unwrap_labeled(y);

        wrap_labeled(sigma)
    });

    println!("result: {:?}", z.endorse_idempotent().await);
}

fn main() {
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        foo().await;
    });

    drop(spawner);

    handle.join().unwrap();
}
