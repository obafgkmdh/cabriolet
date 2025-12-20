use crate::karma::{Peripheral, PeripheralMsg};

use crossbeam::channel::{Receiver, Sender, select, unbounded};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::{
    sync::{Arc, Mutex}, task::Waker, thread, time::Duration
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RadioState {
    NotInit,
    Receive,
    Transmit,
    SendInProgress,
}

#[derive(Clone, Debug)]
enum RadioInputMsg {
    Init,
    StateTransmit,
    StateReceive,
    Send(Vec<u8>),
}

#[derive(Clone, Debug)]
enum RadioOutputMsg {
    DataReceived(Vec<u8>),
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
        }
    }

    fn resulting_state(&self) -> RadioState {
        match self {
            RadioOutputMsg::SendDone => RadioState::Transmit,
            RadioOutputMsg::DataReceived(_) => RadioState::Receive,
        }
    }
}

pub struct Radio {
    id: u64,

    // Shared memory region w/ radio "hardware"
    current_state: Arc<Mutex<RadioState>>,
    // "Interrupt" sender
    waker: Arc<Mutex<Option<Waker>>>,

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

    fn send_msg(&mut self, msg: Self::InputMsg) {
        *self.waker.lock().unwrap() = None;

        self.command_sender.send(msg).unwrap();
    }
}

impl Radio {
    pub fn new(id: u64) -> Self {
        // Radio starts in RadioState::NotInit
        let state = Arc::new(Mutex::new(RadioState::NotInit));

        let (command_sender, command_receiver) = unbounded();
        let (interrupt_sender, interrupt_receiver) = unbounded();

        let (data_gen_sender, data_gen_receiver) = unbounded();

        // Spawn the radio backend thread
        let hw_state = state.clone();
        thread::spawn(|| {
            radio_backend(
                hw_state,
                command_receiver,
                interrupt_sender,
                data_gen_receiver,
            );
        });

        // Spawn the thread that receives data
        thread::spawn(|| {
            radio_data_generator(data_gen_sender);
        });

        let waker = Arc::new(Mutex::new(None));

        Radio {
            id,
            current_state: state,
            command_sender,
            interrupt_receiver,
            waker,
        }
    }
}

// The radio "hardware" logic
fn radio_backend(
    state: Arc<Mutex<RadioState>>,
    command_receiver: Receiver<RadioInputMsg>,
    interrupt_sender: Sender<RadioOutputMsg>,
    data_gen_receiver: Receiver<Vec<u8>>,
) {
    loop {
        let prev_state = *state.lock().unwrap();

        select! {
            // Receive some data over the radio
            recv(data_gen_receiver) -> data => {
                let data = data.unwrap();

                println!("Radio hardware received data: {:?}", data);

                match prev_state {
                    RadioState::Receive => {
                        println!(" -> forwarding to CPU...");
                        interrupt_sender.send(RadioOutputMsg::DataReceived(data)).unwrap();
                    },
                    _ => println!(" -> not in RadioState::Receive! ignoring..."),
                }
            }
            // Receive a command from the CPU
            recv(command_receiver) -> msg => {
                let msg = msg.unwrap();

                println!("Radio hardware received message: {:?}", msg);

                match msg {
                    RadioInputMsg::Init => {
                        assert!(prev_state == RadioState::NotInit);

                        let mut state = state.lock().unwrap();
                        *state = RadioState::Receive;
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
                        let mut statep = state.lock().unwrap();
                        *statep = RadioState::SendInProgress;

                        // Simulated rate of 1 byte / 0.5 sec
                        let time = Duration::from_millis(data.len() as u64 * 500);
                        thread::sleep(time);

                        // Return to Transmit state
                        let mut statep = state.lock().unwrap();
                        *statep = RadioState::Transmit;
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
