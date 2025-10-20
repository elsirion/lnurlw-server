use anyhow::Result;
use crate::{
    crypto::{AesKey, aes_decrypt, verify_cmac, parse_decrypted_data, CardUid, Counter},
    db::models::Card,
};

/// Result of card validation
#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    Success {
        uid: CardUid,
        counter: Counter,
    },
    Error(String),
}

/// Trait for database operations needed for validation
#[async_trait::async_trait]
pub trait CardRepository {
    async fn get_card_by_id(&self, card_id: i64) -> Result<Option<Card>>;
    async fn update_card_uid(&self, card_id: i64, uid: &str) -> Result<()>;
    async fn update_card_counter(&self, card_id: i64, counter: i64) -> Result<bool>;
}

/// Trait for crypto operations
pub trait CryptoService {
    fn decrypt(&self, key: &AesKey, ciphertext: &[u8]) -> Result<Vec<u8>>;
    fn verify_cmac(&self, key: &AesKey, uid: &CardUid, counter: &Counter, expected_cmac: &[u8]) -> Result<bool>;
    fn parse_decrypted_data(&self, decrypted: &[u8]) -> Result<(CardUid, Counter)>;
}

/// Default implementation of crypto operations
pub struct DefaultCryptoService;

impl CryptoService for DefaultCryptoService {
    fn decrypt(&self, key: &AesKey, ciphertext: &[u8]) -> Result<Vec<u8>> {
        aes_decrypt(key, ciphertext)
    }

    fn verify_cmac(&self, key: &AesKey, uid: &CardUid, counter: &Counter, expected_cmac: &[u8]) -> Result<bool> {
        verify_cmac(key, uid, counter, expected_cmac)
    }

    fn parse_decrypted_data(&self, decrypted: &[u8]) -> Result<(CardUid, Counter)> {
        parse_decrypted_data(decrypted)
    }
}

/// Card validation service
pub struct CardValidator<C: CryptoService> {
    crypto: C,
}

impl<C: CryptoService> CardValidator<C> {
    pub fn new(crypto: C) -> Self {
        Self { crypto }
    }

    /// Validate card parameters and return UID and counter if valid
    pub async fn validate_card<R: CardRepository>(
        &self,
        repo: &R,
        card_id: i64,
        p_hex: &str,
        c_hex: &str,
    ) -> ValidationResult {
        // Decode hex parameters
        let p_bytes = match hex::decode(p_hex) {
            Ok(bytes) => bytes,
            Err(_) => return ValidationResult::Error("Invalid p parameter".to_string()),
        };
        let c_bytes = match hex::decode(c_hex) {
            Ok(bytes) => bytes,
            Err(_) => return ValidationResult::Error("Invalid c parameter".to_string()),
        };

        if p_bytes.len() != 16 || c_bytes.len() != 8 {
            return ValidationResult::Error("Invalid parameter length".to_string());
        }

        // Look up the card
        let card = match repo.get_card_by_id(card_id).await {
            Ok(Some(card)) => card,
            Ok(None) => return ValidationResult::Error("Card not found".to_string()),
            Err(_) => return ValidationResult::Error("Database error".to_string()),
        };

        if !card.enabled {
            return ValidationResult::Error("Card disabled".to_string());
        }

        // Parse keys
        let k1 = match AesKey::from_hex(&card.k1_decrypt_key) {
            Ok(key) => key,
            Err(_) => return ValidationResult::Error("Invalid card key".to_string()),
        };
        let k2 = match AesKey::from_hex(&card.k2_cmac_key) {
            Ok(key) => key,
            Err(_) => return ValidationResult::Error("Invalid card key".to_string()),
        };

        // Decrypt the data
        let decrypted = match self.crypto.decrypt(&k1, &p_bytes) {
            Ok(data) => data,
            Err(_) => return ValidationResult::Error("Decryption failed".to_string()),
        };

        // Parse UID and counter
        let (uid, counter) = match self.crypto.parse_decrypted_data(&decrypted) {
            Ok((uid, counter)) => (uid, counter),
            Err(_) => return ValidationResult::Error("Invalid decrypted data".to_string()),
        };

        // Verify CMAC
        match self.crypto.verify_cmac(&k2, &uid, &counter, &c_bytes) {
            Ok(true) => {}, // CMAC is valid
            Ok(false) => return ValidationResult::Error("Invalid CMAC - card authentication failed".to_string()),
            Err(_) => return ValidationResult::Error("CMAC verification error".to_string()),
        }

        // Update UID if not set
        if card.uid.is_empty() {
            if let Err(_) = repo.update_card_uid(card_id, &uid.to_string()).await {
                return ValidationResult::Error("Database error".to_string());
            }
        } else if card.uid != uid.to_string() {
            return ValidationResult::Error("UID mismatch".to_string());
        }

        // Check and update counter (replay protection)
        if counter.value() as i64 <= card.last_counter {
            return ValidationResult::Error("Invalid counter - possible replay attack".to_string());
        }

        match repo.update_card_counter(card_id, counter.value() as i64).await {
            Ok(true) => {},
            Ok(false) => return ValidationResult::Error("Counter update failed".to_string()),
            Err(_) => return ValidationResult::Error("Database error".to_string()),
        }

        ValidationResult::Success { uid, counter }
    }
}

impl CardValidator<DefaultCryptoService> {
    /// Create a validator with default crypto service
    pub fn new_default() -> Self {
        Self::new(DefaultCryptoService)
    }
}

pub mod db_repository;
pub mod pure;

pub use pure::validate_card_pure;
