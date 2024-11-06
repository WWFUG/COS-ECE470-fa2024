use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::block::{Block, Content, Header};
use crate::types::hash::H256;
use crate::types::merkle::MerkleTree; // Make sure to include the MerkleTree
use crate::types::hash::Hashable; 
use std::time::SystemTime;
use hex_literal::hex;


pub struct Blockchain {
    blocks: HashMap<H256, Block>, // Storing blocks by their hash
    tip: H256, // The hash of the latest block in the longest chain
    heights: HashMap<H256, usize>, // Mapping of block hashes to their heights
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        // Create the genesis block with fixed values
        // println!("New Blockchain");
        let difficulty = hex!("0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").into(); // set difficulty
        let genesis_block = Block {
            header: Header {
                parent: H256::from([0x00; 32]), // Genesis block has no parent
                nonce: 0,
                difficulty: difficulty, // Example fixed difficulty
                timestamp: 0,
                merkle_root: H256::from([0x00; 32]), // Placeholder for merkle root
            },
            content: Content{
                        transactions: Vec::new(),}, // Use the public constructor
        };

        // println!("Genesis Block");
        let genesis_hash = genesis_block.hash();
        println!("Genesis Block {}\ndifficulty {}", genesis_hash, difficulty);
        
        // Return a new instance of Blockchain with initialized fields
        Blockchain {
            blocks: {
                let mut map = HashMap::new();
                map.insert(genesis_hash.clone(), genesis_block); // Insert the genesis block
                map
            },
            tip: genesis_hash, // Set the tip to the genesis hash
            heights: {
                let mut heights_map = HashMap::new();
                heights_map.insert(genesis_hash, 0); // Store height of the genesis block
                heights_map
            },
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        // println!("Insert Block");
        let block_hash = block.hash(); // Calculate the hash of the new block

        // Determine the height of the new block
        let new_height = match self.heights.get(&block.get_parent()) {
            Some(&parent_height) => parent_height + 1, // Increment parent's height
            None => return, // Handle invalid parent case
        };

        // Add the block to the blockchain
        self.blocks.insert(block_hash.clone(), block.clone());
        self.heights.insert(block_hash.clone(), new_height); // Store the new block's height

        // Update the tip if this block extends the longest chain
        if new_height > self.heights[&self.tip] {
            self.tip = block_hash;
        }
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        // let mut hashes = Vec::new();
        // let mut current_hash = self.tip;

        // while let Some(hash) = self.blocks.get(&current_hash) {
        //     hashes.push(current_hash);
        //     current_hash = hash.get_parent();
        // }

        // hashes.reverse(); // Return in order from genesis to tip
        // hashes
        let mut new_vec: Vec<H256> = vec!();
        let mut temp_hash = self.tip();
        for i in 0..(self.heights[&self.tip]+1){
            let temp_block = self.blocks.get(&temp_hash).unwrap();
            let temp_block_copy = temp_block.clone();
            let block_parent_hash = temp_block_copy.header.parent.clone();

            new_vec.insert(0,temp_hash);
            temp_hash = block_parent_hash;
        }
        new_vec
    }

    /// Get all transactions of the longest chain, ordered from genesis to the tip
    pub fn all_tx_in_longest_chain(&self) -> Vec<Vec<H256>> {
        let longest_chain = self.all_blocks_in_longest_chain();

        let mut tx_vec = Vec::new();
        for n in 0..longest_chain.len() {
            let block = self.blocks.get(&longest_chain[n]).unwrap();
            let txs: Vec<H256> = block.get_transactions()
                                 .into_iter().map(|tx| tx.hash()).collect();
            tx_vec.push(txs);
        }
        return tx_vec;
    }

    pub fn exist(&self, hash: &H256) -> bool {
        self.blocks.contains_key(hash)
    }

    pub fn get_block(&self, hash: &H256) -> Block {
        return self.blocks.get(hash).unwrap().clone();
    }  
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST