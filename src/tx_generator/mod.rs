pub mod worker;

use crate::mempool::Mempool;
use crate::network::message::Message;
use crate::network::server::Handle as ServerHandle;
use crate::types::address::Address;
use crate::types::hash::{Hashable, H256};
use rand::distributions::{Distribution, Uniform}; // 0.6.5

use crate::types::transaction::{
    generate_random_hash, sign, SignedTransaction, Transaction, TxIn, TxOut,
};
use ring::signature::{self, Ed25519KeyPair, KeyPair, Signature};

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;
use rand::seq::SliceRandom;
use rand::Rng;

use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time;
use std::{thread, vec};

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    tx_mempool: Arc<Mutex<Mempool>>,
    tx_sender: Sender<SignedTransaction>,
    // utxo_state: Arc<Mutex<UtxoState>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    tx_mempool: &Arc<Mutex<Mempool>>,
    // utxo_state: &Arc<Mutex<UtxoState>>,
) -> (Context, Handle, Receiver<SignedTransaction>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (tx_sender, tx_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        tx_mempool: Arc::clone(tx_mempool),
        tx_sender: tx_sender,
        // utxo_state: Arc::clone(utxo_state),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, tx_receiver)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, theta: u64) {
        self.control_chan.send(ControlSignal::Start(theta)).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("tx_generator".to_string())
            .spawn(move || {
                self.gen_loop();
            })
            .unwrap();
        info!("Generator initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                println!("Generator shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                println!("Generator starting in continuous mode with theta {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn gen_loop(&mut self) {
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Generator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            let mempool_with_lock = self.tx_mempool.lock().unwrap();

            let new_tx = generate_random_signed_transaction();
            let new_tx_hash = new_tx.hash();
            // send to worker, worker will put into mempool and broadcast
            // don't allow duplicate tx
            if !mempool_with_lock.tx_to_process.contains(&new_tx_hash) {
                self.tx_sender
                    .send(new_tx.clone())
                    .expect("Send new tx error");
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval * 100);
                }
            }
            std::mem::drop(mempool_with_lock);
        }
    }
}

pub fn generate_random_signed_transaction() -> SignedTransaction {
    let mut vec_vec: Vec<[u8; 85]> = Vec::new();
    vec_vec.push([
        48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 187, 131, 74, 161, 134, 11, 240,
        6, 188, 109, 18, 108, 124, 219, 167, 164, 215, 125, 168, 79, 204, 194, 232, 91, 58, 186,
        181, 230, 212, 78, 163, 28, 161, 35, 3, 33, 0, 233, 72, 146, 218, 220, 235, 17, 123, 202,
        112, 119, 63, 134, 105, 134, 71, 34, 185, 71, 193, 59, 66, 43, 137, 50, 194, 120, 234, 97,
        132, 235, 159,
    ]);
    vec_vec.push([
        48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 154, 186, 73, 239, 105, 129, 142,
        211, 156, 79, 213, 209, 229, 87, 22, 92, 113, 203, 244, 222, 244, 33, 199, 254, 130, 102,
        178, 65, 198, 67, 20, 132, 161, 35, 3, 33, 0, 161, 153, 171, 27, 96, 146, 25, 237, 5, 189,
        186, 116, 0, 24, 2, 8, 28, 143, 5, 119, 20, 47, 142, 186, 55, 234, 189, 167, 154, 15, 210,
        97,
    ]);
    vec_vec.push([
        48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 11, 212, 170, 1, 126, 8, 32, 58,
        40, 116, 165, 98, 48, 127, 67, 109, 86, 251, 249, 203, 244, 203, 1, 223, 248, 164, 176,
        195, 23, 17, 146, 8, 161, 35, 3, 33, 0, 206, 15, 234, 106, 58, 45, 177, 81, 0, 193, 13,
        113, 249, 55, 152, 151, 227, 224, 35, 185, 148, 49, 186, 234, 17, 106, 132, 216, 83, 196,
        127, 99,
    ]);
    vec_vec.push([
        48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 40, 29, 27, 179, 25, 183, 68,
        113, 252, 19, 20, 114, 160, 221, 228, 195, 253, 87, 245, 176, 226, 99, 249, 28, 87, 61,
        101, 129, 207, 87, 90, 195, 161, 35, 3, 33, 0, 254, 57, 159, 24, 159, 141, 184, 159, 58,
        86, 112, 217, 153, 215, 65, 7, 88, 14, 57, 80, 42, 33, 151, 211, 208, 52, 42, 208, 111,
        174, 223, 27,
    ]);
    vec_vec.push([
        48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 224, 231, 169, 219, 160, 221,
        218, 51, 189, 197, 202, 218, 24, 20, 166, 105, 31, 55, 241, 231, 5, 165, 51, 106, 174, 11,
        110, 84, 17, 115, 230, 56, 161, 35, 3, 33, 0, 127, 130, 60, 237, 224, 179, 64, 241, 25,
        174, 45, 64, 52, 179, 70, 249, 26, 49, 128, 103, 188, 201, 48, 55, 221, 154, 12, 83, 40,
        123, 3, 157,
    ]);
    let mut key_vec: Vec<Ed25519KeyPair> = Vec::new();
    for i in 0..vec_vec.len() {
        let key = signature::Ed25519KeyPair::from_pkcs8(vec_vec[i].as_ref().into()).unwrap();
        key_vec.push(key);
    }

    let mut address_vec: Vec<Address> = Vec::new();
    for i in 0..vec_vec.len() {
        address_vec.push(Address::address_from_public_key(*key_vec[i].public_key()));
    }

    //(address, signature, public_key)
    let step = Uniform::new(0, vec_vec.len());
    let mut rng = rand::thread_rng();
    let rand_choice = step.sample(&mut rng);

    let new_recipient = address_vec[rand_choice];
    let new_key = &key_vec[rand_choice];
    let new_public_key = new_key.public_key().as_ref().to_vec();

    let input = vec![TxIn {
        previous_output: generate_random_hash(),
        index: 0,
    }];
    let output = vec![TxOut {
        recipient_addr: new_recipient,
        value: 0,
    }];

    let new_tx = Transaction {
        tx_input: input,
        tx_output: output,
    };
    let new_signature = sign(&new_tx, &new_key).as_ref().to_vec();

    SignedTransaction {
        transaction: new_tx,
        signature: new_signature,
        public_key: new_public_key,
    }
}
