#!/bin/sh
# This is a comment!

./target/debug/bitcoin --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 
./target/debug/bitcoin --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 
./target/debug/bitcoin --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001