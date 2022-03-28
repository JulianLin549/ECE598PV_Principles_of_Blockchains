use crate::types::address::generate_random_address;
use crate::types::address::Address;
use crate::types::hash::{Hashable, H256};
use rand::Rng;
use ring::digest;
use ring::signature::{self, Ed25519KeyPair, Signature};
use serde::{Deserialize, Serialize};

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

// Tx_out: 0001[小美50, 小美50]
// tx_input: [TxIn{0001,0}, TxIn{0001,1}] v
// tx_input: [TxIn{0001,0}, TxIn{0001,0}] x double spend

// Tx1 { tx_input: [TxIn{0001,0}] tx_out: [老王50]}
// Tx2 { tx_input: [TxIn{0001,0}] tx_out: [老李50]} x double spend

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
