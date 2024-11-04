use log::info;
use std::time;
use std::thread;
use crate::types::block::Block;
use crate::types::hash::Hashable;
use crate::blockchain::Blockchain;
use crate::types::mempool::Mempool;
use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};

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
            unimplemented!();

            if theta != 0 {
                let interval = time::Duration::from_millis(10 * theta);
                thread::sleep(interval);
            }
        }
    }
}
