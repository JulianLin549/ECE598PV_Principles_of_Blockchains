pub mod worker;

use log::info;

use crate::mempool::Mempool;
use crate::types::block::Block;
use crate::types::block::Content;
use crate::types::block::Header;
use crate::types::hash::Hashable;
use crate::types::merkle::MerkleTree;
use crate::types::state::BlockToStateMap;
use crate::types::state::State;
use crate::Blockchain;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::time::SystemTime;

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update,     // update the block in mining, it may due to new blockchain tip or new transaction
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
    finished_block_chan: Sender<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_mempool: Arc<Mutex<Mempool>>,
    state: Arc<Mutex<State>>,
    bts_map: Arc<Mutex<BlockToStateMap>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    blockchain: &Arc<Mutex<Blockchain>>,
    tx_mempool: &Arc<Mutex<Mempool>>,
    state: &Arc<Mutex<State>>,
    bts_map: &Arc<Mutex<BlockToStateMap>>,
) -> (Context, Handle, Receiver<Block>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_block_sender, finished_block_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_block_chan: finished_block_sender,
        blockchain: Arc::clone(blockchain),
        tx_mempool: Arc::clone(tx_mempool),
        state: Arc::clone(state),
        bts_map: Arc::clone(bts_map),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_block_receiver)
}

#[cfg(any(test, test_utilities))]
fn test_new() -> (Context, Handle, Receiver<Block>) {
    let blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(blockchain));
    let tx_mempool = Mempool::new();
    let tx_mempool = Arc::new(Mutex::new(tx_mempool));
    let state = State::new();
    let state = Arc::new(Mutex::new(state));
    let bts_map = BlockToStateMap::new();
    let bts_map = Arc::new(Mutex::new(bts_map));
    new(&blockchain, &tx_mempool, &state, &bts_map)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn miner_loop(&mut self) {
        // main mining loop
        // block: 50 txs
        let block_tx_num_limit = 50;
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            println!("Miner shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            println!("Miner starting in continuous mode with lambda {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Miner shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Miner starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }
            let mut blockchain_with_lock = self.blockchain.lock().unwrap();
            let mut mempool_with_lock = self.tx_mempool.lock().unwrap();
            let mut state_with_lock = self.state.lock().unwrap();
            let mut bts_map_with_lock = self.bts_map.lock().unwrap();
            let parent_hash = blockchain_with_lock.tip;
            // mining: create random nonce
            let mut rng = rand::thread_rng();
            let new_nonce: u32 = rng.gen();

            let difficulty = blockchain_with_lock.blockchain[&parent_hash]
                .header
                .difficulty;
            let timestamp: u128 = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let mut transactions = Vec::new();
            let mut block_tx_num = 0; //keep track of how many tx are selected

            // if mempool no enough tx to process
            // it has to be here so that there will be at least one tx in a block
            if mempool_with_lock.tx_map.len() < 1 {
                continue;
            }
            // select txs from mempool
            for (_tx_key, tx) in mempool_with_lock.tx_map.iter() {
                if block_tx_num + 1 > block_tx_num_limit {
                    break;
                }
                transactions.push(tx.clone());
                block_tx_num += 1;
            }

            //create merkle root
            let merkle_tree = MerkleTree::new(transactions.as_ref());
            let merkle_root = merkle_tree.root();

            // create empty content
            let content: Content = Content { data: transactions };

            let header = Header {
                parent: parent_hash,
                nonce: new_nonce,
                difficulty: difficulty,
                timestamp: timestamp,
                merkle_root: merkle_root,
            };
            // create block to be mined
            let block = Block {
                header: header,
                content: content,
            };

            // Check whether the proof-of-work hash puzzle is solved or not.
            if block.hash() <= difficulty {
                println!("Successfully mined a block {:?}", block);

                // remove used tx in mempool, update state
                for tx in block.clone().content.data {
                    mempool_with_lock.remove(&tx);
                    state_with_lock.update(&tx);

                    // remove any double spend tx_in in mempool found in block,
                    // add tx_in in block to spent_tx_in
                    for tx_in in tx.clone().transaction.tx_input {
                        if mempool_with_lock
                            .spent_tx_in
                            .contains_key(&(tx_in.previous_output, tx_in.index))
                        {
                            // remove tx in mempool using hash
                            let tx_hash = mempool_with_lock.spent_tx_in
                                [&(tx_in.previous_output, tx_in.index)];
                            mempool_with_lock.remove_with_hash(tx_hash);
                        }
                        // mark tx_in as spent in spent_tx_in
                        mempool_with_lock
                            .spent_tx_in
                            .insert((tx_in.previous_output, tx_in.index), tx.hash());
                    }
                }
                //insert into block-to-state-map
                bts_map_with_lock.insert(block.hash(), state_with_lock.clone());
                //insert into blockchain
                blockchain_with_lock.insert(&block);
                self.finished_block_chan
                    .send(block.clone())
                    .expect("Send finished block error");
            }
            std::mem::drop(blockchain_with_lock);
            std::mem::drop(mempool_with_lock);
            std::mem::drop(state_with_lock);
            std::mem::drop(bts_map_with_lock);

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use crate::types::hash::Hashable;
    use ntest::timeout;

    #[test]
    #[timeout(60000)]
    fn miner_three_block() {
        let (miner_ctx, miner_handle, finished_block_chan) = super::test_new();
        miner_ctx.start();
        miner_handle.start(0);
        let mut block_prev = finished_block_chan.recv().unwrap();
        for _ in 0..2 {
            let block_next = finished_block_chan.recv().unwrap();
            assert_eq!(block_prev.hash(), block_next.get_parent());
            block_prev = block_next;
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
