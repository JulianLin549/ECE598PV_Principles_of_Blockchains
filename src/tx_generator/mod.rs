pub mod worker;

use crate::mempool::Mempool;
use crate::types::address::Address;
use crate::types::hash::{Hashable, H256};

use crate::types::state;
use crate::types::transaction::{
    generate_random_hash, sign, SignedTransaction, Transaction, TxIn, TxOut,
};
use crate::State;
use rand::distributions::{Distribution, Uniform}; // 0.6.5
use ring::signature::{self, Ed25519KeyPair, KeyPair};

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;

use std::collections::HashMap;
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
    state: Arc<Mutex<State>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    tx_mempool: &Arc<Mutex<Mempool>>,
    state: &Arc<Mutex<State>>,
) -> (Context, Handle, Receiver<SignedTransaction>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (tx_sender, tx_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        tx_mempool: Arc::clone(tx_mempool),
        tx_sender: tx_sender,
        state: Arc::clone(state),
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
        // tx recipient will be selected randomly for these 10 addresses
        let vec_vec: Vec<[u8; 32]> = vec![
            *b"00000000000000000000000000000000",
            *b"00000000000000000000000000000001",
            *b"00000000000000000000000000000002",
            *b"00000000000000000000000000000003",
            *b"00000000000000000000000000000004",
            *b"00000000000000000000000000000005",
            *b"00000000000000000000000000000006",
            *b"00000000000000000000000000000007",
            *b"00000000000000000000000000000008",
            *b"00000000000000000000000000000009",
            *b"00000000000000000000000000000010",
        ];

        let mut key_vec: Vec<Ed25519KeyPair> = Vec::new();
        for i in 0..vec_vec.len() {
            let key = signature::Ed25519KeyPair::from_seed_unchecked(&vec_vec[i]).unwrap();
            key_vec.push(key);
        }
        let mut address_vec: Vec<Address> = Vec::new();
        for i in 0..vec_vec.len() {
            address_vec.push(Address::address_from_public_key(*key_vec[i].public_key()));
        }

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
            let state_with_lock = self.state.lock().unwrap();

            let new_tx =
                generate_random_signed_transaction(&key_vec, &address_vec, state_with_lock.clone());
            std::mem::drop(state_with_lock);
            let new_tx_hash = new_tx.hash();
            // 7. mempool.insert
            // send to worker, worker will put into mempool and broadcast
            // don't allow duplicate tx
            if !mempool_with_lock.tx_evidence.contains(&new_tx_hash) {
                self.tx_sender
                    .send(new_tx.clone())
                    .expect("Send new tx error");
            }
            std::mem::drop(mempool_with_lock);

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval * 300);
                }
            }
        }
    }
}

pub fn generate_random_signed_transaction(
    key_vec: &Vec<Ed25519KeyPair>,
    address_vec: &Vec<Address>,
    state: State,
) -> SignedTransaction {
    // 1. select random utxo from state
    // first change hashmap key to vector
    let state_keys = state
        .utxo
        .clone()
        .keys()
        .cloned()
        .collect::<Vec<(H256, u8)>>();
    let rand_utxo_key = state_keys[random_select(state_keys.len())];
    let rand_utxo_value = state.utxo[&rand_utxo_key];

    // 2. see who is the owner of the utxo
    let owner_address = rand_utxo_value.1;
    let mut new_tx_ins: Vec<TxIn> = Vec::new();
    let mut new_tx_outs: Vec<TxOut> = Vec::new();
    // 3. collect utxos belongs to the owner, if amount more than 1000, break
    let mut total_amount = 0;
    for key in state.utxo.keys() {
        if owner_address == state.utxo[key].1 {
            total_amount += state.utxo[key].0;
            let new_tx_in = TxIn {
                previous_output: key.0,
                index: key.1,
            };
            new_tx_ins.push(new_tx_in);
            if total_amount > 1000 {
                break;
            }
        }
    }
    // 4. select random recipient not equal to owner (first delete from address_vec, then random choose)
    // also we can get owner's key during the process. key is for signing tx
    let mut idx = 0;
    for addr in address_vec {
        if *addr == owner_address {
            break;
        }
        idx += 1;
    }

    let owner_key = &key_vec[idx];

    let mut recipient_addresses: Vec<Address> = Vec::new();
    for addr in address_vec {
        if *addr != owner_address {
            recipient_addresses.push(*addr);
        }
    }

    let recipient_address = recipient_addresses[random_select(recipient_addresses.len())];

    // 5. send 1/4 of the amount to recipient, 3/4 to owner if 1/2 amount is greater than 200
    // 6. else send all amount to recipient.
    let amount_to_send = if total_amount / 4 > 0 {
        total_amount / 4
    } else {
        total_amount
    };
    new_tx_outs.push(TxOut {
        recipient_addr: recipient_address,
        value: amount_to_send,
    });
    // if there is remaining amount, send back to owner
    let amount_remain = total_amount - amount_to_send;
    if amount_remain > 0 {
        new_tx_outs.push(TxOut {
            recipient_addr: owner_address,
            value: amount_remain,
        });
    }
    let new_tx = Transaction {
        tx_input: new_tx_ins,
        tx_output: new_tx_outs,
    };
    let owner_pub_key = owner_key.public_key().as_ref().to_vec();
    let new_signature = sign(&new_tx, &owner_key).as_ref().to_vec();

    SignedTransaction {
        transaction: new_tx,
        signature: new_signature,
        public_key: owner_pub_key,
    }
}
pub fn random_select(vec_len: usize) -> usize {
    let step = Uniform::new(0, vec_len);
    let mut rng = rand::thread_rng();
    step.sample(&mut rng)
}
