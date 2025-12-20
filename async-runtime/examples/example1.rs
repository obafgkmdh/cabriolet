use std::{thread::sleep, time::Duration};

use async_runtime::executor::spawn_executor_thread;
use async_runtime::temperature_sensor::{TemperatureSensor, TemperatureSensorFuture};
use async_runtime::timer::TimerFuture;

fn main() {
    // start the runtime thread
    let (handle, spawner) = spawn_executor_thread();

    //spawner.spawn(async {
    //    println!("Before timer1!");
    //    let timer = TimerFuture::new(Duration::from_secs(3));
    //    let result = timer.await;
    //    println!("After timer1: {:?}", result);

    //    println!("Before timer2!");
    //    let timer = TimerFuture::new(Duration::from_secs(1));
    //    let result = timer.await;
    //    println!("After timer2: {:?}", result);
    //});

    spawner.spawn(async {
        let mut sensor = TemperatureSensor::new();
        loop {
            let temps = sensor.read().unwrap().await;
            sleep(Duration::from_secs(5));

            println!("received temps: {:#?}", temps);
        }
    });

    println!("OOOOOOOOOOOOOOOOOOOOO");

    drop(spawner);

    handle.join().unwrap();
}
