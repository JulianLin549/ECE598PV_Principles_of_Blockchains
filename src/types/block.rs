use crate::types::hash::{Hashable, H256};
use crate::types::merkle::MerkleTree;
use crate::types::transaction::SignedTransaction;
use rand::Rng;
use ring::digest;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256,
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let serialized = bincode::serialize(&self).unwrap();
        digest::digest(&ring::digest::SHA256, &serialized).into()
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
    pub data: Vec<SignedTransaction>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn get_parent(&self) -> H256 {
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    let mut rng = rand::thread_rng();
    let new_nonce: u32 = rng.gen();
    let timestamp: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut temp: [u8; 32] = [10; 32];
    let difficulty: H256 = temp.into();
    let transactions = Vec::new();

    let merkle_tree = MerkleTree::new(transactions.as_ref());
    let merkle_root = merkle_tree.root();

    let header = Header {
        parent: *parent,
        nonce: new_nonce,
        difficulty: difficulty,
        timestamp: timestamp,
        merkle_root: merkle_root,
    };
    let content = Content { data: transactions };
    Block {
        header: header,
        content: content,
    }
}
