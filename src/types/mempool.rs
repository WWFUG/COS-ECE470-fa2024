use super::{
    hash::{Hashable, H256},
    transaction::SignedTransaction,
};

#[derive(Debug, Default, Clone)]
pub struct Mempool {
    transactions: HashMap<H256, SignedTransaction>,
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

}
