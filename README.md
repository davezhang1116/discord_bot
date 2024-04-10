# Dogecoin Discord Bot 
A Rust implementation of a dogecoin bot on Discord
## Introduction
This is a project inspired by the now-defunct Reddit Dogecoin bot. It supports sending, receiving, storing and tipping your dogecoins. This program, by default, uses the Dogecoin Testnet, meaning that all transactions done by this bot involve no monetary value.

## Setup
In order to build this program, you will need a Rust compiler (I am using rustc 1.75.0-nightly)

Rust official installation page: https://www.rust-lang.org/tools/install

You will also need to set up a Dogecoin Testnet Node

Dogecoin Core Github page: https://github.com/dogecoin/dogecoin

## Configuration
You will need to create a username and a password on your ```dogecoin.conf``` and copy the auth data to the ```file.xml```. You may need to update the node ip address. Note: you may run into trouble if you decide to use a remote node because of firewall setting on the system or the dogecoin core.

## Building and running the program
Build

```cargo build --release```

Run

```cargo run --release```

## Known problems

When you are compiling [libsqlite3-sys](https://crates.io/crates/libsqlite3-sys), you may need to install the C binding first. The project's Github repo is https://github.com/rusqlite/rusqlite/.


## Example Dogecoin Core Config

```
testnet=1
server=1
rpcbind=0.0.0.0
rpcuser=dave
rpcpassword=password
rpcallowip=127.0.0.1
rpcallowip=10.0.0.1/225.225.255.0
rpcport=44555
rpcconnect=127.0.0.1
disablesafemode=1
```

