use std::{
    sync::{
        mpsc::{sync_channel, Receiver}, Arc
    },
    task::{Context, Poll},
    thread::{self, JoinHandle},
};

use futures::task::{ArcWake, waker_ref};

use crate::task::{Spawner, Task};

const MAX_TASKS: usize = 10_000;

pub struct Executor {
    receiver: Receiver<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.sender.try_send(arc_self.clone()).unwrap();
    }
}

fn build_runtime() -> (Executor, Spawner) {
    let (sender, receiver) = sync_channel(MAX_TASKS);
    (Executor { receiver }, Spawner { sender })
}

pub fn spawn_executor_thread() -> (JoinHandle<()>, Spawner) {
    let (executor, spawner) = build_runtime();

    let handle = thread::spawn(move || {
        while let Ok(task) = executor.receiver.recv() {
            let mut future_slot = task.future.lock().unwrap();

            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let mut context = Context::from_waker(&waker);

                // call poll
                let result = future.as_mut().poll(&mut context);
                match result {
                    Poll::Ready(_) => println!("We finished a task! Yippee!!"),
                    Poll::Pending => {
                        *future_slot = Some(future);
                    }
                }
            }
        }
    });

    (handle, spawner)
}
