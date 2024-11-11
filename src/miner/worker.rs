use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
use crate::network::message::Message::{NewBlockHashes, self};
use crate::types::block::Block;
use crate::types::mempool::Mempool;
use crate::types::state::{StatePerBlock, State};
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::{Blockchain};
use crate::types::hash::Hashable;
use std::thread;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Worker {
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
    state_per_block: Arc<Mutex<StatePerBlock>>,
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
        blockchain: &Arc<Mutex<Blockchain>>,
        state_per_block: &Arc<Mutex<StatePerBlock>>,
    ) -> Self {
        Self {
            server: server.clone(),
            finished_block_chan,
            blockchain: Arc::clone(blockchain),
            state_per_block: Arc::clone(state_per_block),
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("miner-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let _block = self
                .finished_block_chan
                .recv()
                .expect("Receive finished block error");

            {
                let mut blockchain = self.blockchain.lock().unwrap();
                blockchain.insert(&_block);
                debug!("Block {} succesfully mined; Broadcasting ...", _block.hash());
                self.server
                    .broadcast(Message::NewBlockHashes(vec![_block.hash()])); // blocking operation
            }
        }
    }


}