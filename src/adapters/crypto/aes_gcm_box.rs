use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use rand::RngCore;

use crate::domain::{ApiKey, DomainError, DomainResult, EncryptedBlob};
use crate::ports::SecretBox;

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub struct AesGcmSecretBox {
    cipher: Aes256Gcm,
}

impl AesGcmSecretBox {
    pub fn from_master_key_bytes(key: &[u8]) -> DomainResult<Self> {
        if key.len() != KEY_LEN {
            return Err(DomainError::Crypto(format!(
                "master key must be {KEY_LEN} bytes, got {}",
                key.len()
            )));
        }
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| DomainError::Crypto(e.to_string()))?;
        Ok(Self { cipher })
    }

    /// Accepts base64 (standard or URL-safe) or hex encoding of 32 bytes.
    pub fn from_master_key_str(raw: &str) -> DomainResult<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Crypto("SMART_HAWK_MASTER_KEY is empty".into()));
        }

        if let Ok(bytes) = hex::decode(trimmed) {
            if bytes.len() == KEY_LEN {
                return Self::from_master_key_bytes(&bytes);
            }
        }

        let engine = base64::engine::general_purpose::STANDARD;
        if let Ok(bytes) = engine.decode(trimmed) {
            if bytes.len() == KEY_LEN {
                return Self::from_master_key_bytes(&bytes);
            }
        }

        let url_engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        if let Ok(bytes) = url_engine.decode(trimmed) {
            if bytes.len() == KEY_LEN {
                return Self::from_master_key_bytes(&bytes);
            }
        }

        // Raw 32-char passphrase stretched via SHA-256 (documented for dev only)
        if trimmed.len() >= 16 {
            use sha2::{Digest, Sha256};
            let hash = Sha256::digest(trimmed.as_bytes());
            return Self::from_master_key_bytes(&hash);
        }

        Err(DomainError::Crypto(
            "SMART_HAWK_MASTER_KEY must be 32-byte hex, base64, or a passphrase (≥16 chars)".into(),
        ))
    }
}

impl SecretBox for AesGcmSecretBox {
    fn encrypt(&self, key: &ApiKey) -> DomainResult<EncryptedBlob> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, key.expose().as_bytes())
            .map_err(|e| DomainError::Crypto(format!("encrypt failed: {e}")))?;
        Ok(EncryptedBlob {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        })
    }

    fn decrypt(&self, blob: &EncryptedBlob) -> DomainResult<ApiKey> {
        if blob.nonce.len() != NONCE_LEN {
            return Err(DomainError::Crypto("invalid nonce length".into()));
        }
        let nonce = Nonce::from_slice(&blob.nonce);
        let plain = self
            .cipher
            .decrypt(nonce, blob.ciphertext.as_ref())
            .map_err(|_| DomainError::Crypto("decrypt failed (wrong master key?)".into()))?;
        let s = String::from_utf8(plain)
            .map_err(|_| DomainError::Crypto("decrypted key is not utf-8".into()))?;
        ApiKey::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let box_ = AesGcmSecretBox::from_master_key_str(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let key = ApiKey::new("sk-test-secret-key-value").unwrap();
        let blob = box_.encrypt(&key).unwrap();
        let back = box_.decrypt(&blob).unwrap();
        assert_eq!(back.expose(), key.expose());
        assert!(format!("{key:?}").contains("***"));
    }

    #[test]
    fn passphrase_stretch() {
        let box_ = AesGcmSecretBox::from_master_key_str("dev-only-passphrase!!").unwrap();
        let key = ApiKey::new("abc").unwrap();
        let blob = box_.encrypt(&key).unwrap();
        assert_eq!(box_.decrypt(&blob).unwrap().expose(), "abc");
    }
}
