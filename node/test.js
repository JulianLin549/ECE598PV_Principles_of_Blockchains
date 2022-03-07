let request = require('request-promise');

let myUrls = ['http://127.0.0.1:7000/blockchain/longest-chain', 
                'http://127.0.0.1:7001/blockchain/longest-chain', 
                'http://127.0.0.1:7002/blockchain/longest-chain'] ;
// let myUrls = ['http://127.0.0.1:7000/blockchain/longest-chain-tx-count', 
// 'http://127.0.0.1:7001/blockchain/longest-chain-tx-count', 
// 'http://127.0.0.1:7002/blockchain/longest-chain-tx-count'] 

// let myUrls = ['http://127.0.0.1:7000/blockchain/txs-in-mempool', 'http://127.0.0.1:7001/blockchain/txs-in-mempool',' http://127.0.0.1:7002/blockchain/txs-in-mempool'] ;

async function load() {
  try {

    let results = await Promise.all(myUrls.map(request));
    let block1 = JSON.parse(results[0]);
    let block2 = JSON.parse(results[1]);
    let block3 = JSON.parse(results[2]);
    // let block1 = results[0];
    // let block2 = results[1];
    // let block3 = results[2];

    console.log(block1.length, block2.length, block3.length)
  } catch (e) {
    console.log(e);
  }
}
load()
