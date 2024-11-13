use log::info;
use std::time;
use std::thread;
use std::sync::{Arc, Mutex};

use ring::signature::{KeyPair, Ed25519KeyPair};
use rand::{Rng, thread_rng};
use ring::rand::SystemRandom;

use crate::types::block::Block;
use crate::types::hash::Hashable;
use crate::blockchain::Blockchain;
use crate::types::mempool::Mempool;
use crate::types::state::{State, StatePerBlock};
use crate::network::server::Handle as ServerHandle;
use crate::types::address::Address;
use crate::types::transaction::{SignedTransaction, sign, Transaction};
use crate::types::key_pair;
use crate::network::message::Message;

#[derive(Clone)]
pub struct TransactionGenerator {
    server: ServerHandle,
    mempool: Arc<Mutex<Mempool>>,
    state_per_block: Arc<Mutex<StatePerBlock>>,
    blockchain: Arc<Mutex<Blockchain>>,
    vec_key_pairs: Vec<Arc<Ed25519KeyPair>>,
}

impl TransactionGenerator {
    pub fn new(
        server: &ServerHandle,
        mempool: &Arc<Mutex<Mempool>>,
        state_per_block: &Arc<Mutex<StatePerBlock>>,
        blockchain: &Arc<Mutex<Blockchain>>,
        key_pair: Arc<Ed25519KeyPair>,
    ) -> Self {
        Self { 
            server: server.clone(), 
            mempool: Arc::clone(mempool),
            state_per_block: Arc::clone(state_per_block),
            blockchain: Arc::clone(blockchain),
            vec_key_pairs: vec![key_pair],
        }
    }

    pub fn start(mut self, theta: u64) {
        thread::Builder::new()
            .name("transaction-generator".to_string())
            .spawn(move || {
                self.generate_transactions(theta);
            })
            .unwrap();
        info!("Transaction generator started");
    }

    fn generate_transactions(&mut self, theta: u64) {
        let mut rng = thread_rng();
    
        loop {
            // get the tip of the blockchain
            let mut tip_hash;
            {
                let blockchain = self.blockchain.lock().unwrap();
                tip_hash = blockchain.tip();
            }
            // acquire the current state
            let mut cur_state;
            {
                let state_per_block = self.state_per_block.lock().unwrap();
                cur_state = state_per_block.get_state(&tip_hash);
            }
    
            // Randomly select a sender with non-zero balance
            let mut sender_index;
            let mut sender_pub_key;
            let mut sender_account;
            loop {
                sender_index = rng.gen_range(0..self.vec_key_pairs.len());
                sender_pub_key = self.vec_key_pairs[sender_index].public_key().clone();  // Clone here to avoid later immutable borrow
                sender_account = Address::from_public_key_bytes(sender_pub_key.as_ref());
                if cur_state.get_balance(&sender_account) > 0 {
                    break;
                }
            }
    
            // Generate a valid transaction value and nonce
            let value: u32 = rng.gen_range(1..cur_state.get_balance(&sender_account));
            let n = cur_state.get_nonce(&sender_account) + 1;
    
            // Generate or pick a receiver
            let receiver_account = if rng.gen_range(1..10) >= 9 {
                let receiver_key_pair = Arc::new(key_pair::random());
                self.vec_key_pairs.push(Arc::clone(&receiver_key_pair));  // Mutably borrow here
                Address::from_public_key_bytes(receiver_key_pair.public_key().as_ref())
            } else {
                let accounts: Vec<_> = self.vec_key_pairs.iter().map(|kp| {
                    Address::from_public_key_bytes(kp.public_key().as_ref())
                }).collect();
    
                let mut receiver_account;
                loop {
                    let receiver_index = rng.gen_range(0..accounts.len());
                    receiver_account = accounts[receiver_index];
                    if receiver_account != sender_account {
                        break;
                    }
                }
                receiver_account
            };
    
            // Create and sign the transaction
            let tx = Transaction {
                receiver: receiver_account,
                value,
                account_nonce: n,
            };
            let signature = sign(&tx, &self.vec_key_pairs[sender_index]);
            let signed_tx = SignedTransaction {
                transaction: tx,
                signature: signature.as_ref().to_vec(),
                public_key: sender_pub_key.as_ref().to_vec(),
            };
    
            // Insert into the mempool and broadcast
            {
                let mut mempool = self.mempool.lock().unwrap();
                mempool.insert(&signed_tx);
            }
            self.server.broadcast(Message::NewTransactionHashes(vec![signed_tx.hash()]));
    
            // Control generation frequency
            if theta != 0 {
                let interval = time::Duration::from_millis(5 * theta);
                thread::sleep(interval);
            }
        }
    }
}
