use log::info;
use std::time;
use std::thread;

use ring::signature::{KeyPair, Ed25519KeyPair, Signature};

use crate::types::block::Block;
use crate::types::hash::Hashable;
use crate::blockchain::Blockchain;
use crate::types::mempool::Mempool;
use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use crate::types::address::Address;
use crate::types::transaction::{SignedTransaction, sign, Transaction};
use crate::types::key_pair::random;

#[derive(Clone)]
pub struct TransactionGenerator {
    server: ServerHandle,
    mempool: Arc<Mutex<Mempool>>,
}

impl TransactionGenerator {
    pub fn new(
        server: &ServerHandle,
        mempool: &Arc<Mutex<Mempool>>,
    ) -> Self {
        Self {server.clone(), Arc::clone(mempool)}
    }

    pub fn start(self, theta: u64) {
        thread::Builder::new()
            .name("transaction-generator".to_string())
            .spawn(move || {
                self.generate_transactions(theta);
            })
            .unwrap();
        info!("Transaction generator started");
    }

    fn generate_transactions(&self, theta: u64) {
        loop {
            // implement transaction generation logic here

            // In part 7 it's fine to generate a random transaction
            let receiver = Address::random();
            let value = rand::random::<u32>();
            let account_nonce = rand::random::<u32>(); 
            let _new_tx = Transaction{
                receiver,
                value,
                account_nonce,
            }

            let key = key_pair::random();
            let signature = transaction::sign(&_new_tx, &key);
            let _signed_tx = SignedTransaction{
                transaction: _new_tx,
                signature: signature.as_ref().to_vec(),
                public_key: key.public_key().as_ref().to_vec(),
            };


            let mut mempool = self.mempool.lock().unwrap();
            mempool.insert(&_signed_tx);
            drop(mempool);
            self.server.broadcast(Message::NewTransactionHashes(vec![_signed_tx.hash()])); 

            if theta != 0 {
                let interval = time::Duration::from_millis(10 * theta);
                thread::sleep(interval);
            }
        }
    }
}
