use anyhow::Result;
use crate::crypto::{AesKey, aes_decrypt, verify_cmac, parse_decrypted_data, CardUid, Counter};

/// Result of pure card validation
#[derive(Debug, PartialEq)]
pub struct ValidationResult {
    pub uid: CardUid,
    pub counter: Counter,
}

/// Pure validation function that validates card parameters without database dependencies
///
/// # Arguments
/// * `k1_hex` - K1 decrypt key as hex string
/// * `k2_hex` - K2 CMAC key as hex string
/// * `p_hex` - Encrypted UID + counter as hex string
/// * `c_hex` - CMAC as hex string
///
/// # Returns
/// * `Ok(ValidationResult)` - Contains UID and counter if validation succeeds
/// * `Err(String)` - Error message if validation fails
pub fn validate_card_pure(
    k1_hex: &str,
    k2_hex: &str,
    p_hex: &str,
    c_hex: &str,
) -> Result<ValidationResult, String> {
    // Decode hex parameters
    let p_bytes = hex::decode(p_hex)
        .map_err(|_| "Invalid p parameter")?;
    let c_bytes = hex::decode(c_hex)
        .map_err(|_| "Invalid c parameter")?;

    if p_bytes.len() != 16 || c_bytes.len() != 8 {
        return Err("Invalid parameter length".to_string());
    }

    // Parse keys
    let k1 = AesKey::from_hex(k1_hex)
        .map_err(|_| "Invalid k1 key")?;
    let k2 = AesKey::from_hex(k2_hex)
        .map_err(|_| "Invalid k2 key")?;

    // Decrypt the data
    let decrypted = aes_decrypt(&k1, &p_bytes)
        .map_err(|_| "Decryption failed")?;

    // Parse UID and counter
    let (uid, counter) = parse_decrypted_data(&decrypted)
        .map_err(|_| "Invalid decrypted data")?;

    // Verify CMAC
    match verify_cmac(&k2, &uid, &counter, &c_bytes) {
        Ok(true) => Ok(ValidationResult { uid, counter }),
        Ok(false) => Err("Invalid CMAC - card authentication failed".to_string()),
        Err(_) => Err("CMAC verification error".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test data from the boltcard test vectors (known working)
    const TEST_K1_DECRYPT_KEY: &str = "0c3b25d92b38ae443229dd59ad34b85d";
    const TEST_K2_CMAC_KEY: &str = "b45775776cb224c75bcde7ca3704e933";
    const TEST_P_ENCRYPTED: &str = "4E2E289D945A66BB13377A728884E867";
    const TEST_C_CMAC: &str = "E19CCB1FED8892CE";

    #[test]
    fn test_validation_success_with_real_data() {
        // Debug: Let's manually test each step
        use crate::crypto::{AesKey, aes_decrypt, parse_decrypted_data, verify_cmac};

        // Step 1: Decrypt
        let k1 = AesKey::from_hex(TEST_K1_DECRYPT_KEY).unwrap();
        let p_bytes = hex::decode(TEST_P_ENCRYPTED).unwrap();
        let decrypted = aes_decrypt(&k1, &p_bytes).unwrap();

        // Step 2: Parse
        let (uid, counter) = parse_decrypted_data(&decrypted).unwrap();

        // Step 3: Verify CMAC
        let k2 = AesKey::from_hex(TEST_K2_CMAC_KEY).unwrap();
        let c_bytes = hex::decode(TEST_C_CMAC).unwrap();
        let cmac_result = verify_cmac(&k2, &uid, &counter, &c_bytes).unwrap();

        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            TEST_P_ENCRYPTED,
            TEST_C_CMAC,
        );

        match result {
            Ok(ValidationResult { uid, counter }) => {
                // Verify the UID was extracted correctly
                assert_eq!(uid.to_string(), "04996c6a926980");

                // Verify the counter was extracted correctly
                // The counter should be greater than 0 (the last_counter from DB)
                assert!(counter.value() > 0);

                println!("Validation successful!");
                println!("UID: {}", uid);
                println!("Counter: {}", counter);
            }
            Err(msg) => {
                panic!("Validation failed with error: {}", msg);
            }
        }
    }

    #[test]
    fn test_validation_invalid_hex_parameters() {
        // Test with invalid hex in p parameter
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            "invalid_hex",
            TEST_C_CMAC,
        );
        assert_eq!(result, Err("Invalid p parameter".to_string()));

        // Test with invalid hex in c parameter
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            TEST_P_ENCRYPTED,
            "invalid_hex",
        );
        assert_eq!(result, Err("Invalid c parameter".to_string()));
    }

    #[test]
    fn test_validation_wrong_parameter_lengths() {
        // Test with wrong length p parameter (should be 16 bytes = 32 hex chars)
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            "1234567890abcdef", // 16 hex chars = 8 bytes
            TEST_C_CMAC,
        );
        assert_eq!(result, Err("Invalid parameter length".to_string()));

        // Test with wrong length c parameter (should be 8 bytes = 16 hex chars)
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            TEST_P_ENCRYPTED,
            "12345678", // 8 hex chars = 4 bytes
        );
        assert_eq!(result, Err("Invalid parameter length".to_string()));
    }

    #[test]
    fn test_validation_invalid_keys() {
        // Test with invalid k1 key
        let result = validate_card_pure(
            "invalid_key",
            TEST_K2_CMAC_KEY,
            TEST_P_ENCRYPTED,
            TEST_C_CMAC,
        );
        assert_eq!(result, Err("Invalid k1 key".to_string()));

        // Test with invalid k2 key
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            "invalid_key",
            TEST_P_ENCRYPTED,
            TEST_C_CMAC,
        );
        assert_eq!(result, Err("Invalid k2 key".to_string()));
    }

    #[test]
    fn test_validation_wrong_cmac() {
        // Test with wrong CMAC
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            TEST_P_ENCRYPTED,
            "0000000000000000", // Wrong CMAC
        );
        assert_eq!(result, Err("Invalid CMAC - card authentication failed".to_string()));
    }

    #[test]
    fn test_validation_wrong_encrypted_data() {
        // Test with wrong encrypted data
        let result = validate_card_pure(
            TEST_K1_DECRYPT_KEY,
            TEST_K2_CMAC_KEY,
            "00000000000000000000000000000000", // Wrong encrypted data
            TEST_C_CMAC,
        );
        // This should fail either at decryption or CMAC verification
        assert!(result.is_err());
    }
}
