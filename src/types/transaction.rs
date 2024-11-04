use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, UnparsedPublicKey, ED25519};
use rand::Rng;
use super::address::Address;

// We use UTXO model for transactions
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub receiver: Address,
    pub value: u32,
    pub account_nonce: u32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    transaction: Transaction,
    signature: Vec<u8>,
    public_key: Vec<u8>,
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let encoded_trans = bincode::serialize(&t).unwrap();
    key.sign(&encoded_trans[..])
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let encoded_trans = bincode::serialize(&t).unwrap();
    let decoded_pub_key = UnparsedPublicKey::new(&ED25519, public_key);
    decoded_pub_key.verify(&encoded_trans, signature).is_ok()
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    let mut rng = rand::thread_rng();
    let mut sender = [0u8; 20];
    let mut receiver = [0u8; 20];
    let value : u32 = rng.gen::<u32>();
    rng.fill(&mut sender);
    rng.fill(&mut receiver);

    Transaction{
        sender: Address::from(sender),
        receiver: Address::from(receiver),
        value: value, // Random value between 1 and 1000
    }
    
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::key_pair;
    use ring::signature::KeyPair;


    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }
    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST