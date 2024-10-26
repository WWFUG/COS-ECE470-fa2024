use serde::{Serialize, Deserialize};
use crate::types::hash::{H256, Hashable};
use super::transaction::SignedTransaction;
use super::merkle::MerkleTree;
use ring::digest;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content(pub Vec<SignedTransaction>);


impl Content {
    pub fn new(transactions: Vec<SignedTransaction>) -> Self {
        Content(transactions)
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let encoded_strans = bincode::serialize(&self).unwrap();
        let digest = digest::digest(&digest::SHA256, &encoded_strans);
        digest.into()
    }
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let encoded_header = bincode::serialize(&self).unwrap();
        let digest = digest::digest(&digest::SHA256, &encoded_header);
        digest.into()
    }
}

impl Block {
    pub fn get_parent(&self) -> H256 {
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    let nonce = rand::random::<u32>(); // Generate random nonce
    let content = Content(Vec::new()); // Empty content

    let merkle_root = MerkleTree::new(&Vec::<H256>::new()).root(); // Empty Merkle tree
    println!("After???");
    let header = Header {
        parent: *parent,
        nonce,
        difficulty: H256::default(), 
        timestamp: std::time::SystemTime::now(), // Current system time
        merkle_root,
    };

    Block { header, content }
}