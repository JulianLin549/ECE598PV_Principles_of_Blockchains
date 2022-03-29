use crate::types::block::Block;
use crate::types::block::*;
use crate::types::hash::{Hashable, H256};
use crate::types::merkle::MerkleTree;
use std::collections::HashMap;
#[derive(Debug, Default)]
//////
/// Blockchain
/// tip: the tip of the blockchain
/// longest: the longest chain length
/// length: keep track of height of block
//////
pub struct Blockchain {
    pub blockchain: HashMap<H256, Block>,
    pub tip: H256,
    pub longest: u128,
    pub length: HashMap<H256, u128>,
}
//////
/// Blockchain
///
//////
impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let parent: H256 = [0u8; 32].into();
        let nonce: u32 = 0;
        // difficulty_setting
        let difficulty: H256 = [
            0, 5, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ]
        .into();
        let timestamp: u128 = 0;
        let transactions = Vec::new();
        let merkle_tree = MerkleTree::new(transactions.as_ref());
        let merkle_root = merkle_tree.root();
        let header = Header {
            parent: parent,
            nonce: nonce,
            difficulty: difficulty,
            timestamp: timestamp,
            merkle_root: merkle_root,
        };
        let content = Content { data: transactions };
        let genesis_block = Block {
            header: header,
            content: content,
        };
        let mut blockchain = HashMap::new();
        let mut length = HashMap::new();
        let genesis_hash = genesis_block.hash();
        // genesis_block.header.parent = genesis_hash.clone();

        let tip = genesis_hash;
        let longest: u128 = 0;
        blockchain.insert(genesis_hash, genesis_block);
        length.insert(genesis_hash, 0);
        Blockchain {
            blockchain: blockchain,
            tip: tip,
            length: length,
            longest: longest,
        }
    }

    /// Insert a block into blockchain
    /// please use blockchain.insert instead of blockchain.blockchain.insert
    /// the later one will ruin the consistency
    pub fn insert(&mut self, block: &Block) {
        let hash = block.hash();
        let cur_len = self.length[&block.header.parent] + 1; // get current length

        // longest chain change, need to change tip and longest
        if cur_len > self.longest {
            self.tip = hash;
            self.longest = cur_len;
        }
        self.length.insert(hash, cur_len);
        self.blockchain.insert(hash, block.clone());
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut longest_chain = Vec::new();
        let mut cur_block_hash = self.tip;

        if self.longest == 0 {
            longest_chain.push(cur_block_hash);
            return longest_chain;
        }

        for _ in 0..self.longest {
            longest_chain.push(cur_block_hash);
            cur_block_hash = self.blockchain[&cur_block_hash].header.parent;
        }
        longest_chain.push(cur_block_hash);
        longest_chain.reverse();

        longest_chain
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
