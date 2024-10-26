use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use crate::types::hash::H256;
use crate::types::hash::Hashable; 
use crate::blockchain::Blockchain;
use crate::types::block::{Block};

use std::collections::VecDeque;
use std::collections::HashMap;

use log::{debug, warn, error};

use std::thread;

#[cfg(any(test,test_utilities))]
use super::peer::TestReceiver as PeerTestReceiver;
#[cfg(any(test,test_utilities))]
use super::server::TestReceiver as ServerTestReceiver;
#[derive(Clone)]
pub struct Worker {
    msg_chan: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>, 
}

#[derive(Clone)]
pub struct OrphanBuffer {
    buffer: HashMap<H256, Vec<Block>>,
}

impl OrphanBuffer {
    pub fn exist_parent(&self, hash: &H256) -> bool {
        self.buffer.contains_key(hash)
    }

    pub fn insert_child(&mut self, block: &Block) {
        if self.buffer.contains_key(&block.get_parent()) {
            self.buffer.get_mut(&block.get_parent()).unwrap().push(block.clone());
        } else {
            self.buffer.insert(block.get_parent(), vec![block.clone()]);
        }
    }
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain),
        }
    }

    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        let mut orphan_buffer = OrphanBuffer{buffer: HashMap::new()};
        loop {
            let result = smol::block_on(self.msg_chan.recv());
            if let Err(e) = result {
                error!("network worker terminated {}", e);
                break;
            }
            let msg = result.unwrap();
            let (msg, mut peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hash_vec) => {
                    let blockchain = self.blockchain.lock().unwrap();
                    let missing_hashes: Vec<H256> = hash_vec
                                        .into_iter()
                                        .filter(|hash| !blockchain.exist(hash))
                                        .collect();
                    drop(blockchain);
                    if !missing_hashes.is_empty() {
                        debug!(" Getting missing block hashes from peer");
                        for hash in &missing_hashes{
                            debug!("Missing block hash: {}", hash);
                        }
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                }
                Message::GetBlocks(hash_vec) => {
                    let blockchain = self.blockchain.lock().unwrap();       
                    let block_vec: Vec<Block> = hash_vec
                                        .into_iter()
                                        .filter(|hash| blockchain.exist(&hash))
                                        .map(|hash| blockchain.get_block(&hash))
                                        .collect();
                    drop(blockchain);
                    if !block_vec.is_empty(){
                        debug!(" Sending requested blocks to peer");
                        for blk in &block_vec{
                            debug!("New block hash: {}", blk.hash());
                        }
                        peer.write(Message::Blocks(block_vec));
                    }
                }
                Message::Blocks(block_vec) => {
                    let mut blockchain = self.blockchain.lock().unwrap();
                    // let mut new_blk_hashes = Vec::<H256>::new();
                    
                    // let mut block_queue: VecDeque<Block> = VecDeque::from(block_vec); // Convert vector to VecDeque

                    // while let Some(blk) = block_queue.pop_front() {
                    //     // PoW validity check
                    //     if blk.hash() > blk.get_difficulty() { // invalid block
                    //         continue;
                    //     }
                    //     // Parent check for existence
                    //     if !blockchain.exist(&blk.get_parent()) {
                    //         //handling orphan block
                    //         orphan_buffer.insert_child(&blk.clone());
                    //         continue;
                    //     }
                    //     // Consistency of difficulty check
                    //     let parent_difficulty = blockchain.get_block(&blk.get_parent()).get_difficulty();
                    //     if parent_difficulty != blk.get_difficulty() {
                    //         continue;
                    //     }

                    //     // Insert the block into the blockchain
                    //     let mut cur_blk = blk.clone();
                    //     if !blockchain.exist(&blk.hash()) {

                    //         blockchain.insert(&blk);
                    //         new_blk_hashes.push(blk.hash());
                            
                    //         // Check if the block is a parent of any orphan block
                    //         if let Some(orphan_blocks) = orphan_buffer.buffer.remove(&blk.hash()) {
                    //             for orphan in orphan_blocks {
                    //                 block_queue.push_back(orphan); // Extend with orphan blocks
                    //             }
                    //         }
                    //     }
                    // }

                    // drop(blockchain);

                    // if !orphan_buffer.buffer.is_empty() {
                    //     print!(" Handling orphan blocks");
                    //     peer.write(Message::GetBlocks(orphan_buffer.buffer.keys().cloned().collect()));
                    // }

                    // let new_blk_hashes: Vec<H256>  = block_vec
                    //         .into_iter()
                    //         .filter_map(|block| {
                    //             if !blockchain.exist(&block.hash()) {
                    //                 blockchain.insert(&block);  // Insert the block into the blockchain
                    //                 Some(block.hash())          // Return the hash to be collected
                    //             } else {
                    //                 None                        // If block already exists, skip it
                    //             }
                    //         })
                    //         .collect();
                    let new_blk_hashes: Vec<H256>  = block_vec
                            .into_iter()
                            .filter_map(|block| {
                                if !blockchain.exist(&block.hash()) {
                                    blockchain.insert(&block);  // Insert the block into the blockchain
                                    Some(block.hash())          // Return the hash to be collected
                                } else {
                                    None                        // If block already exists, skip it
                                }
                            })
                            .collect();
                    drop(blockchain);
                    if !new_blk_hashes.is_empty() {
                        debug!(" Broadcasting new block hashes");
                        for hash in &new_blk_hashes{
                            debug!("New block hash: {}", hash);
                        }
                        self.server.broadcast(Message::NewBlockHashes(new_blk_hashes));
                    }            
                }
                _ => {
                    unimplemented!();
                }
            }
        }
    }
}


#[cfg(any(test,test_utilities))]
struct TestMsgSender {
    s: smol::channel::Sender<(Vec<u8>, peer::Handle)>
}
#[cfg(any(test,test_utilities))]
impl TestMsgSender {
    fn new() -> (TestMsgSender, smol::channel::Receiver<(Vec<u8>, peer::Handle)>) {
        let (s,r) = smol::channel::unbounded();
        (TestMsgSender {s}, r)
    }

    fn send(&self, msg: Message) -> PeerTestReceiver {
        let bytes = bincode::serialize(&msg).unwrap();
        let (handle, r) = peer::Handle::test_handle();
        smol::block_on(self.s.send((bytes, handle))).unwrap();
        r
    }
}
#[cfg(any(test,test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(blockchain));
    let (server, server_receiver) = ServerHandle::new_for_test();
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    let worker = Worker::new(1, msg_chan, &server, &blockchain);
    worker.start(); 
    let all_blocks = blockchain.lock().unwrap().all_blocks_in_longest_chain();
    (test_msg_sender, server_receiver, all_blocks)
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    use super::super::message::Message;
    use super::generate_test_worker_and_start;

    #[test]
    #[timeout(60000)]
    fn reply_new_block_hashes() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut peer_receiver = test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_get_blocks() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let h = v.last().unwrap().clone();
        let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
        let reply = peer_receiver.recv();
        if let Message::Blocks(v) = reply {
            assert_eq!(1, v.len());
            assert_eq!(h, v[0].hash())
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_blocks() {
        let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
        let reply = server_receiver.recv().unwrap();
        if let Message::NewBlockHashes(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST