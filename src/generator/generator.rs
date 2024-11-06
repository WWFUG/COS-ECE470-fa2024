use log::info;
use std::time;
use std::thread;

use ring::signature::{KeyPair, Ed25519KeyPair, Signature};
use rand::{Rng, thread_rng};
use ring::rand::SystemRandom;

use crate::types::block::Block;
use crate::types::hash::Hashable;
use crate::blockchain::Blockchain;
use crate::types::mempool::Mempool;
use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use crate::types::address::Address;
use crate::types::transaction::{SignedTransaction, sign, Transaction};
use crate::types::key_pair;
use crate::network::message::Message;

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
        Self { 
            server: server.clone(), 
            mempool: Arc::clone(mempool),
        }
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
            let mut rng = thread_rng();
            let value : u32 = rng.gen::<u32>();
            let n = 0;
            let receiver_key: [u8; 32] = rng.gen();
            
            let tx = Transaction {
                receiver: Address::from_public_key_bytes(&receiver_key), 
                value: value,
                account_nonce: n,
            };


            // Generate a key pair based on the random seed.
            let key_pair = key_pair::random();
            let pub_key = key_pair.public_key().as_ref().to_vec();            

            let sign_vec: Vec<u8> = sign(&tx, &key_pair).as_ref().to_vec();

            let signed_tx = SignedTransaction{
                transaction : tx,
                signature : sign_vec,
                public_key : pub_key,
            };

            {
                let mut mempool = self.mempool.lock().unwrap();
                mempool.insert(&signed_tx);
            }
            self.server.broadcast(Message::NewTransactionHashes(vec![signed_tx.hash()])); 

            // println!("New transaction generated: {:?}", signed_tx.hash());

            if theta != 0 {
                let interval = time::Duration::from_millis( 5*theta );
                thread::sleep(interval);
            }
        }
    }
}
