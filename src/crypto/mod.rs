use aes::Aes128;
use cipher::{BlockDecrypt, KeyInit};
use cmac::{Cmac, Mac};
use hex;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::fmt;

/// A 16-byte AES key
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AesKey([u8; 16]);

impl AesKey {
    pub fn generate() -> Self {
        let bytes: [u8; 16] = rand::random();
        Self(bytes)
    }
    
    pub fn from_hex(s: &str) -> Result<Self> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 16 {
            return Err(anyhow!("AES key must be 16 bytes"));
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
    
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for AesKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Serialize for AesKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AesKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// A 7-byte card UID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardUid([u8; 7]);

impl CardUid {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 7 {
            return Err(anyhow!("UID must be 7 bytes"));
        }
        let mut arr = [0u8; 7];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
    
    pub fn from_hex(s: &str) -> Result<Self> {
        let bytes = hex::decode(s)?;
        Self::from_bytes(&bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 7] {
        &self.0
    }
}

impl fmt::Display for CardUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Serialize for CardUid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CardUid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// Card counter value for replay protection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Counter(u32);

impl Counter {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 3 {
            return Err(anyhow!("Counter must be 3 bytes"));
        }
        // Little-endian
        let value = u32::from(bytes[2]) << 16 
                  | u32::from(bytes[1]) << 8 
                  | u32::from(bytes[0]);
        Ok(Self(value))
    }
    
    pub fn to_bytes(&self) -> [u8; 3] {
        [
            (self.0 & 0xFF) as u8,
            ((self.0 >> 8) & 0xFF) as u8,
            ((self.0 >> 16) & 0xFF) as u8,
        ]
    }
    
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn aes_decrypt(key: &AesKey, ciphertext: &[u8]) -> Result<Vec<u8>> {
    if ciphertext.len() != 16 {
        return Err(anyhow!("Ciphertext must be 16 bytes"));
    }
    
    let cipher = Aes128::new_from_slice(key.as_bytes()).map_err(|e| anyhow!("Invalid key length: {:?}", e))?;
    let mut block = [0u8; 16];
    block.copy_from_slice(ciphertext);
    
    cipher.decrypt_block((&mut block).into());
    Ok(block.to_vec())
}

pub fn verify_cmac(key: &AesKey, uid: &CardUid, counter: &Counter, expected_cmac: &[u8]) -> Result<bool> {
    if expected_cmac.len() != 8 {
        return Err(anyhow!("CMAC must be 8 bytes"));
    }
    
    // Build SV2 data structure for CMAC
    let mut sv2 = [0u8; 16];
    sv2[0] = 0x3c;
    sv2[1] = 0xc3;
    sv2[2] = 0x00;
    sv2[3] = 0x01;
    sv2[4] = 0x00;
    sv2[5] = 0x80;
    sv2[6..13].copy_from_slice(uid.as_bytes());
    let counter_bytes = counter.to_bytes();
    sv2[13..16].copy_from_slice(&counter_bytes);
    
    let mut mac = <Cmac<Aes128> as Mac>::new_from_slice(key.as_bytes()).map_err(|e| anyhow!("Invalid key length: {:?}", e))?;
    mac.update(&sv2);
    let result = mac.finalize();
    let computed = result.into_bytes();
    
    // Compare first 8 bytes of computed CMAC with expected
    Ok(computed[..8] == *expected_cmac)
}

pub fn parse_decrypted_data(decrypted: &[u8]) -> Result<(CardUid, Counter)> {
    if decrypted.len() != 16 {
        return Err(anyhow!("Decrypted data must be 16 bytes"));
    }
    
    // Check for 0xC7 prefix
    if decrypted[0] != 0xC7 {
        return Err(anyhow!("Invalid decrypted data format"));
    }
    
    // Extract UID (7 bytes)
    let uid = CardUid::from_bytes(&decrypted[1..8])?;
    
    // Extract counter (3 bytes at positions 8,9,10)
    let counter = Counter::from_bytes(&decrypted[8..11])?;
    
    Ok((uid, counter))
}