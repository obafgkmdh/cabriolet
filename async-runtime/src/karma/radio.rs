use crate::karma::{InputOrOutput, Karma, Peripheral, PeripheralMsg};

use crossbeam::channel::{Receiver, Sender, select, unbounded};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum RadioState {
    NotInit,
    Receive,
    Transmit,
    SendInProgress,
}

#[derive(Clone, Debug)]
pub enum RadioInputMsg {
    Init,
    StateTransmit,
    StateReceive,
    Send(Vec<u8>),
}

#[derive(Clone, Debug)]
pub enum RadioOutputMsg {
    DataReceived(Vec<u8>),
    InitDone,
    SendDone,
}

impl PeripheralMsg<RadioState> for RadioInputMsg {
    fn required_initial_state(&self) -> RadioState {
        match self {
            RadioInputMsg::Init => RadioState::NotInit,
            RadioInputMsg::StateTransmit => RadioState::Receive,
            RadioInputMsg::StateReceive => RadioState::Transmit,
            RadioInputMsg::Send(_) => RadioState::Transmit,
        }
    }

    fn resulting_state(&self) -> RadioState {
        match self {
            RadioInputMsg::Init => RadioState::Receive,
            RadioInputMsg::StateTransmit => RadioState::Transmit,
            RadioInputMsg::StateReceive => RadioState::Receive,
            RadioInputMsg::Send(_) => RadioState::SendInProgress,
        }
    }
}

impl PeripheralMsg<RadioState> for RadioOutputMsg {
    fn required_initial_state(&self) -> RadioState {
        match self {
            RadioOutputMsg::SendDone => RadioState::SendInProgress,
            RadioOutputMsg::DataReceived(_) => RadioState::Receive,
            RadioOutputMsg::InitDone => RadioState::NotInit,
        }
    }

    fn resulting_state(&self) -> RadioState {
        match self {
            RadioOutputMsg::SendDone => RadioState::Transmit,
            RadioOutputMsg::DataReceived(_) => RadioState::Receive,
            RadioOutputMsg::InitDone => RadioState::Receive,
        }
    }
}

#[derive(Clone)]
pub struct Radio {
    id: u64,

    // Shared memory region w/ radio "hardware"
    current_state: Arc<Mutex<RadioState>>,
    // "Interrupt" sender
    wakers: Arc<Mutex<Vec<Waker>>>,

    // Used for power cycles
    power_cycle_sender: Sender<()>,

    // Queue for sending commands from CPU to radio
    command_sender: Sender<RadioInputMsg>,
    // Interrupt queue for receiving results from radio on CPU
    interrupt_receiver: Receiver<RadioOutputMsg>,
}

impl Peripheral<RadioState> for Radio {
    type InputMsg = RadioInputMsg;
    type OutputMsg = RadioOutputMsg;

    fn get_id(&self) -> u64 {
        self.id
    }

    fn get_current_state(&self) -> RadioState {
        *self.current_state.lock().unwrap()
    }

    fn power_cycle(&mut self) {
        // Send a power cycle signal to the hw
        self.power_cycle_sender.send(()).unwrap();
    }
}

pub struct RadioFuture {
    wakers: Arc<Mutex<Vec<Waker>>>,
    receiver: Receiver<RadioOutputMsg>,
    support_queue: Arc<Mutex<VecDeque<InputOrOutput<RadioInputMsg, RadioOutputMsg>>>>,

    orig_arg: RadioFutureCreateArg,
}

impl Future for RadioFuture {
    type Output = Option<RadioOutputMsg>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.orig_arg {
            RadioFutureCreateArg::InputMsg(
                RadioInputMsg::StateTransmit | RadioInputMsg::StateReceive,
            ) => return Poll::Ready(None),
            _ => (),
        }

        // Try to receive the message
        while let Ok(msg) = self.receiver.try_recv() {
            // If we see a message that matches what we're waiting for,
            // return Ready
            match (&self.orig_arg, &msg) {
                (RadioFutureCreateArg::InputMsg(RadioInputMsg::Init), RadioOutputMsg::InitDone)
                | (
                    RadioFutureCreateArg::InputMsg(RadioInputMsg::Send(_)),
                    RadioOutputMsg::SendDone,
                )
                | (RadioFutureCreateArg::AwaitReceive, RadioOutputMsg::DataReceived(_)) => {
                    self.push_to_support_queue(InputOrOutput::Output(msg.clone()));
                    return Poll::Ready(Some(msg));
                }
                _ => (),
            }
        }

        // If we didn't see anything right now, set waker and remain pending
        let mut wakers = self.wakers.lock().unwrap();
        wakers.push(cx.waker().clone());
        Poll::Pending
    }
}

#[derive(Clone)]
pub enum RadioFutureCreateArg {
    InputMsg(RadioInputMsg),
    AwaitReceive,
}

impl RadioFuture {
    pub fn push_to_support_queue(&mut self, event: InputOrOutput<RadioInputMsg, RadioOutputMsg>) {
        // Ignore any inputs or outputs that don't affect the state machine
        let has_sm_effect = match &event {
            InputOrOutput::Input(input)
                if input.required_initial_state() == input.resulting_state() =>
            {
                false
            }
            InputOrOutput::Output(output)
                if output.required_initial_state() == output.resulting_state() =>
            {
                false
            }
            _ => true,
        };
        if has_sm_effect {
            self.support_queue.lock().unwrap().push_back(event);
        }

        println!(
            "Current support queue: {:?}",
            *self.support_queue.lock().unwrap()
        );
    }

