use super::hash::{Hashable, H256};
use ring::digest;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct MerkleTree {
    tree: Vec<H256>,
    leaf_size: usize,
}

fn get_height(leaf_num: usize) -> u32 {
    let mut height = 0;
    let mut leaf_num = leaf_num;
    while leaf_num > 1 {
        height += 1;
        leaf_num /= 2;
    }
    return height;
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self
    where
        T: Hashable,
    {
        let mut tree: Vec<H256> = Vec::new();
        let mut data_len = data.len();

        if data_len == 0 {
            let data_block: [u8; 32] = [0; 32];
            tree.push(data_block.into());
            return MerkleTree {
                tree: tree,
                leaf_size: 0,
            };
        }
        let mut queue: VecDeque<H256> = VecDeque::new();

        for elem in data.into_iter() {
            queue.push_back(elem.hash());
        }

        if data_len % 2 != 0 {
            queue.push_back(data[data_len - 1].hash());
            data_len = data_len + 1;
        }

        let mut count = 0;
        let mut level_len = data_len;
        while !queue.is_empty() {
            let hash1 = queue.pop_front().unwrap();
            tree.push(hash1);
            count = count + 1;
            let temp = queue.pop_front();
            if temp == None {
                break;
            }

            let hash2 = temp.unwrap();
            tree.push(hash2);
            count = count + 1;

            let mut hash = digest::Context::new(&digest::SHA256);
            hash.update(hash1.as_ref());
            hash.update(hash2.as_ref());
            let parent: H256 = hash.finish().into();
            queue.push_back(parent);

            if count == level_len && count != 2 {
                let i = level_len / 2;
                if i % 2 != 0 {
                    queue.push_back(parent);
                    level_len = i + 1;
                    count = 0;
                } else {
                    level_len = i;
                    count = 0;
                }
            }
        }

        MerkleTree {
            tree: tree,
            leaf_size: data_len,
        }
    }

    pub fn root(&self) -> H256 {
        self.tree[self.tree.len() - 1]
    }

    fn construct_proof(&self, proof: &mut Vec<H256>, height: u32, index: usize) {
        let mut cur_layer_index = index;
        let mut offset = 0;
        for i in 0..height {
            let small_offset = (cur_layer_index - offset) / 2;
            if cur_layer_index % 2 == 0 {
                proof.push(self.tree[cur_layer_index + 1]);
            } else {
                proof.push(self.tree[cur_layer_index - 1]);
            }

            if i != 0 {
                offset += 2usize.pow(height - i);
            } else {
                offset += self.leaf_size;
            }
            cur_layer_index = offset + small_offset;
        }
    }

    // Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof = Vec::new();
        if index >= self.leaf_size {
            // invalid index
            return proof;
        }

        let height = get_height(self.leaf_size);
        self.construct_proof(&mut proof, height, index);
        return proof;
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.

pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, _leaf_size: usize) -> bool {
    let height = proof.len();
    //temp
    let mut cur_node = *datum;
    let mut cur_index = index;

    for iter in 0..height {
        let mut ctx = digest::Context::new(&digest::SHA256);

        if cur_index % 2 == 0 {
            ctx.update(cur_node.as_ref());
            ctx.update(proof[iter].as_ref());
        } else {
            ctx.update(proof[iter].as_ref());
            ctx.update(cur_node.as_ref());
        }

        cur_node = ctx.finish().into();

        cur_index = cur_index / 2;
    }

    return cur_node == *root;
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash::H256;

    macro_rules! gen_merkle_tree_assignment2 {
        () => {{
            vec![
                (hex!("0000000000000000000000000000000000000000000000000000000000000011")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000022")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000033")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000044")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000055")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000066")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000077")).into(),
                (hex!("0000000000000000000000000000000000000000000000000000000000000088")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_assignment2_another {
        () => {{
            vec![
                (hex!("1000000000000000000000000000000000000000000000000000000000000088")).into(),
                (hex!("2000000000000000000000000000000000000000000000000000000000000077")).into(),
                (hex!("3000000000000000000000000000000000000000000000000000000000000066")).into(),
                (hex!("4000000000000000000000000000000000000000000000000000000000000055")).into(),
                (hex!("5000000000000000000000000000000000000000000000000000000000000044")).into(),
                (hex!("6000000000000000000000000000000000000000000000000000000000000033")).into(),
                (hex!("7000000000000000000000000000000000000000000000000000000000000022")).into(),
                (hex!("8000000000000000000000000000000000000000000000000000000000000011")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(
            proof,
            vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(
            &merkle_tree.root(),
            &input_data[0].hash(),
            &proof,
            0,
            input_data.len()
        ));
    }

    #[test]
    fn merkle_012() {
        let input_data: Vec<H256> = gen_merkle_tree_assignment2!();
        let merkle_tree = MerkleTree::new(&input_data);
        for i in 0..input_data.len() {
            let proof = merkle_tree.proof(i);
            assert!(verify(
                &merkle_tree.root(),
                &input_data[i].hash(),
                &proof,
                i,
                input_data.len()
            ));
        }
        let input_data_2: Vec<H256> = gen_merkle_tree_assignment2_another!();
        let merkle_tree_2 = MerkleTree::new(&input_data_2);
        assert!(!verify(
            &merkle_tree.root(),
            &input_data[0].hash(),
            &merkle_tree_2.proof(0),
            0,
            input_data.len()
        ));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
