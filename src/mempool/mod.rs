use crate::types::hash::Hashable;
use crate::types::hash::H256;
use crate::types::transaction::SignedTransaction;
use std::collections::{HashMap, HashSet};
pub struct Mempool {
    pub tx_evidence: HashSet<H256>,
    pub tx_map: HashMap<H256, SignedTransaction>,
}
impl Mempool {
    pub fn new() -> Self {
        Mempool {
            tx_evidence: HashSet::new(),
            tx_map: HashMap::new(),
        }
    }

    // TODO: tx check?
    pub fn insert(&mut self, transaction: &SignedTransaction) {
        let tx_hash: H256 = transaction.hash();
        // no duplicate tx
        if self.tx_evidence.contains(&tx_hash) {
            return;
        }
        self.tx_map.insert(tx_hash, transaction.clone());
        self.tx_evidence.insert(tx_hash);
    }

    pub fn remove(&mut self, transaction: &SignedTransaction) {
        let tx_hash: H256 = transaction.hash();
        if self.tx_map.contains_key(&tx_hash) {
            self.tx_map.remove(&tx_hash);
            // self.tx_evidence.remove(&tx_hash);
        }
    }
}
