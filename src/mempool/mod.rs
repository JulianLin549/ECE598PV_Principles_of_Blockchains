use crate::types::hash::Hashable;
use crate::types::hash::H256;
use crate::types::transaction::SignedTransaction;
use std::collections::{HashMap, HashSet};
pub struct Mempool {
    pub tx_to_process: HashSet<H256>,
    pub tx_map: HashMap<H256, SignedTransaction>,
}
impl Mempool {
    pub fn new() -> Self {
        Mempool {
            tx_to_process: HashSet::new(),
            tx_map: HashMap::new(),
        }
    }

    // mempool_with_lock.tx_map.insert(signed_tx_hash, signed_tx);
    // mempool_with_lock.tx_to_process.insert(signed_tx_hash, true);
    pub fn insert(&mut self, transaction: &SignedTransaction) {
        let tx_hash: H256 = transaction.hash();
        if self.tx_to_process.contains(&tx_hash) {
            return;
        }
        self.tx_map.insert(tx_hash, transaction.clone());
        self.tx_to_process.insert(tx_hash);
    }

    pub fn remove(&mut self, transaction: &SignedTransaction) {
        let tx_hash: H256 = transaction.hash();
        if self.tx_to_process.contains(&tx_hash) {
            self.tx_map.remove(&tx_hash);
            self.tx_to_process.remove(&tx_hash);
        }
    }
}
