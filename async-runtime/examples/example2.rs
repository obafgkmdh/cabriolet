use async_runtime::{
    executor::spawn_executor_thread,
    karma::{
        Karma,
        radio::{Radio, RadioFuture, RadioInputMsg},
    },
};

fn main() {
    // start the runtime thread
    let (handle, spawner) = spawn_executor_thread();

    spawner.spawn(async {
        let mut karma = Karma::new(Radio::new(1));

        let msg = RadioInputMsg::Init;
        let r1_f = RadioFuture::new(&mut karma, msg);
        let r1_out = r1_f.await.unwrap();

        println!("r1_out: {:?}", r1_out);

        let msg = RadioInputMsg::StateTransmit;
        let r2_f = RadioFuture::new(&mut karma, msg);
        let r2_out = r2_f.await;
        // StateTransmit should have no response
        assert!(r2_out.is_none());
        println!("r2_out: {:?}", r2_out);

        let msg = RadioInputMsg::Send(vec![1, 2, 3, 4]);
        let r3_f = RadioFuture::new(&mut karma, msg);
        let r3_out = r3_f.await.unwrap();
        println!("r3_out: {:?}", r3_out);
    });

    drop(spawner);

    handle.join().unwrap();
}
