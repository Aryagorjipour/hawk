use crate::domain::{ApiKey, DomainResult, EncryptedBlob};

pub trait SecretBox: Send + Sync {
    fn encrypt(&self, key: &ApiKey) -> DomainResult<EncryptedBlob>;
    fn decrypt(&self, blob: &EncryptedBlob) -> DomainResult<ApiKey>;
}
