extern crate some_macros;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::FutureExt;
use some_macros::labeled_block;

use secrets_structs::{LabelNonIdem, LabelTimely, Labeled};
use toy_async::{executor::spawn_executor_thread, timer::TimerFuture};

async fn foo() {
    let x = 1;

    let before = Instant::now();
    let y = labeled_block!(LabelTimely<10000> || {
        let timer_future = TimerFuture::new(Duration::from_millis(3000));
        timer_future.await;

        let now = Instant::now();
        let elapsed = now - before;

        elapsed.as_millis() as i32
    });

    let n: Labeled<i32, LabelTimely<100>> = labeled_block!(LabelTimely<100> |y| {
        let yp = y.endorse_idempotent().await;

        yp + 67
    });

    println!("n: {:?}", n.endorse_idempotent().await);

    let z = labeled_block!(LabelTimely<100> |y| {
        std::thread::sleep(std::time::Duration::from_millis(1000));

        let sigma = x + unwrap_labeled(y);

        sigma
    });

    println!("result: {:?}", z.endorse_idempotent().await);

    let w = labeled_block!(LabelNonIdem |y| {
        std::thread::sleep(std::time::Duration::from_millis(1000));

        let sigma = x + unwrap_labeled(y);

        wrap_labeled(sigma)
    });

    println!("result: {:?}", w.endorse_idempotent().await);
}

fn main() {
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        foo().await;
    });

    drop(spawner);

    handle.join().unwrap();
}
