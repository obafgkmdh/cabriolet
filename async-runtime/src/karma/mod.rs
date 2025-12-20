// S: State type enum

use std::{
    collections::VecDeque,
    fmt::Debug,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

pub trait PeripheralMsg<S> {
    // TODO: should these return Option, with None
    // indicating that the message does not indicate a state
    // change?
    fn required_initial_state(&self) -> S;
    fn resulting_state(&self) -> S;
}

pub trait Peripheral<S> {
    type InputMsg: PeripheralMsg<S> + Debug;
    type OutputMsg: PeripheralMsg<S> + Debug;

    fn get_id(&self) -> u64;

    fn get_current_state(&self) -> S;

    fn power_cycle(&mut self);
}

#[derive(Debug)]
enum InputOrOutput<I, O> {
    Input(I),
    Output(O),
}

#[derive(Clone)]
pub struct Karma<P, S>
where
    P: Peripheral<S>,
{
    peripheral: P,
    support_queue: Arc<Mutex<VecDeque<InputOrOutput<P::InputMsg, P::OutputMsg>>>>,

    _pd: PhantomData<S>,
}

impl<P, S> Karma<P, S>
where
    P: Peripheral<S>,
{
    pub fn new(peripheral: P) -> Self {
        Self {
            peripheral,
            support_queue: Arc::new(Mutex::new(VecDeque::new())),

            _pd: PhantomData,
        }
    }

    pub async fn replay_support_queue(&mut self) {
        for e in self.support_queue.lock().unwrap().iter() {
            // TODO
        }
    }
}

// Simulated peripherals
pub mod radio;
