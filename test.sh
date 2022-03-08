#!/bin/sh
# This is a comment!
# node ./node/longest-chain-tx.js 
# ./target/debug/bitcoin --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 
# ./target/debug/bitcoin --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 
# ./target/debug/bitcoin --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001

curl http://127.0.0.1:7000/tx-generator/start?theta=100
curl http://127.0.0.1:7001/tx-generator/start?theta=100
curl http://127.0.0.1:7002/tx-generator/start?theta=100

curl http://127.0.0.1:7000/miner/start?lambda=0
# curl http://127.0.0.1:7001/miner/start?lambda=0
# curl http://127.0.0.1:7002/miner/start?lambda=0