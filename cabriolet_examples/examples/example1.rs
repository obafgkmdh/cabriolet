extern crate some_macros;

use std::{
    sync::Arc,
};

use futures::FutureExt;
use some_macros::labeled_block;

use async_runtime::{
    executor::spawn_executor_thread,
    karma::{
        Karma,
        radio::{Radio, RadioFuture, RadioFutureCreateArg, RadioInputMsg},
    },
};
use secrets_structs::{LabelNonIdem, LabelTimely, Labeled};

async fn foo() {
    let karma = Karma::new(Radio::new(1));

    let x = labeled_block!(LabelTimely<1000> |karma| {
        println!("BEGINNING");

        let f = RadioFuture::new(&mut karma, RadioFutureCreateArg::InputMsg(RadioInputMsg::Init));
        let out = f.await.unwrap();
        println!("out: {:?}", out);

        let f = RadioFuture::new(&mut karma, RadioFutureCreateArg::AwaitReceive);
        let out = f.await.unwrap();
        println!("out: {:?}", out);


        match out {
            async_runtime::karma::radio::RadioOutputMsg::DataReceived(data) => data,
            _ => panic!("Received wrong output message type"),
        }
    });

    let y = x.endorse_idempotent().await;
}

fn main() {
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        foo().await;
    });

    drop(spawner);

    handle.join().unwrap();
}
