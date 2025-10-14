use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Card {
    pub card_id: i64,
    pub uid: String,
    pub k0_auth_key: String,
    pub k1_decrypt_key: String,
    pub k2_cmac_key: String,
    pub k3: String,
    pub k4: String,
    pub last_counter: i64,
    pub enabled: bool,
    pub tx_limit_sats: i64,
    pub day_limit_sats: i64,
    pub card_name: String,
    pub one_time_code: Option<String>,
    pub one_time_code_expiry: Option<String>,
    pub one_time_code_used: Option<bool>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CardPayment {
    pub payment_id: i64,
    pub card_id: i64,
    pub k1: String,
    pub invoice: Option<String>,
    pub amount_msats: Option<i64>,
    pub paid: Option<bool>,
    pub payment_time: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCardRequest {
    pub card_name: String,
    pub tx_limit_sats: Option<i64>,
    pub day_limit_sats: Option<i64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRegistrationResponse {
    pub protocol_name: String,
    pub protocol_version: i32,
    pub card_name: String,
    pub lnurlw_base: String,
    pub k0: String,
    pub k1: String,
    pub k2: String,
    pub k3: String,
    pub k4: String,
}