use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Ed25519 keypair wrapper
pub struct KeyPair {
    inner: SigningKey,
}

impl KeyPair {
    /// Generate a new keypair
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        Self { inner: signing_key }
    }

    /// Get public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.inner.verifying_key().to_bytes().to_vec()
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.inner.sign(data).to_bytes().to_vec()
    }
}

/// Seed for community registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    pub onion_address: String,
    pub public_key: Vec<u8>,
    pub timestamp: u64,
    pub gate_address: String,
    pub signature: Vec<u8>,
}

impl Seed {
    /// Get signing data (all fields except signature)
    pub fn signing_data(&self) -> Vec<u8> {
        let data = format!(
            "{}:{}:{}:{}",
            self.onion_address,
            hex::encode(&self.public_key),
            self.timestamp,
            self.gate_address
        );
        data.into_bytes()
    }
}

/// Sign a seed with a keypair
pub fn sign_seed(keypair: &KeyPair, seed: &mut Seed) {
    let data = seed.signing_data();
    seed.signature = keypair.sign(&data);
}

/// Verify a seed's signature
pub fn verify_seed_signature(seed: &Seed) -> bool {
    // Parse public key
    let public_key_bytes: [u8; 32] = match seed.public_key.as_slice().try_into() {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let public_key = match VerifyingKey::from_bytes(&public_key_bytes) {
        Ok(pk) => pk,
        Err(_) => return false,
    };

    // Parse signature
    let signature_bytes: [u8; 64] = match seed.signature.as_slice().try_into() {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let signature = Signature::from_bytes(&signature_bytes);

    // Verify
    let data = seed.signing_data();
    public_key.verify(&data, &signature).is_ok()
}

// Hex encoding helper
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key_bytes();

        assert_eq!(public_key.len(), 32);
    }

    #[test]
    fn test_keypair_signing() {
        let keypair = KeyPair::generate();
        let data = b"test message";
        let signature = keypair.sign(data);

        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_seed_signing_and_verification() {
        let keypair = KeyPair::generate();

        let mut seed = Seed {
            onion_address: "test123.onion".to_string(),
            public_key: keypair.public_key_bytes(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            gate_address: "http://127.0.0.1:9002".to_string(),
            signature: Vec::new(),
        };

        // Sign
        sign_seed(&keypair, &mut seed);

        // Verify
        assert!(verify_seed_signature(&seed));
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = KeyPair::generate();

        let seed = Seed {
            onion_address: "test123.onion".to_string(),
            public_key: keypair.public_key_bytes(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            gate_address: "http://127.0.0.1:9002".to_string(),
            signature: vec![0u8; 64], // Invalid signature
        };

        // Should fail verification
        assert!(!verify_seed_signature(&seed));
    }

    #[test]
    fn test_tampered_seed() {
        let keypair = KeyPair::generate();

        let mut seed = Seed {
            onion_address: "test123.onion".to_string(),
            public_key: keypair.public_key_bytes(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            gate_address: "http://127.0.0.1:9002".to_string(),
            signature: Vec::new(),
        };

        // Sign
        sign_seed(&keypair, &mut seed);

        // Tamper with data
        seed.onion_address = "tampered.onion".to_string();

        // Should fail verification
        assert!(!verify_seed_signature(&seed));
    }
}
