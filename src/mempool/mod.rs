use crate::types::hash::Hashable;
use crate::types::hash::H256;
use crate::types::transaction::SignedTransaction;
use std::collections::{HashMap, HashSet};
pub struct Mempool {
    pub tx_evidence: HashSet<H256>,
    pub tx_map: HashMap<H256, SignedTransaction>,
    pub spent_tx_in: HashSet<(H256, u8)>, // for double spend prevention
}
impl Mempool {
    pub fn new() -> Self {
        Mempool {
            tx_evidence: HashSet::new(),
            tx_map: HashMap::new(),
            spent_tx_in: HashSet::new(),
        }
    }

    pub fn insert(&mut self, tx: &SignedTransaction) -> bool {
        let tx_hash: H256 = tx.clone().hash();
        // no duplicate tx
        if self.tx_evidence.contains(&tx_hash) {
            return false;
        }
        // prevent tx_input: [TxIn{0001,0}, TxIn{0001,0}]
        let mut tx_in_temp_map: HashSet<(H256, u8)> = HashSet::new();
        for tx_in in tx.clone().transaction.tx_input {
            if tx_in_temp_map.contains(&(tx_in.previous_output, tx_in.index)) {
                return false;
            } else {
                tx_in_temp_map.insert((tx_in.previous_output, tx_in.index));
            }
        }

        // prevent double spend
        // Tx1 { tx_input: [TxIn{0001,0}] tx_out: [老王50]}
        // Tx2 { tx_input: [TxIn{0001,0}] tx_out: [老李50]}
        for tx_in in tx.clone().transaction.tx_input {
            if self
                .spent_tx_in
                .contains(&(tx_in.previous_output, tx_in.index))
            {
                return false;
            }
        }
        //insert into spent_tx_in, mark as spent
        for tx_in in tx.clone().transaction.tx_input {
            self.spent_tx_in
                .insert((tx_in.previous_output, tx_in.index));
        }

        self.tx_map.insert(tx_hash, tx.clone());
        self.tx_evidence.insert(tx_hash);
        true
    }
    // TODO: when block in chain, update spent_tx_in
    pub fn update_spent_tx_in() {}

    pub fn remove(&mut self, transaction: &SignedTransaction) {
        let tx_hash: H256 = transaction.hash();
        if self.tx_map.contains_key(&tx_hash) {
            self.tx_map.remove(&tx_hash);
            // self.tx_evidence.remove(&tx_hash);
        }
    }
}