    pub fn new(karma: &mut Karma<Radio, RadioState>, arg: RadioFutureCreateArg) -> Self {
        let radio = &mut karma.peripheral;

        let mut ret = Self {
            wakers: radio.wakers.clone(),
            receiver: radio.interrupt_receiver.clone(),
            orig_arg: arg.clone(),
            support_queue: karma.support_queue.clone(),
        };

        match arg {
            RadioFutureCreateArg::InputMsg(input) => {
                // Send the message to the radio
                radio.command_sender.send(input.clone()).unwrap();

                // Update the support queue
                ret.push_to_support_queue(InputOrOutput::Input(input));
            }
            RadioFutureCreateArg::AwaitReceive => (),
        }

        ret
    }
}

impl Radio {
    pub fn new(id: u64) -> Self {
        // Radio starts in RadioState::NotInit
        let state = Arc::new(Mutex::new(RadioState::NotInit));

        let (command_sender, command_receiver) = unbounded();
        let (interrupt_sender, interrupt_receiver) = unbounded();

        let (data_gen_sender, data_gen_receiver) = unbounded();

        let (power_cycle_sender, power_cycle_receiver) = unbounded();

        let wakers = Arc::new(Mutex::new(vec![]));

        // Spawn the radio backend thread
        let hw_state = state.clone();
        let hw_wakers = wakers.clone();
        thread::spawn(|| {
            radio_backend(
                hw_state,
                hw_wakers,
                command_receiver,
                interrupt_sender,
                data_gen_receiver,
                power_cycle_receiver,
            );
        });

        // Spawn the thread that receives data
        thread::spawn(|| {
            radio_data_generator(data_gen_sender);
        });

        Radio {
            id,
            current_state: state,
            command_sender,
            interrupt_receiver,
            wakers,
            power_cycle_sender,
        }
    }
}

// The radio "hardware" logic
fn radio_backend(
    state: Arc<Mutex<RadioState>>,
    wakers: Arc<Mutex<Vec<Waker>>>,
    command_receiver: Receiver<RadioInputMsg>,
    interrupt_sender: Sender<RadioOutputMsg>,
    data_gen_receiver: Receiver<Vec<u8>>,
    power_cycle_receiver: Receiver<()>,
) {
    loop {
        let prev_state = *state.lock().unwrap();

        select! {
            // Power cycle signal: kill this "hardware" (thread)
            recv(power_cycle_receiver) -> data => {
                if data.is_err() {
                    println!("Received power cycle receiver error");
                    return;
                }

                println!("Radio received power-cycle signal; resetting");

                *state.lock().unwrap() = RadioState::NotInit;
            }
            // Receive some data over the radio
            recv(data_gen_receiver) -> data => {
                let Ok(data) = data else {
                    println!("Received data gen receiver error");
                    return;
                };

                println!("Radio hardware received data: {:?}", data);

                match prev_state {
                    RadioState::Receive => {
                        println!(" -> forwarding to CPU...");
                        interrupt_sender.send(RadioOutputMsg::DataReceived(data)).unwrap();

                        // TODO: remove wakers afterwards
                        let wakers = wakers.lock().unwrap();
                        for waker in wakers.iter() {
                            waker.wake_by_ref();
                        }
                    },
                    _ => println!(" -> not in RadioState::Receive! ignoring..."),
                }
            }
            // Receive a command from the CPU
            recv(command_receiver) -> msg => {
                let Ok(msg) = msg else {
                    println!("Received command receiver error");
                    return;
                };

                println!("Radio hardware received message: {:?}", msg);

                match msg {
                    RadioInputMsg::Init => {
                        assert!(prev_state == RadioState::NotInit);

                        let mut state = state.lock().unwrap();
                        *state = RadioState::Receive;

                        interrupt_sender.send(RadioOutputMsg::InitDone).unwrap();

                        // TODO: remove wakers afterwards
                        let wakers = wakers.lock().unwrap();
                        for waker in wakers.iter() {
                            waker.wake_by_ref();
                        }
                    },
                    RadioInputMsg::StateTransmit => {
                        assert!(prev_state == RadioState::Receive);

                        let mut state = state.lock().unwrap();
                        *state = RadioState::Transmit;
                    },
                    RadioInputMsg::StateReceive => {
                        assert!(prev_state == RadioState::Transmit);

                        let mut state = state.lock().unwrap();
                        *state = RadioState::Receive;
                    },
                    RadioInputMsg::Send(data) => {
                        assert!(prev_state == RadioState::Transmit);

                        // Enter SendInProgress state
                        {
                            let mut state = state.lock().unwrap();
                            *state = RadioState::SendInProgress;
                        }

                        // Simulated rate of 1 byte / 0.5 sec
                        let time = Duration::from_millis(data.len() as u64 * 500);
                        thread::sleep(time);

                        // Return to Transmit state
                        {
                            let mut state = state.lock().unwrap();
                            *state = RadioState::Transmit;
                        }

                        // Send SendDone message
                        interrupt_sender.send(RadioOutputMsg::SendDone).unwrap();
                        // TODO: remove wakers afterwards
                        let wakers = wakers.lock().unwrap();
                        for waker in wakers.iter() {
                            waker.wake_by_ref();
                        }
                    },
                }
            }
        }
    }
}

// The "background" that generates data for the simulated radio
fn radio_data_generator(msg_sender: Sender<Vec<u8>>) {
    loop {
        let mut rng = SmallRng::from_os_rng();

        let secs = rng.random_range(5..15);
        let duration = Duration::from_secs(secs);

        thread::sleep(duration);

        // Generate random input
        let data: Vec<u8> = rng.random_iter().take(10).collect();

        // Send it (as radio waves, say), to the radio hardware simulation
        msg_sender.send(data).unwrap();
    }
}
