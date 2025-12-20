// S: State type enum

use std::{collections::VecDeque, marker::PhantomData};

pub trait PeripheralMsg<S> {
    // TODO: should these return Option, with None
    // indicating that the message does not indicate a state
    // change?
    fn required_initial_state(&self) -> S;
    fn resulting_state(&self) -> S;
}

pub trait Peripheral<S> {
    type InputMsg: PeripheralMsg<S>;
    type OutputMsg: PeripheralMsg<S>;

    fn get_id(&self) -> u64;

    fn get_current_state(&self) -> S;

    fn send_msg(&mut self, msg: Self::InputMsg);
}

enum InputOrOutput<I, O> {
    Input(I),
    Output(O),
}

pub struct Karma<P, S>
where
    P: Peripheral<S>,
{
    peripheral: P,
    support_queue: VecDeque<InputOrOutput<P::InputMsg, P::OutputMsg>>,

    _pd: PhantomData<S>,
}

impl<P, S> Karma<P, S> where P: Peripheral<S> {
    pub async fn replay_support_queue(&mut self) {
    }
}

// Simulated peripherals
pub mod radio;
