let request = require('request-promise');
const fs = require('fs').promises;

let myUrls = ['http://127.0.0.1:7000/blockchain/longest-chain-tx', 
                'http://127.0.0.1:7001/blockchain/longest-chain-tx']

let txs_urls = ['http://127.0.0.1:7000/blockchain/txs-in-mempool', 
'http://127.0.0.1:7001/blockchain/txs-in-mempool'] ;

async function load() {
	try {

	let results = await Promise.all(myUrls.map(request));
	let txs1 = JSON.parse(results[0]);
	let txs2 = JSON.parse(results[1]);
	await fs.writeFile('tx1.json', JSON.stringify(txs1));
	await fs.writeFile('tx2.json', JSON.stringify(txs2));

	console.log("block size:", txs1.length, txs2.length)
	let flag1 = true;
	if(txs1.length > txs2.length){
		for(let i=0; i< txs2.length; i++){
			if (JSON.stringify(txs1[i]) !== JSON.stringify(txs2[i])){
				flag1 = false;
				break;
			}
		}
	}else{
		for(let i=0; i< txs1.length; i++){
			if (JSON.stringify(txs1[i]) !== JSON.stringify(txs2[i])){
				flag1 = false;
				break;
			}
		}
	}

	console.log("tx equal?", flag1)
	results = await Promise.all(txs_urls.map(request));
	txs1 = JSON.parse(results[0]);
	txs2 = JSON.parse(results[1]);
	console.log("mempool length:", txs1.length, txs2.length);

	} catch (e) {
	console.log(e);
	}
}
load()
