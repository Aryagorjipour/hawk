use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Plaintext API key. Never logs its contents.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(raw: impl Into<String>) -> Result<Self, super::error::DomainError> {
        let s = raw.into().trim().to_string();
        if s.is_empty() {
            return Err(super::error::DomainError::Validation(
                "API key must not be empty".into(),
            ));
        }
        if s.len() > 512 {
            return Err(super::error::DomainError::Validation(
                "API key is unreasonably long".into(),
            ));
        }
        Ok(Self(s))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn fingerprint_bytes(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.0.as_bytes());
        hasher.finalize().into()
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// Encrypted secret blob stored in the database.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl fmt::Debug for EncryptedBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedBlob")
            .field("nonce_len", &self.nonce.len())
            .field("ciphertext_len", &self.ciphertext.len())
            .finish()
    }
}
