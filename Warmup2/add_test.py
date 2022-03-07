test_code = r'''

#[cfg(test)]
mod tests {
    use crate::types::hash::H256;
    use super::*;
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
    #[test]
    fn sp2022autograder011() {
        let input_data: Vec<H256> = gen_merkle_tree_assignment2!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6e18c8441bc8b0d1f0d4dc442c0d82ff2b4f38e2d7ca487c92e6db435d820a10")).into()
        );
    }
    #[test]
    fn sp2022autograder012() {
        let input_data: Vec<H256> = gen_merkle_tree_assignment2!();
        let merkle_tree = MerkleTree::new(&input_data);
        for i in 0.. input_data.len() {
            let proof = merkle_tree.proof(i);
            assert!(verify(&merkle_tree.root(), &input_data[i].hash(), &proof, i, input_data.len()));
        }
        let input_data_2: Vec<H256> = gen_merkle_tree_assignment2_another!();
        let merkle_tree_2 = MerkleTree::new(&input_data_2);
        assert!(!verify(&merkle_tree.root(), &input_data[0].hash(), &merkle_tree_2.proof(0), 0, input_data.len()));
    }
    #[test]
    fn sp2022autograder013() {
        use std::collections::HashSet;
        let input_data: Vec<H256> = gen_merkle_tree_assignment2!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(5);
        let proof: HashSet<H256> = proof.into_iter().collect();
        let p: H256 = (hex!("c8c37c89fcc6ee7f5e8237d2b7ed8c17640c154f8d7751c774719b2b82040c76")).into();
        assert!(proof.contains(&p));
        let p: H256 = (hex!("bada70a695501195fb5ad950a5a41c02c0f9c449a918937267710a0425151b77")).into();
        assert!(proof.contains(&p));
        let p: H256 = (hex!("1e28fb71415f259bd4b0b3b98d67a1240b4f3bed5923aa222c5fdbd97c8fb002")).into();
        assert!(proof.contains(&p));
    }

}

'''
import re
import sys
import os.path as path
file_path = path.join(sys.argv[1], 'src','types','merkle.rs')
print(path.dirname(file_path), end=' ')
before_pat = r'// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST'
after_pat = r'// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST'
change_before = False
change_after = False
file_changed = []
with open(file_path) as fin:
    for line in fin:
        if after_pat in line:
            change_after = True
        if not change_before or change_before and change_after:
            file_changed.append(line)
        if before_pat in line:
            change_before = True
            file_changed.append(test_code)
if change_before and change_after:
    print("\033[92m {}\033[00m".format("Changed the test code"))
    with open(file_path, "w") as fout:
        fout.write(''.join(file_changed))
else:
    print("\033[91m {}\033[00m".format("Code format wrong"))