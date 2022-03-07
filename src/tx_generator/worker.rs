use crate::mempool::{self, Mempool};
use crate::network::message::Message;
use crate::network::server::Handle as ServerHandle;
use crate::types::hash::Hashable;
use crate::types::transaction::{self, SignedTransaction};
use crossbeam::channel::Receiver;
use log::info;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Worker {
    server: ServerHandle,
    tx_receiver: Receiver<SignedTransaction>,
    tx_mempool: Arc<Mutex<Mempool>>,
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        tx_receiver: Receiver<SignedTransaction>,
        tx_mempool: &Arc<Mutex<Mempool>>,
    ) -> Self {
        Self {
            server: server.clone(),
            tx_receiver,
            tx_mempool: Arc::clone(tx_mempool),
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("tx-generator-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Tx-generator initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let _transaction = self
                .tx_receiver
                .recv()
                .expect("Receive finished block error");
            let mut mempool_with_lock = self.tx_mempool.lock().unwrap();
            mempool_with_lock.insert(&_transaction);
            self.server
                .broadcast(Message::NewTransactionHashes(vec![_transaction.hash()]));
            std::mem::drop(mempool_with_lock);
        }
    }
}
