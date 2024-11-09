use std::collections::HashMap;
use rand::Rng;
use ring::signature::{Ed25519KeyPair, KeyPair};
use super::{
    address::Address,
};
use super::transaction::{Transaction, SignedTransaction, verify, sign};
use super::hash::H256;
use super::block::Block;
use crate::types::hash::Hashable;


#[derive(Debug, Clone)]
pub struct AccountState{
    pub nonce: u32,
    pub balance: u32,
}

#[derive(Debug, Clone)]
pub struct State {
    pub account_states: HashMap<Address, AccountState>,
}

impl State {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        // generate three deterministic accounts accorss 3 nodes

        let seed = vec![0, 1, 2];

        for i in 0..3 {
            let key_pair = Ed25519KeyPair::from_seed_unchecked(&[seed[i];32]).unwrap();
            let pub_key = key_pair.public_key();
            let address = Address::from_public_key_bytes(pub_key.as_ref());
            let balance = 10000;
            let account_state = AccountState {
                nonce: 0,
                balance: balance,
            };
            states.insert(address, account_state);
        }
        Self {
            account_states: states,
        }
    }

    pub fn get_balance(&self, address: &Address) -> u32 {
        self.account_states.get(address).map(|state| state.balance).unwrap()
    }

    pub fn get_nonce(&self, address: &Address) -> u32 {
        self.account_states.get(address).map(|state| state.nonce).unwrap()
    }

    pub fn exist(&self, address: &Address) -> bool {
        self.account_states.contains_key(address)
    }

    // assume the nonce always starts from 0
    pub fn add_account(&mut self, address: Address, balance: u32) {
        let account_state = AccountState {
            nonce: 0,
            balance: balance,
        };
        self.account_states.insert(address, account_state);
    }

    // make sure that the transaction is valid before calling this function
    // also make sure that the sender and receiver exist
    pub fn update_with_tx(&mut self, signed_tx: &SignedTransaction) {
        let tx = signed_tx.transaction.clone();
        let receiver = tx.receiver.clone();
        let value = tx.value;
        let nonce = tx.account_nonce;
        let sender = Address::from_public_key_bytes(&signed_tx.public_key);
    
        assert!( self.exist(&sender) && self.exist(&receiver) );
        assert!( self.get_balance(&sender) >= value );


        let mut sender_state = self.account_states.get(&sender).unwrap().clone();
        let mut receiver_state = self.account_states.get(&receiver).unwrap().clone();
        sender_state.nonce += 1;
        sender_state.balance -= value;
        receiver_state.balance += value;

        // reinsert the updated states since rust does not support mutable reference to a value in a hashmap
        self.account_states.insert(sender, sender_state);
        self.account_states.insert(receiver, receiver_state);
    }

}



#[derive(Debug, Clone)]
pub struct StatePerBlock{
    pub state_copy: HashMap<H256, State>, // from block hash to State
}

impl StatePerBlock {
    pub fn new(genesis_hash: H256) -> Self {
        let state = State::new(); // three initial accounts initialized
        let mut s_copy = HashMap::new();
        s_copy.insert(genesis_hash, state);
        Self {
            state_copy: s_copy,
        }
    }

    pub fn get_state(&self, hash: &H256) -> State {
        self.state_copy.get(hash).unwrap().clone()
    }

    pub fn exist(&self, hash: &H256) -> bool {
        self.state_copy.contains_key(hash)
    }

    // make sure that the block contains valid transactions
    pub fn update_with_block(&mut self, block: &Block) {
        assert!(!self.exist(&block.hash()));

        let parent_hash = block.header.parent.clone();
        assert!(self.exist(&parent_hash));

        let parent_state = self.get_state(&parent_hash);
        let mut state = parent_state.clone();

        for tx in block.get_transactions() {
            let signed_tx = tx.clone();
            state.update_with_tx(&signed_tx);
        }

        self.state_copy.insert(block.hash(), state);
    }
}