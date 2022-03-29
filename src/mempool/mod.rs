use crate::types::hash::Hashable;
use crate::types::hash::H256;
use crate::types::transaction::SignedTransaction;
use crate::types::transaction::TxIn;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Mempool {
    pub tx_evidence: HashSet<H256>,
    pub tx_map: HashMap<H256, SignedTransaction>,
    pub spent_tx_in: HashMap<(H256, u8), H256>, // for double spend prevention (tx) (pre_tx, index): cur_tx_hash
}
impl Mempool {
    pub fn new() -> Self {
        Mempool {
            tx_evidence: HashSet::new(),
            tx_map: HashMap::new(),
            spent_tx_in: HashMap::new(),
        }
    }

    pub fn insert(&mut self, tx: &SignedTransaction) -> bool {
        let tx_hash: H256 = tx.clone().hash();
        // no duplicate tx
        if self.tx_evidence.contains(&tx_hash) {
            println!("mempool insert fail, duplicate tx");
            return false;
        }
        // prevent tx_input: [TxIn{0001,0}, TxIn{0001,0}]
        let mut tx_in_temp_set: HashSet<(H256, u8)> = HashSet::new();
        for tx_in in tx.clone().transaction.tx_input {
            if tx_in_temp_set.contains(&(tx_in.previous_output, tx_in.index)) {
                println!("mempool insert fail, double spend in same tx");
                return false;
            } else {
                tx_in_temp_set.insert((tx_in.previous_output, tx_in.index));
            }
        }

        // prevent double spend
        // Tx1 { tx_input: [TxIn{0001,0}] tx_out: [老王50]}
        // Tx2 { tx_input: [TxIn{0001,0}] tx_out: [老李50]}
        for tx_in in tx.clone().transaction.tx_input {
            if self
                .spent_tx_in
                .contains_key(&(tx_in.previous_output, tx_in.index))
            {
                println!("mempool insert fail, tx_in already in spent_tx_in");
                println!("{:?}", self.spent_tx_in);
                return false;
            }
        }
        //insert into spent_tx_in, mark as spent
        for tx_in in tx.clone().transaction.tx_input {
            self.spent_tx_in
                .insert((tx_in.previous_output, tx_in.index), tx.hash());
        }
        println!("{:?}", tx.transaction);
        self.tx_map.insert(tx_hash, tx.clone());
        self.tx_evidence.insert(tx_hash);
        true
    }

    pub fn remove(&mut self, transaction: &SignedTransaction) {
        let tx_hash: H256 = transaction.hash();
        if self.tx_map.contains_key(&tx_hash) {
            self.tx_map.remove(&tx_hash);
            // self.tx_evidence.remove(&tx_hash);
        }
    }
    // remove using tx hash
    pub fn remove_with_hash(&mut self, tx_hash: H256) {
        if self.tx_map.contains_key(&tx_hash) {
            self.tx_map.remove(&tx_hash);
            // self.tx_evidence.remove(&tx_hash);
        }
    }
}
