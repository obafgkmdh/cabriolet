extern crate some_macros;

use std::sync::Arc;

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

    let karma2 = Karma::new(Radio::new(2));
    let y = labeled_block!(LabelTimely<1000> |x, karma2| {
        println!("BEGINNING 2");

        let f = RadioFuture::new(&mut karma2, RadioFutureCreateArg::InputMsg(RadioInputMsg::Init));
        let out = f.await.unwrap();
        println!("out2: {:?}", out);

        let f = RadioFuture::new(&mut karma2, RadioFutureCreateArg::AwaitReceive);
        let out = f.await.unwrap();
        println!("out2: {:?}", out);

        let data = unwrap_labeled(x);

        // result is x + radio_response_y
        let data2 = match out {
            async_runtime::karma::radio::RadioOutputMsg::DataReceived(data) => data,
            _ => panic!("Received wrong output message type"),
        };

        let result: Vec<_> = data.into_iter().zip(data2.into_iter()).collect();

        result
    });

    let z = y.endorse_idempotent().await;
    println!("result: {:?}", z);
}

fn main() {
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        foo().await;
    });

    drop(spawner);

    handle.join().unwrap();
}
