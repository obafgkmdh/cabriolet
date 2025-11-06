use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

use rand::{Rng, SeedableRng, rngs::SmallRng};

type Temperature = f64;

struct SharedState {
    future_exists: bool,
    buffer: Vec<Temperature>,
    waker: Option<Waker>,
}

pub struct TemperatureSensorFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl Future for TemperatureSensorFuture {
    type Output = Vec<Temperature>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        assert!(shared_state.future_exists);

        if shared_state.buffer.len() != 0 {
            let buffer = shared_state.buffer.clone();
            shared_state.buffer.clear();
            shared_state.waker = None;
            shared_state.future_exists = false;
            Poll::Ready(buffer)
        } else {
            shared_state.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}

pub struct TemperatureSensor {
    shared_state: Arc<Mutex<SharedState>>,
}

impl TemperatureSensor {
    pub fn read(&mut self) -> Option<TemperatureSensorFuture> {
        let mut handle = self.shared_state.lock().unwrap();

        if handle.future_exists {
            return None;
        }

        handle.future_exists = true;
        assert!(handle.waker.is_none());

        Some(TemperatureSensorFuture {
            shared_state: self.shared_state.clone(),
        })
    }

    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            future_exists: false,
            buffer: vec![],
            waker: None,
        }));

        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            let mut rng = SmallRng::from_os_rng();

            loop {
                let duration = Duration::from_secs(rng.random_range(1..5));

                thread::sleep(duration);

                let temp: f64 = rng.random_range(0.0..100.0);
                // send the temperature
                let mut handle = thread_shared_state.lock().unwrap();
                handle.buffer.push(temp);

                // call the waker
                if let Some(waker) = &handle.waker {
                    waker.wake_by_ref();
                }
            }
        });

        Self {
            shared_state,
        }
    }
}
