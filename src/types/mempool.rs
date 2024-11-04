use super::{
    hash::{Hashable, H256},
    transaction::SignedTransaction,
};

#[derive(Debug, Default, Clone)]
pub struct Mempool {
    pub transactions: HashMap<H256, SignedTransaction>,
}

impl Mempool {
    pub fn new() -> Self {
        let transactions: HashMap <H256, SignedTransaction> = HashMap::new();
        Mempool { transactions }
    }

    pub fn all_transactions(&self) -> Vec<SignedTransaction> {
        let mut ret_vec = Vec::new();
        for (_, transaction) in self.transactions.iter() {
            transactions.push(transaction.clone());
        }
        ret_vec
    }

    pub fn insert(&mut self, tx: &SignedTransaction) {
        self.transactions.insert(tx.hash(), tx.clone());
    }

    pub fn remove(&mut self, tx: &SignedTransaction) {
        self.transactions.remove(&tx.hash());
    }

    pub fn exist(&self, hash: &H256) -> bool {
        self.transactions.contains_key(hash)
    }

    pub fn get_tx(&self, hash: &H256) -> SignedTransaction {
        self.transactions.get(hash).unwrap().clone()
    }

}
