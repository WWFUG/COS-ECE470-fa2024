use super::hash::{Hashable, H256};

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    nodes: Vec<H256>,
    n: usize,
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        // create an empty mutable H256 vector
        let mut hashed_vec: Vec<H256> = Vec::new();
        // store hashed value into hashed_vec
        let mut n = data.len();

        for i in 0..n{
            let hashed = data[i].hash();
            hashed_vec.push(hashed);
        }
        // merging adjacent trees
        n /= 2;
        let mut base = 0;
        while n > 0 {
            for i in 0..n{
                let mut concat = Vec::new();
                concat.extend_from_slice(hashed_vec[base+2*i].as_ref());
                concat.extend_from_slice(hashed_vec[base+2*i+1].as_ref());
                // let hashed = concat.hash();
                let hashed: H256 = ring::digest::digest(&ring::digest::SHA256, &concat).into();
                hashed_vec.push(hashed);
                // update base
                base += 2*n;
                // update n
                n /= 2;
            }
        }
        MerkleTree{nodes: hashed_vec, n: data.len()}
    }

    pub fn root(&self) -> H256 {
        if self.nodes.is_empty() {
            return H256::default(); // or some other appropriate default value
        }
        *(self.nodes.last().unwrap())
    }


    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut level_cnt = self.n;
        let mut idx = index;
        let mut base = 0;
        let mut proof = Vec::new();

        while level_cnt > 1 {
            let mut sibling_id;
            if idx % 2 == 0{
                sibling_id = idx+1;
            } else {
                sibling_id = idx-1;
            }              
            
            proof.push(self.nodes[base+sibling_id]);
            
            base += level_cnt;
            idx = idx/2;
            level_cnt /= 2;
        }

        proof
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut hashed: H256 = datum.clone(); // the datum is already hashed
    let mut idx = index;
    for sibling in proof{
        let mut concat = Vec::with_capacity(64);
        if idx % 2 == 0 {
            concat.extend_from_slice(hashed.as_ref());
            concat.extend_from_slice(sibling.as_ref());
            // println!("left: {}", &hashed);
            // println!("right: {}", &sibling);
        } else {
            concat.extend_from_slice(sibling.as_ref());
            concat.extend_from_slice(hashed.as_ref());
            // println!("left: {}", &sibling);
            // println!("right: {}", &hashed);
        }
        hashed = ring::digest::digest(&ring::digest::SHA256, &concat).into();
        // println!("hash: {}", &hashed);
        idx /= 2;
    }
    println!("{}", &hashed);
    println!("{}", root);
    &hashed == root
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use crate::types::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }

}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST