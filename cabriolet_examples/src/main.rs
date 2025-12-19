extern crate some_macros;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::FutureExt;
use some_macros::labeled_block;

use secrets_structs::{LabelNonIdem, LabelTimely, Labeled, TimelyClosure};
use toy_async::{executor::spawn_executor_thread, timer::TimerFuture};

async fn foo() {
    let x = 1;

    let before = Instant::now();
    let m: TimelyClosure<_> = Arc::new(move || {
        async move {
            let timer_future = TimerFuture::new(Duration::from_millis(1500));
            timer_future.await;

            let now = Instant::now();
            let elapsed = now - before;

            elapsed.as_millis() as i32
        }
        .boxed()
    });

    let y: Labeled<i32, LabelTimely<10>> = Labeled::new(m);

    let nc: TimelyClosure<_> = Arc::new(move || {
        let yclone: Labeled<i32, LabelTimely<10>> = y.clone();

        async move {
            let yp = yclone.endorse_idempotent().await;

            yp + 67
        }
        .boxed()
    });
    let n: Labeled<i32, LabelTimely<10>> = Labeled::new(nc);

    println!("n: {:?}", n.endorse_idempotent().await);

    //let mut z = labeled_block!(LabelNonIdem {
    //    std::thread::sleep(std::time::Duration::from_millis(1500));

    //    let sigma = x + unwrap_labeled(y);

    //    wrap_labeled(sigma)
    //});

    //println!("result: {:?}", z.endorse_idempotent().await);
}

fn main() {
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        foo().await;
    });

    drop(spawner);

    handle.join().unwrap();
}
