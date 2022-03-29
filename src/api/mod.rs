use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::miner::Handle as MinerHandle;
use crate::network::message::Message;
use crate::network::server::Handle as NetworkServerHandle;
use crate::tx_generator::{self, Handle as TxGeneratorHandle};
use crate::types::address::Address;
use crate::types::hash::{Hashable, H256};
use crate::types::state::State;
use crate::BlockToStateMap;

use serde::Serialize;

use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server as HTTPServer;
use url::Url;

pub struct Server {
    handle: HTTPServer,
    miner: MinerHandle,
    network: NetworkServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_generator: TxGeneratorHandle,
    tx_mempool: Arc<Mutex<Mempool>>,
    state: Arc<Mutex<State>>,
    bts_map: Arc<Mutex<BlockToStateMap>>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let payload = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let resp = Response::from_string(serde_json::to_string_pretty(&payload).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}
macro_rules! respond_json {
    ( $req:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let resp = Response::from_string(serde_json::to_string(&$message).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

impl Server {
    pub fn start(
        addr: std::net::SocketAddr,
        miner: &MinerHandle,
        network: &NetworkServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        tx_generator: &TxGeneratorHandle,
        tx_mempool: &Arc<Mutex<Mempool>>,
        state: &Arc<Mutex<State>>,
        bts_map: &Arc<Mutex<BlockToStateMap>>,
    ) {
        let handle = HTTPServer::http(&addr).unwrap();
        let server = Self {
            handle,
            miner: miner.clone(),
            network: network.clone(),
            blockchain: Arc::clone(blockchain),
            tx_generator: tx_generator.clone(),
            tx_mempool: Arc::clone(tx_mempool),
            state: Arc::clone(state),
            bts_map: Arc::clone(bts_map),
        };
        thread::spawn(move || {
            for req in server.handle.incoming_requests() {
                let miner = server.miner.clone();
                let tx_generator = server.tx_generator.clone();
                let network = server.network.clone();
                let blockchain = Arc::clone(&server.blockchain);
                let state = Arc::clone(&server.state);
                let tx_mempool = Arc::clone(&server.tx_mempool);
                let bts_map = Arc::clone(&server.bts_map);
                thread::spawn(move || {
                    // a valid url requires a base
                    let base_url = Url::parse(&format!("http://{}/", &addr)).unwrap();
                    let url = match base_url.join(req.url()) {
                        Ok(u) => u,
                        Err(e) => {
                            respond_result!(req, false, format!("error parsing url: {}", e));
                            return;
                        }
                    };
                    match url.path() {
                        "/miner/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            miner.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        "/tx-generator/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let theta = match params.get("theta") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing theta");
                                    return;
                                }
                            };
                            let theta = match theta.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing theta: {}", e)
                                    );
                                    return;
                                }
                            };
                            tx_generator.start(theta);
                            respond_result!(req, true, "ok");
                        }
                        "/network/ping" => {
                            network.broadcast(Message::Ping(String::from("Test ping")));
                            respond_result!(req, true, "ok");
                        }
                        "/blockchain/longest-chain" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            let v_string: Vec<String> =
                                v.into_iter().map(|h| h.to_string()).collect();
                            std::mem::drop(blockchain);
                            respond_json!(req, v_string);
                        }
                        "/blockchain/longest-chain-tx" => {
                            let mut result = Vec::new();
                            let blockchain_with_lock = blockchain.lock().unwrap();
                            let block_hashes = blockchain_with_lock.all_blocks_in_longest_chain();
                            for block_hash in block_hashes {
                                let block = blockchain_with_lock.blockchain[&block_hash].clone();
                                let txs = block.content.data;
                                let mut txs_hashes: Vec<String> = Vec::new();
                                for tx in txs {
                                    txs_hashes.push(tx.hash().to_string());
                                }
                                result.push(txs_hashes);
                            }
                            std::mem::drop(blockchain_with_lock);
                            respond_json!(req, result);
                        }
                        "/blockchain/longest-chain-tx-count" => {
                            let mut result = 0;
                            let blockchain_with_lock = blockchain.lock().unwrap();
                            let block_hashes = blockchain_with_lock.all_blocks_in_longest_chain();
                            for block_hash in block_hashes {
                                let block = blockchain_with_lock.blockchain[&block_hash].clone();
                                let txs = block.content.data;
                                result += txs.len();
                            }
                            std::mem::drop(blockchain_with_lock);
                            respond_json!(req, result);
                        }
                        "/blockchain/txs-in-mempool" => {
                            let mempool_with_lock = tx_mempool.lock().unwrap();
                            let tx_map = &mempool_with_lock.tx_map;
                            let mut result = Vec::new();
                            for tx_hash in tx_map.keys() {
                                result.push(tx_hash.to_string());
                            }
                            std::mem::drop(mempool_with_lock);
                            respond_json!(req, result);
                        }
                        "/blockchain/state" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let height = match params.get("block") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing block number");
                                    return;
                                }
                            };
                            let height: u64 = match height.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing block number: {}", e)
                                    );
                                    return;
                                }
                            };
                            let blockchain_with_lock = blockchain.lock().unwrap();
                            let state_with_lock = state.lock().unwrap();
                            let bts_map_with_lock = bts_map.lock().unwrap();
                            let mut state_string: Vec<(String, String, String, String)> = vec![];
                            // get the hight-th block in longest chain's state_map
                            let longest_chain_blocks: Vec<H256> =
                                blockchain_with_lock.all_blocks_in_longest_chain();
                            if height as usize > longest_chain_blocks.len() - 1 {
                                respond_result!(req, false, "block number exceed longest-chain.");
                            }
                            let block_hash = longest_chain_blocks[height as usize];
                            let state: State = bts_map_with_lock.bts_map[&block_hash];
                            let utxo = state.utxo;

                            for (k, v) in utxo.iter() {
                                let tx_hash = k.clone().0.to_string();
                                let index = k.clone().1.to_string();
                                let value = v.clone().0.to_string();
                                let recipient = v.clone().1.to_string();
                                state_string.push((tx_hash, index, value, recipient));
                            }
                            std::mem::drop(blockchain_with_lock);
                            std::mem::drop(state_with_lock);
                            std::mem::drop(bts_map_with_lock);

                            respond_json!(req, state_string);
                        }
                        _ => {
                            let content_type =
                                "Content-Type: application/json".parse::<Header>().unwrap();
                            let payload = ApiResponse {
                                success: false,
                                message: "endpoint not found".to_string(),
                            };
                            let resp = Response::from_string(
                                serde_json::to_string_pretty(&payload).unwrap(),
                            )
                            .with_header(content_type)
                            .with_status_code(404);
                            req.respond(resp).unwrap();
                        }
                    }
                });
            }
        });
        info!("API server listening at {}", &addr);
    }
}
