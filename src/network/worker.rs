use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::mempool::Mempool;
use crate::types::address::Address;
use crate::types::block::Block;
use crate::types::hash::Hashable;
use crate::types::hash::H256;
use crate::types::state::BlockToStateMap;
use crate::types::state::State;
use crate::types::transaction::verify;
use crate::types::transaction::SignedTransaction;

use ring::digest;

use crate::Blockchain;
use log::{debug, error, warn};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[cfg(any(test, test_utilities))]
use super::peer::TestReceiver as PeerTestReceiver;
#[cfg(any(test, test_utilities))]
use super::server::TestReceiver as ServerTestReceiver;
use std::thread;
#[derive(Clone)]
pub struct Worker {
    msg_chan: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_mempool: Arc<Mutex<Mempool>>,
    orphan_buffer: Arc<Mutex<HashMap<H256, Block>>>,
    state: Arc<Mutex<State>>,
    bts_map: Arc<Mutex<BlockToStateMap>>,
}

impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        tx_mempool: &Arc<Mutex<Mempool>>,
        orphan_buffer: &Arc<Mutex<HashMap<H256, Block>>>,
        state: &Arc<Mutex<State>>,
        bts_map: &Arc<Mutex<BlockToStateMap>>,
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain),
            tx_mempool: Arc::clone(&tx_mempool),
            orphan_buffer: Arc::clone(&orphan_buffer),
            state: Arc::clone(&state),
            bts_map: Arc::clone(&bts_map),
        }
    }

    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let result = smol::block_on(self.msg_chan.recv());
            if let Err(e) = result {
                error!("network worker terminated {}", e);
                break;
            }
            let msg = result.unwrap();
            let (msg, mut peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            let mut blockchain_with_lock = self.blockchain.lock().unwrap();
            let mut mempool_with_lock = self.tx_mempool.lock().unwrap();
            let mut state_with_lock = self.state.lock().unwrap();
            let mut orphan_buffer = self.orphan_buffer.lock().unwrap();
            let mut bts_map_with_lock = self.bts_map.lock().unwrap();

            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                // receiving NewBlockHashes message mean that peer have new blocks to serve
                Message::NewBlockHashes(recv_new_hashes) => {
                    println!("Receive NewBlockHashes message");
                    // check if blocks are in chain
                    for recv_hash in recv_new_hashes {
                        // if block already exists in either blockchain or orphan_buffer, skip
                        if blockchain_with_lock.blockchain.contains_key(&recv_hash)
                            || orphan_buffer.contains_key(&recv_hash)
                        {
                            continue;
                        }
                        peer.write(Message::GetBlocks(vec![recv_hash.clone()]));
                    }
                }

                // receiving GetBlocks message mean that peer do not have the blocks, if I have it, I send it.
                Message::GetBlocks(missing_hashes) => {
                    println!("Receive GetBlocks message");
                    for missing_hash in missing_hashes {
                        // if found block in either blockchain or orphan_buffer, send it
                        if blockchain_with_lock.blockchain.contains_key(&missing_hash) {
                            peer.write(Message::Blocks(vec![blockchain_with_lock.blockchain
                                [&missing_hash]
                                .clone()]));
                        }
                        if orphan_buffer.contains_key(&missing_hash) {
                            peer.write(Message::Blocks(vec![orphan_buffer[&missing_hash].clone()]));
                        }
                    }
                }
                // receiving Blocks message means receiving new blocks
                Message::Blocks(recv_blocks) => {
                    println!("new block received!");
                    let mut new_block_hashes: Vec<H256> = Vec::new();
                    let mut get_blocks = Vec::new();
                    for block in recv_blocks {
                        //if new block not in blockchain
                        if !blockchain_with_lock.blockchain.contains_key(&block.hash()) {
                            // if parent not in blockchain, then is orphan
                            // insert into orphan buffer
                            if !blockchain_with_lock
                                .blockchain
                                .contains_key(&block.header.parent)
                            {
                                if !orphan_buffer.contains_key(&block.hash()) {
                                    get_blocks.push(block.header.parent);
                                    orphan_buffer.insert(block.header.parent, block);
                                }
                            } else {
                                // if parent in block chain
                                // if block hash smaller or equal to parent difficulty and not in blockchain, then proceed
                                if block.hash()
                                    <= blockchain_with_lock.blockchain[&block.header.parent]
                                        .header
                                        .difficulty
                                {
                                    let txs = block.clone().content.data;
                                    //check all transactions in block are valid
                                    if !is_block_tx_valid(txs.clone(), state_with_lock.clone()) {
                                        continue;
                                    }

                                    // block received is valid, remove tx from mempool, update state
                                    for tx in txs {
                                        mempool_with_lock.remove(&tx);
                                        state_with_lock.update(&tx);

                                        // remove any double spend tx_in in mempool found in block
                                        // mark tx_in in block as spent in spent_tx_in
                                        for tx_in in tx.clone().transaction.tx_input {
                                            if mempool_with_lock
                                                .spent_tx_in
                                                .contains_key(&(tx_in.previous_output, tx_in.index))
                                            {
                                                // remove tx in mempool using hash
                                                let tx_hash = mempool_with_lock.spent_tx_in
                                                    [&(tx_in.previous_output, tx_in.index)];
                                                mempool_with_lock.remove_with_hash(tx_hash);
                                            }
                                            // add tx_in to spent_tx_in, mark as spent
                                            mempool_with_lock.spent_tx_in.insert(
                                                (tx_in.previous_output, tx_in.index),
                                                tx.hash(),
                                            );
                                        }
                                    }

                                    //insert into block-to-state-map
                                    bts_map_with_lock.insert(block.hash(), state_with_lock.clone());
                                    // insert into blockchain
                                    blockchain_with_lock.insert(&block);
                                    new_block_hashes.push(block.hash());
                                    println!("new block inserted!");

                                    //if current block is orphan_buffer block's parent
                                    let mut queue: VecDeque<H256> = VecDeque::new();
                                    // bfs
                                    queue.push_back(block.hash());
                                    while !queue.is_empty() {
                                        let cur_hash = queue.pop_front().unwrap();
                                        let mut orphans_with_parent = Vec::new();

                                        for (hash, orphan_block) in orphan_buffer.iter() {
                                            if orphan_block.header.parent == cur_hash {
                                                let txs = orphan_block.clone().content.data;

                                                //remove tx from mempool
                                                for tx in txs {
                                                    mempool_with_lock.remove(&tx);
                                                    state_with_lock.update(&tx);
                                                    //remove any double spend tx_in in mempool found in block
                                                    //add tx_in in block to spent_tx_in
                                                    for tx_in in tx.clone().transaction.tx_input {
                                                        if mempool_with_lock
                                                            .spent_tx_in
                                                            .contains_key(&(
                                                                tx_in.previous_output,
                                                                tx_in.index,
                                                            ))
                                                        {
                                                            // remove tx in mempool using hash
                                                            let tx_hash = mempool_with_lock
                                                                .spent_tx_in[&(
                                                                tx_in.previous_output,
                                                                tx_in.index,
                                                            )];
                                                            mempool_with_lock
                                                                .remove_with_hash(tx_hash);
                                                        }
                                                        // add tx_in to spent_tx_in
                                                        mempool_with_lock.spent_tx_in.insert(
                                                            (tx_in.previous_output, tx_in.index),
                                                            tx.hash(),
                                                        );
                                                    }
                                                }
                                                //insert into block-to-state-map
                                                bts_map_with_lock
                                                    .insert(block.hash(), state_with_lock.clone());
                                                // insert into blockchain
                                                blockchain_with_lock.insert(&orphan_block);

                                                orphans_with_parent.push(hash.clone());

                                                queue.push_back(hash.clone())
                                            }
                                        }
                                        for inserted_block in orphans_with_parent {
                                            orphan_buffer.remove(&inserted_block);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if get_blocks.len() != 0 {
                        self.server.broadcast(Message::GetBlocks(get_blocks));
                    }
                    if new_block_hashes.len() != 0 {
                        self.server
                            .broadcast(Message::NewBlockHashes(new_block_hashes));
                    }
                }
                Message::NewTransactionHashes(recv_new_hashes) => {
                    let mut missing_txs: Vec<H256> = Vec::new();
                    for recv_tx_hash in recv_new_hashes {
                        if !mempool_with_lock.tx_evidence.contains(&recv_tx_hash) {
                            missing_txs.push(recv_tx_hash.clone());
                        } else {
                            // println!("tx {} already in mempool", recv_tx_hash);
                        }
                    }
                    if missing_txs.len() != 0 {
                        peer.write(Message::GetTransactions(missing_txs));
                    }
                }

                Message::GetTransactions(missing_txs_hash) => {
                    let mut txs_to_send: Vec<SignedTransaction> = Vec::new();

                    for missing_tx_hash in missing_txs_hash {
                        if mempool_with_lock.tx_map.contains_key(&missing_tx_hash) {
                            let tx = mempool_with_lock.tx_map[&missing_tx_hash].clone();
                            txs_to_send.push(tx);
                        }
                    }
                    if txs_to_send.len() != 0 {
                        peer.write(Message::Transactions(txs_to_send));
                    }
                }
                Message::Transactions(signed_txs) => {
                    let mut new_tx_hashes: Vec<H256> = Vec::new();
                    for signed_tx in signed_txs {
                        // check is transaction valid
                        if !transaction_check(signed_tx.clone(), state_with_lock.clone()) {
                            continue;
                        }

                        let signed_tx_hash = signed_tx.hash();
                        if !mempool_with_lock.tx_evidence.contains(&signed_tx_hash) {
                            // if pass double spend check, (in mempool.insert)
                            // then insert tx into current node's mempool then send to everyone
                            if mempool_with_lock.insert(&signed_tx) {
                                new_tx_hashes.push(signed_tx.hash());
                            }
                        }
                    }

                    self.server
                        .broadcast(Message::NewTransactionHashes(new_tx_hashes));
                }
            }

            std::mem::drop(blockchain_with_lock);
            std::mem::drop(mempool_with_lock);
            std::mem::drop(orphan_buffer);
            std::mem::drop(state_with_lock);
            std::mem::drop(bts_map_with_lock);
        }
    }
}
pub fn is_block_tx_valid(signed_txs: Vec<SignedTransaction>, state_with_lock: State) -> bool {
    for signed_tx in signed_txs.clone() {
        if !transaction_check(signed_tx, state_with_lock.clone()) {
            return false;
        }
    }
    true
}

pub fn transaction_check(signed_tx: SignedTransaction, state_with_lock: State) -> bool {
    //Verify digital signature of a transaction
    //Check if the transaction is signed correctly by the public key(s).
    if !verify(
        &signed_tx.transaction,
        &signed_tx.public_key,
        &signed_tx.signature,
    ) {
        println!("fail tx signature check");
        return false;
    }
    let tx_ins = signed_tx.transaction.tx_input;
    let mut input_amount: u64 = 0;
    let owner_pk = signed_tx.public_key;
    for tx_in in tx_ins {
        let pre_out = tx_in.previous_output;
        let index = tx_in.index;
        // check if the inputs to the transactions are not spent i.e. exist in State
        if !state_with_lock.utxo.contains_key(&(pre_out, index)) {
            println!("fail signature check: tx_in not exist, tx double spend");
            return false;
        }
        let temp = state_with_lock.utxo[&(pre_out, index)]; //find tx in state
        let pre_tx_recipient_address = temp.1;
        let owner_pk_hash: H256 = digest::digest(&digest::SHA256, owner_pk.as_ref()).into();
        let owner_address: Address = owner_pk_hash.to_addr();
        //check the public key(s) matches the owner(s)'s address of these inputs.
        if owner_address != pre_tx_recipient_address {
            println!("fail signature check: owner of tx input doesn't match to previous tx output");
            return false;
        }
        input_amount += temp.0;
    }
    let tx_outputs = signed_tx.transaction.tx_output;
    let mut output_amount: u64 = 0;
    for tx_out in tx_outputs {
        output_amount += tx_out.value;
    }
    // Spending Check: check the values of inputs are not less than those of outputs.
    if input_amount < output_amount {
        println!("fail spending check: input less than output");
        return false;
    }
    true
}

#[cfg(any(test, test_utilities))]
struct TestMsgSender {
    s: smol::channel::Sender<(Vec<u8>, peer::Handle)>,
}
#[cfg(any(test, test_utilities))]
impl TestMsgSender {
    fn new() -> (
        TestMsgSender,
        smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    ) {
        let (s, r) = smol::channel::unbounded();
        (TestMsgSender { s }, r)
    }

    fn send(&self, msg: Message) -> PeerTestReceiver {
        let bytes = bincode::serialize(&msg).unwrap();
        let (handle, r) = peer::Handle::test_handle();
        smol::block_on(self.s.send((bytes, handle))).unwrap();
        r
    }
}
#[cfg(any(test, test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let (server, server_receiver) = ServerHandle::new_for_test();
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    let blockchain = Blockchain::new();
    let mut hashes: Vec<H256> = Vec::new();

    for (hash, _) in blockchain.blockchain.iter() {
        hashes.push(*hash);
    }
    let blockchain = Arc::new(Mutex::new(blockchain));
    let tx_mempool = Arc::new(Mutex::new(Mempool::new()));
    let orphan_buffer: HashMap<H256, Block> = HashMap::new();
    let orphan_buffer = Arc::new(Mutex::new(orphan_buffer));
    let state = State::new();
    let state = Arc::new(Mutex::new(state));
    let bts_map = BlockToStateMap::new();
    let bts_map = Arc::new(Mutex::new(bts_map));

    let worker = Worker::new(
        1,
        msg_chan,
        &server,
        &blockchain,
        &tx_mempool,
        &orphan_buffer,
        &state,
        &bts_map,
    );
    worker.start();
    (test_msg_sender, server_receiver, hashes)
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;
    use ntest::timeout;

    use super::super::message::Message;
    use super::generate_test_worker_and_start;

    #[test]
    #[timeout(60000)]
    fn reply_new_block_hashes() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut peer_receiver =
            test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_get_blocks() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let h = v.last().unwrap().clone();
        let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
        let reply = peer_receiver.recv();
        if let Message::Blocks(v) = reply {
            assert_eq!(1, v.len());
            assert_eq!(h, v[0].hash())
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(6000)]
    fn reply_blocks() {
        let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        println!("random block {:?}", random_block);

        let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
        let reply = server_receiver.recv().unwrap();

        if let Message::NewBlockHashes(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
