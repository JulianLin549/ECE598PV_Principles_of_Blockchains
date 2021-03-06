use crate::mempool::Mempool;
use crate::types::address::Address;
use crate::types::hash::{Hashable, H256};
use crate::types::transaction::SignedTransaction;
use ring::digest;
use ring::signature::{self, KeyPair};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

//////
/// State is for storing information about the current state,
/// the state contains utxo, which is a hashmap of the (previous_out, index): (amount, recipient) key value pair.
/// we initialize state, granting 100000 coin to "00000000000000000000000000000000"
/// when updating state, we remove previous used txin then add txout to the state.
/// the initial state does not belong to any transaction.
//////

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct State {
    //utxo
    //key: (previous_out, index)
    //value: (amount, recipient)
    pub utxo: HashMap<(H256, u8), (u64, Address)>,
}
impl State {
    // ICO
    pub fn new() -> Self {
        let mut utxo = HashMap::new();
        let bytes32 = [0u8; 32];
        let tx_hash: H256 = bytes32.into();
        let output_idx: u8 = 0;
        let value: u64 = 100000;

        let seed = *b"00000000000000000000000000000000";
        let key = signature::Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();

        let public_key = key.public_key();
        let pb_hash: H256 = digest::digest(&digest::SHA256, public_key.as_ref()).into();
        let recipient: Address = pb_hash.to_addr();
        let init_key = (tx_hash, output_idx);
        let init_val = (value, recipient);
        utxo.insert(init_key, init_val);

        println!(
            "ICO completed. {:?} coins are granted to {:?}",
            value, recipient
        );
        let state = State { utxo: utxo };
        println!("{:?}", state);
        return state;
    }

    pub fn update(&mut self, signed_tx: &SignedTransaction) {
        // println!("Before state update");
        // for (key, val) in self.utxo.iter() {
        //     println!("key: {:?}, val: {:?}", key, val);
        // }
        let tx = signed_tx.transaction.clone();
        let tx_inputs = tx.tx_input;
        let tx_outputs = tx.tx_output;
        for tx_in in tx_inputs {
            let key = (tx_in.previous_output, tx_in.index);
            self.utxo.remove(&key);
        }
        let mut idx = 0;
        for tx_out in tx_outputs {
            let tx_hash = signed_tx.hash();
            self.utxo
                .insert((tx_hash, idx), (tx_out.value, tx_out.recipient_addr));
            idx += 1;
        }
        // println!("After state update");
        // for (key, val) in self.utxo.iter() {
        //     println!("key: {:?}, val: {:?}", key, val);
        // }
    }
}
//////
/// BlockToStateMap keeps track of screenshot of state at particular block hash.
/// Which means that for each block, there is a related state. We use hashmap for fast retrieval.
//////
pub struct BlockToStateMap {
    pub bts_map: HashMap<H256, State>,
}
impl BlockToStateMap {
    pub fn new() -> Self {
        let bts_map = HashMap::new();
        BlockToStateMap { bts_map: bts_map }
    }
    pub fn insert(&mut self, block_hash: H256, state: State) {
        self.bts_map.insert(block_hash, state);
    }
}
