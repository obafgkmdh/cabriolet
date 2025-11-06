use std::sync::{Arc, Mutex, mpsc::SyncSender};

use futures::future::{BoxFuture, FutureExt};

pub struct Task {
    pub future: Mutex<Option<BoxFuture<'static, ()>>>,

    pub sender: SyncSender<Arc<Task>>,
}

pub struct Spawner {
    pub sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Task {
            future: Mutex::new(Some(future)),
            sender: self.sender.clone(),
        };

        self.sender.try_send(Arc::new(task)).unwrap();
    }
}
