extern crate some_macros;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::FutureExt;
use some_macros::labeled_block;

use secrets_structs::{LabelNonIdem, LabelTimely, Labeled};
use async_runtime::{executor::spawn_executor_thread, karma::{Karma, radio::{Radio, RadioFuture, RadioInputMsg}}, timer::TimerFuture};

async fn foo() {
    let karma = Karma::new(Radio::new(1));

    let x = labeled_block!(LabelTimely<1000> |karma| {
        let f = RadioFuture::new(&mut karma, RadioInputMsg::Init);
        let out = f.await.unwrap();
        println!("out: {:?}", out);

        todo!()
    });
}

fn main() {
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        foo().await;
    });

    drop(spawner);

    handle.join().unwrap();
}
