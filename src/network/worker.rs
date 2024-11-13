use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use crate::types::hash::H256;
use crate::types::hash::Hashable; 
use crate::blockchain::Blockchain;
use crate::types::state::{State, StatePerBlock};
use crate::types::mempool::Mempool;
use crate::types::block::{Block};
use crate::types::transaction::{SignedTransaction, Transaction, verify};
use crate::types::key_pair;
use crate::types::address::Address;

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
    mempool: Arc<Mutex<Mempool>>,
    state_per_block: Arc<Mutex<StatePerBlock>>,
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
        let parent = block.get_parent();
        if self.buffer.contains_key(&parent) {
            self.buffer.get_mut(&parent).unwrap().push(block.clone());
        } else {
            self.buffer.insert(parent, vec![block.clone()]);
        }
    }
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        mempool: &Arc<Mutex<Mempool>>,
        state_per_block: &Arc<Mutex<StatePerBlock>>,
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain),
            mempool: Arc::clone(mempool),
            state_per_block: Arc::clone(state_per_block),
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
                    debug!("Receive New Block Hashes");
                    let mut missing_hashes: Vec<H256> = Vec::new();
                    {
                        let blockchain = self.blockchain.lock().unwrap();
                        missing_hashes = hash_vec
                                        .into_iter()
                                        .filter(|hash| !blockchain.exist(hash))
                                        .collect();
                    }   
                    if !missing_hashes.is_empty() {
                        debug!("Request Missing Blocks");
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                }
                Message::GetBlocks(hash_vec) => {
                    debug!("Receive Get Blocks");
                    let mut block_vec: Vec<Block> = Vec::new(); 
                    {  
                        let blockchain = self.blockchain.lock().unwrap();
                        block_vec  = hash_vec
                                    .into_iter()
                                    .filter(|hash| blockchain.exist(&hash))
                                    .map(|hash| blockchain.get_block(&hash))
                                    .collect();
                    }
                    if !block_vec.is_empty(){
                        debug!("Send Blocks");
                        peer.write(Message::Blocks(block_vec));
                    }
                }
                Message::Blocks(block_vec) => {
                    debug!("Receive Blocks");
                    let mut new_blk_hashes = Vec::<H256>::new();
                    let mut block_queue: VecDeque<Block> = VecDeque::from(block_vec); // Convert vector to VecDeque

                    let mut cur_state: State;;

                    {
                        let mut blockchain = self.blockchain.lock().unwrap();
                        // Process the blocks in the queue
                        while let Some(blk) = block_queue.pop_front() {
                            
                            // PoW validity check
                            debug!("Processing Block hash: {}", blk.hash());
                            if blk.hash() > blk.get_difficulty() { // invalid block
                                continue;
                            }

                            // Parent check for existence
                            if !blockchain.exist(&blk.get_parent()) {
                                //handling orphan block
                                orphan_buffer.insert_child(&blk.clone());

                                peer.write(Message::GetBlocks(vec![blk.get_parent()]));
                                continue;
                            }

                            // Consistency of difficulty check
                            let parent_difficulty = blockchain.get_block(&blk.get_parent()).get_difficulty();
                            if parent_difficulty != blk.get_difficulty() {
                                continue;
                            }

                            // Check if the transactions in the block are invalid
                            let mut invalid_tx = false;
                            for tx in &blk.content.transactions {
                                if !verify(&tx.transaction, &tx.public_key, &tx.signature) {
                                    invalid_tx = true;
                                    break;
                                }
                                let sender_account = Address::from_public_key_bytes(&tx.public_key);
                                let value = tx.transaction.value;
                                let nonce = tx.transaction.account_nonce;
                                let cur_state = self.state_per_block.lock().unwrap().get_state(&blk.get_parent());
                                if (cur_state.get_balance(&sender_account) < value) || 
                                   (cur_state.get_nonce(&sender_account)+1 != nonce) {
                                    invalid_tx = true;
                                    break;
                                }
                            }

                            if invalid_tx {
                                continue;
                            }

                            // Insert the block into the blockchain
                            if !blockchain.exist(&blk.hash()) {

                                blockchain.insert(&blk);
                                self.state_per_block.lock().unwrap().update_with_block(&blk);

                                // remove transactions in this block from mempool
                                // update mempool
                                {
                                    let mut mempool = self.mempool.lock().unwrap();
                                    for tx in &blk.content.transactions {
                                        mempool.remove(&tx);
                                    }
                                }

                                new_blk_hashes.push(blk.hash());
                                debug!("Block {} inserted", blk.hash());
                                
                                // Check if the block is a parent of any orphan block
                                if let Some(orphan_blocks) = orphan_buffer.buffer.remove(&blk.hash()) {
                                    for orphan in orphan_blocks {
                                        block_queue.push_back(orphan); // Extend with orphan blocks
                                    }
                                }
                            }
                        }
                    }

                    if !new_blk_hashes.is_empty() {
                        debug!("Broadcasting new block hashes");
                        self.server.broadcast(Message::NewBlockHashes(new_blk_hashes));
                    }            
                }
                Message::NewTransactionHashes(hash_vec) => {
                    debug!("Receive New Tx Hashes");
                    let mut missing_hashes: Vec<H256> = Vec::new();
                    {
                        let mempool = self.mempool.lock().unwrap();
                        missing_hashes = hash_vec
                                        .into_iter()
                                        .filter(|hash| !mempool.exist(&hash))
                                        .collect();
                    }
                    if !missing_hashes.is_empty() {
                        debug!("Reqeest Missing Txs");
                        peer.write(Message::GetTransactions(missing_hashes));
                    }
                }
                Message::GetTransactions(hash_vec) => {
                    debug!("Receive Get Txs");
                    let mut tx_vec: Vec<SignedTransaction> = Vec::new();
                    {
                        let mempool = self.mempool.lock().unwrap();
                        tx_vec = hash_vec
                                .into_iter()
                                .filter(|hash| mempool.exist(&hash))
                                .map(|hash| mempool.get_tx(&hash))
                                .collect();
                    }
                    if !tx_vec.is_empty(){
                        debug!("Send Txs");
                        peer.write(Message::Transactions(tx_vec));
                    }
                }
                Message::Transactions(tx_vec) => {
                    debug!("Receive Txs");
                    let mut new_tx_hashes = Vec::<H256>::new();
                    {
                        let mut mempool = self.mempool.lock().unwrap();
                        for signed_tx in tx_vec{
                            // Check transaction validity
                            if !verify(&signed_tx.transaction, &signed_tx.public_key, 
                                    &signed_tx.signature) {
                                debug!("Invalid Tx");
                                // println!("Invalid Tx!!!");
                                continue;
                            }

                            // Check if the transaction is already in the mempool
                            if !mempool.exist(&signed_tx.hash()) {
                                mempool.insert(&signed_tx);
                                new_tx_hashes.push(signed_tx.hash());
                                debug!("Tx {} inserted", signed_tx.hash());
                            }
                        }
                    }

                    if !new_tx_hashes.is_empty() {
                        debug!("Broadcasting new tx hashes");
                        self.server.broadcast(Message::NewTransactionHashes(new_tx_hashes));
                    }
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