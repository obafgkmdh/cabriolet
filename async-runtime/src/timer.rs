use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread::{self, sleep},
    time::{Duration, Instant},
};

struct SharedState {
    result: Option<Instant>,
    waker: Option<Waker>,
}

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl Future for TimerFuture {
    type Output = Instant;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();

        if let Some(instant) = shared_state.result {
            Poll::Ready(instant)
        } else {
            shared_state.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}

impl TimerFuture {
    pub fn new(duration: Duration) -> Self {
        let shared_state = Mutex::new(SharedState {
            result: None,
            waker: None,
        });
        let shared_state_ref = Arc::new(shared_state);
        let my_shared_state_ref = shared_state_ref.clone();

        thread::spawn(move || {
            sleep(duration);
            let mut lock = shared_state_ref.lock().unwrap();

            let now = Instant::now();
            lock.result = Some(now);

            if let Some(waker) = &lock.waker {
                waker.wake_by_ref();
            }
        });

        Self {
            shared_state: my_shared_state_ref,
        }
    }
}
