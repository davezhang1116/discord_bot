# Dogecoin Discord Bot 
A Rust implementation of a dogecoin bot on Discord
## Introduction
This is a project inspired by the now-defunct Reddit Dogecoin bot. It supports sending, receiving, storing and tipping your dogecoins. This program, by default, uses the Dogecoin Testnet, meaning that all transactions done by this bot involve no monetary value.

## Setup
In order to build this program, you will need a Rust compiler (I am using rustc 1.75.0-nightly)

Rust official installation page: https://www.rust-lang.org/tools/install

And Python (I am use python 3.10)

Python download page: https://www.python.org/downloads/

You will also need to set up a Dogecoin Testnet Node

Dogecoin Core Github page: https://github.com/dogecoin/dogecoin

You may follow this page to get a discord token: https://www.writebots.com/discord-bot-token/

## Configuration
You will need to create a username and a password on your ```dogecoin.conf``` and copy the auth data (bot token) to the ```file.xml```. You may need to update the node ip address. Note: you may run into trouble if you decide to use a remote node because of firewall settings on the system or the dogecoin core.

## Building and running the program
Build

```cargo build --release```

Run

```cargo run --release```

## Example Dogecoin Core Config

```~/.dogecoin/dogecoin.conf```

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

## Sqlite

When you are compiling [libsqlite3-sys](https://crates.io/crates/libsqlite3-sys), you may need to install the C binding first. The project's Github repo is https://github.com/rusqlite/rusqlite/.

The schema for the database is 
``` 
CREATE TABLE balance( 
    id primary key not null, 
    name TEXT not null, 
    sats TEXT);
```

## libsecp256k1

You may need at least 4GB of RAM to compile this library. [github link](https://github.com/paritytech/libsecp256k1/issues/94)


## Dependencies and Repos
dogecoin-OP_RETURN: https://github.com/INCT-DD/dogecoin-OP_RETURN

anychain: https://github.com/0xcregis/anychain

Serenity: https://github.com/serenity-rs/serenity

PyO3: https://github.com/PyO3/pyo3

You may view all dependencies in ```Cargo.toml```

## Usage Help

Send coins

```
!send <testnet dogecoin address> <amount to send>
!send nZ6oQPaD4NyuhF2pyMCU2Ju3oeTWitz4Xs 102.0
```

Deposit coins

```
!deposit
```

Tip coins

```
!tip <target user mention> <amount to tip>
!tip @dave 120
```

Faucet (to get free coins)

```!faucet```

Balance (get your balance)

```!balance```

Coinflip

```
!coinflip <up or down> <amount to bet>
!coinflip up 20
```

Mines

```
!mines <amount of mines> <amount to bet>
!mines 5 20
```

OP_return Message

```
!op_return_send <your message>
!op_return_send HELLO WORLD
```
You can also attach an utf-8 encoded file (<100kb). The data will be recorded on the blockchain. You may view your data on a block explorer.
