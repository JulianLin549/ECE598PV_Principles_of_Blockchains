let request = require('request-promise');
const fs = require('fs').promises;

let myUrls = ['http://127.0.0.1:7000/blockchain/longest-chain', 
                'http://127.0.0.1:7001/blockchain/longest-chain']

async function load() {
	try {

	let results = await Promise.all(myUrls.map(request));
	let txs1 = JSON.parse(results[0]);
	let txs2 = JSON.parse(results[1]);

	console.log(txs1.length, txs2.length)
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

	console.log(flag1)
	
	} catch (e) {
	console.log(e);
	}
}
load()
