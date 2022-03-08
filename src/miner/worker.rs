use crate::network::message::Message;
use crate::network::server::Handle as ServerHandle;
use crate::types::block::Block;
use crate::types::hash::Hashable;
use crate::Blockchain;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Worker {
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) -> Self {
        Self {
            server: server.clone(),
            finished_block_chan,
            blockchain: Arc::clone(blockchain),
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("miner-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let _block = self
                .finished_block_chan
                .recv()
                .expect("Receive finished block error");
            // TODO for student: insert this finished block to blockchain, and broadcast this block hash
            let mut blockchain_with_lock = self.blockchain.lock().unwrap();
            blockchain_with_lock.insert(&_block);

            self.server
                .broadcast(Message::NewBlockHashes(vec![_block.hash()]));
            //std::mem::drop(blockchain_with_lock);
        }
    }
}
