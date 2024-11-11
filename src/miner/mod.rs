pub mod worker;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

use crate::types::block::{Header, Block, Content};
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::types::state::{State, StatePerBlock};
use crate::types::hash::{H256, Hashable};
use crate::types::mempool::Mempool;
use crate::types::merkle::MerkleTree;
use std::time::{SystemTime, UNIX_EPOCH};


enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update, // update the block in mining, it may due to new blockchain tip or new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_block_chan: Sender<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state_per_block: Arc<Mutex<StatePerBlock>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>, mempool: &Arc<Mutex<Mempool>>, 
           state_per_block: &Arc<Mutex<StatePerBlock>>) -> 
(Context, Handle, Receiver<Block>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_block_sender, finished_block_receiver) = unbounded();
    let blockchain_cloned = Arc::clone(blockchain);
    let mempool_cloned = Arc::clone(mempool);
    let state_per_block_cloned = Arc::clone(state_per_block);

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_block_chan: finished_block_sender,
        blockchain: blockchain_cloned,
        mempool: mempool_cloned,
        state_per_block: state_per_block_cloned,
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_block_receiver)
}

#[cfg(any(test,test_utilities))]
fn test_new() -> (Context, Handle, Receiver<Block>) {
    let blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(blockchain));
    let mempool = Mempool::new();
    let mempool = Arc::new(Mutex::new(mempool));
    let state_per_block = StatePerBlock::new(H256::default());
    new(&blockchain, &mempool, &state_per_block)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn miner_loop(&mut self) {
        // FIXME: put this into the loop
        let mut parent_hash = H256::default();
        let mut parent_difficulty = H256::default();

        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Miner shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Miner starting in continuous mode with lambda {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Miner shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Miner starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // TODO handling transaction using mempool
            // insert the transactions into content

            let tx_limit = 50; // NOTE: this is a temporary value, you can change it

            let mut block_txs = Vec::new();
            {
                let mempool = self.mempool.lock().unwrap();
                for tx in mempool.all_transactions() {
                    block_txs.push(tx.clone());
                    if block_txs.len() == tx_limit {
                        break;
                    }
                }
            }

            {
                let blockchain = self.blockchain.lock().unwrap();
                parent_hash = blockchain.tip();
                parent_difficulty = blockchain.get_block(&parent_hash).get_difficulty();
            }
            
            let difficulty = parent_difficulty;
            let nonce = rand::random::<u32>();
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let content = Content{ transactions: block_txs };
            let merkle_root = MerkleTree::new(&content.transactions.as_slice()).root();
            let header = Header {
                parent: parent_hash,
                nonce: nonce,
                difficulty: difficulty,
                timestamp: timestamp,
                merkle_root: merkle_root,
            };

            
            let block = Block {header, content};

            if block.hash() <= difficulty && !block.get_transactions().is_empty() {

                println!("Block tx size: {}", block.content.transactions.len());

                // TODO remove transactions in this block from mempool 
                {
                    let mut mempool = self.mempool.lock().unwrap();
                    for tx in block.content.transactions.iter() {
                        mempool.remove(&tx);
                    }
                }

                {
                    let mut blockchain = self.blockchain.lock().unwrap();
                    blockchain.insert(&block);
                }
                
                self.finished_block_chan.send(block.clone()).expect("Send finished block error");
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::hash::Hashable;

    #[test]
    #[timeout(60000)]
    fn miner_three_block() {
        let (miner_ctx, miner_handle, finished_block_chan) = super::test_new();
        miner_ctx.start();
        miner_handle.start(0);
        let mut block_prev = finished_block_chan.recv().unwrap();
        // println!("{}", block_prev.hash());
        for _ in 0..2 {
            let block_next = finished_block_chan.recv().unwrap();
            assert_eq!(block_prev.hash(), block_next.get_parent());
            block_prev = block_next;
            // println!("{}", block_prev.hash());
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST