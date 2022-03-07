use crate::types::address::generate_random_address;
use crate::types::address::Address;
use crate::types::hash::{Hashable, H256};
use rand::Rng;
use ring::digest;
use ring::signature::{self, Ed25519KeyPair, KeyPair, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub struct State {
    pub utxo: HashMap<(H256, u8), (u64, Address)>,
}
impl State {
    pub fn new() -> Self {
        use crate::types::key_pair;
        let mut utxo = HashMap::new();
        let bytes32 = [0u8; 32];
        let tx_hash: H256 = bytes32.into();
        let output_idx: u8 = 0;
        let value: u64 = 10000;
        let seed = [0u8; 32];
        let key = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
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
        State { utxo: utxo }
    }

    pub fn update(&mut self, transaction: &SignedTransaction) {
        println!("Before state update");
        for (key, val) in self.utxo.iter() {
            println!("key: {:?}, val: {:?}", key, val);
        }
        let tx = transaction.transaction.clone();
        let input = tx.tx_input;
        let output = tx.tx_output;
        for txin in input {
            let key = (txin.previous_output, txin.index);
            self.utxo.remove(&key);
        }
        let mut idx = 0;
        for txout in output {
            let tx_hash = transaction.hash();
            self.utxo
                .insert((tx_hash, idx), (txout.value, txout.recipient_addr));
            idx += 1;
        }
        println!("After state update");
        for (key, val) in self.utxo.iter() {
            println!("key: {:?}, val: {:?}", key, val);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TxIn {
    pub previous_output: H256,
    pub index: u8,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TxOut {
    pub recipient_addr: Address,
    pub value: u64,
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub tx_input: Vec<TxIn>,
    pub tx_output: Vec<TxOut>,
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let m = bincode::serialize(&self).unwrap();
        digest::digest(&digest::SHA256, m.as_ref()).into()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let m = bincode::serialize(&self).unwrap();
        digest::digest(&digest::SHA256, m.as_ref()).into()
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let msg: Vec<u8> = bincode::serialize(&t).unwrap();
    let hash: Vec<u8> = digest::digest(&digest::SHA256, &msg).as_ref().to_vec();
    key.sign(&hash)
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let msg: Vec<u8> = bincode::serialize(&t).unwrap();
    let hash: Vec<u8> = digest::digest(&digest::SHA256, &msg).as_ref().to_vec();
    let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key);
    pub_key.verify(&hash, signature).is_ok()
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    let input = vec![TxIn {
        previous_output: generate_random_hash(),
        index: 0,
    }];
    let output = vec![TxOut {
        recipient_addr: generate_random_address(),
        value: 0,
    }];

    Transaction {
        tx_input: input,
        tx_output: output,
    }
}
pub fn generate_random_hash() -> H256 {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let mut raw_bytes = [0; 32];
    raw_bytes.copy_from_slice(&random_bytes);
    (&raw_bytes).into()
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::key_pair;
    use ring::signature::KeyPair;

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }
    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
