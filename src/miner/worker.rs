use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
use crate::network::message::Message::{NewBlockHashes, self};
use crate::types::block::Block;
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
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) -> Self {
        Self {
            server: server.clone(),
            finished_block_chan,
            blockchain: Arc::clone(blockchain),
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

            let mut blockchain = self.blockchain.lock().unwrap();
            blockchain.insert(&_block);
            drop(blockchain);
            debug!("Block {} succesfully mined; Broadcasting ...", _block.hash());
            self.server
                .broadcast(Message::NewBlockHashes(vec![_block.hash()])); // blocking operation
        }
    }


}