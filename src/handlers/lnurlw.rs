use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::{
    app_state::AppState,
    crypto::{AesKey, aes_decrypt, verify_cmac, parse_decrypted_data},
    db::queries,
};

#[derive(Debug, Deserialize)]
pub struct LnurlwParams {
    p: String,  // encrypted UID + counter
    c: String,  // CMAC
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LnurlwResponse {
    pub status: String,
    pub callback: String,
    pub k1: String,
    pub default_description: String,
    pub min_withdrawable: u64,
    pub max_withdrawable: u64,
    pub tag: String,
}

#[derive(Debug, Serialize)]
pub struct LnurlwError {
    pub status: String,
    pub reason: String,
}

/// GET /ln?p={encrypted}&c={cmac}
/// LNURLw endpoint that validates card and returns withdrawal info
pub async fn lnurlw_request(
    Query(params): Query<LnurlwParams>,
    State(state): State<AppState>,
) -> Result<Json<LnurlwResponse>, (StatusCode, Json<LnurlwError>)> {
    // Decode hex parameters
    let p_bytes = hex::decode(&params.p)
        .map_err(|_| error_response("Invalid p parameter"))?;
    let c_bytes = hex::decode(&params.c)
        .map_err(|_| error_response("Invalid c parameter"))?;
    
    if p_bytes.len() != 16 || c_bytes.len() != 8 {
        return Err(error_response("Invalid parameter length"));
    }
    
    // Try to find the card by decrypting with each card's k1
    let cards = sqlx::query_as::<_, crate::db::models::Card>(
        "SELECT * FROM cards WHERE enabled = 1"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| error_response("Database error"))?;
    
    for card in cards {
        // Try to decrypt with this card's k1
        let k1 = AesKey::from_hex(&card.k1_decrypt_key)
            .map_err(|_| error_response("Invalid card key"))?;
        
        let decrypted = aes_decrypt(&k1, &p_bytes)
            .map_err(|_| error_response("Decryption failed"))?;
        
        // Parse UID and counter
        let (uid, counter) = parse_decrypted_data(&decrypted)
            .map_err(|_| error_response("Invalid decrypted data"))?;
        
        // Verify CMAC with this card's k2
        let k2 = AesKey::from_hex(&card.k2_cmac_key)
            .map_err(|_| error_response("Invalid card key"))?;
        
        if verify_cmac(&k2, &uid, &counter, &c_bytes).unwrap_or(false) {
            // CMAC verified! This is the right card
            
            // Update UID if not set
            if card.uid.is_empty() {
                sqlx::query("UPDATE cards SET uid = ? WHERE card_id = ?")
                    .bind(uid.to_string())
                    .bind(card.card_id)
                    .execute(&state.pool)
                    .await
                    .map_err(|_| error_response("Database error"))?;
            } else if card.uid != uid.to_string() {
                return Err(error_response("UID mismatch"));
            }
            
            // Check and update counter (replay protection)
            if counter.value() as i64 <= card.last_counter {
                return Err(error_response("Invalid counter - possible replay attack"));
            }
            
            let updated = queries::update_card_counter(&state.pool, card.card_id, counter.value() as i64)
                .await
                .map_err(|_| error_response("Database error"))?;
            
            if !updated {
                return Err(error_response("Counter update failed"));
            }
            
            // Generate k1 for this withdrawal session
            let withdrawal_k1 = hex::encode(rand::random::<[u8; 16]>());
            
            // Create payment record
            queries::create_payment(&state.pool, card.card_id, &withdrawal_k1)
                .await
                .map_err(|_| error_response("Database error"))?;
            
            // Calculate actual withdrawable amount (respecting limits)
            let daily_spent_msats = queries::get_daily_total_msats(&state.pool, card.card_id)
                .await
                .unwrap_or(0);
            let daily_remaining_sats = (card.day_limit_sats * 1000 - daily_spent_msats) / 1000;
            let max_withdrawable_sats = std::cmp::min(card.tx_limit_sats, daily_remaining_sats);
            
            let response = LnurlwResponse {
                status: "OK".to_string(),
                callback: format!("https://{}/ln/callback", state.config.domain),
                k1: withdrawal_k1,
                default_description: format!("Withdrawal from {}", card.card_name),
                min_withdrawable: 1000,  // 1 sat in millisats
                max_withdrawable: (max_withdrawable_sats * 1000) as u64,  // Convert to millisats
                tag: "withdrawRequest".to_string(),
            };
            
            return Ok(Json(response));
        }
    }
    
    Err(error_response("Card not found or invalid"))
}

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    k1: String,
    pr: String,  // Lightning invoice
}

#[derive(Debug, Serialize)]
pub struct CallbackResponse {
    pub status: String,
}

/// GET /ln/callback?k1={k1}&pr={invoice}
/// Process withdrawal with Lightning invoice
pub async fn lnurlw_callback(
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
) -> Result<Json<CallbackResponse>, (StatusCode, Json<LnurlwError>)> {
    use std::str::FromStr;
    
    // Get payment record by k1
    let payment = queries::get_payment_by_k1(&state.pool, &params.k1)
        .await
        .map_err(|_| error_response("Database error"))?
        .ok_or_else(|| error_response("Invalid k1"))?;
    
    if payment.paid.unwrap_or(false) {
        return Err(error_response("Payment already processed"));
    }
    
    // Parse and validate invoice
    let invoice = crate::lightning::Invoice::from_str(&params.pr)
        .map_err(|_| error_response("Invalid invoice"))?;
    
    let amount_msats = invoice.amount_msats()
        .map_err(|_| error_response("Invoice must have amount"))?;
    
    // Get card to check limits
    let card = sqlx::query_as::<_, crate::db::models::Card>(
        "SELECT * FROM cards WHERE card_id = ?"
    )
    .bind(payment.card_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| error_response("Database error"))?;
    
    // Check transaction limit
    if amount_msats > (card.tx_limit_sats * 1000) as u64 {
        return Err(error_response("Amount exceeds transaction limit"));
    }
    
    // Check daily limit
    let daily_spent_msats = queries::get_daily_total_msats(&state.pool, card.card_id)
        .await
        .unwrap_or(0);
    
    if (daily_spent_msats + amount_msats as i64) > (card.day_limit_sats * 1000) {
        return Err(error_response("Amount exceeds daily limit"));
    }
    
    // Update payment with invoice details
    queries::update_payment_with_invoice(&state.pool, payment.payment_id, &params.pr, amount_msats as i64)
        .await
        .map_err(|_| error_response("Database error"))?;
    
    // Pay the invoice
    let payment_result = state.lightning.pay_invoice(&invoice, amount_msats)
        .await
        .map_err(|e| error_response(&format!("Payment failed: {}", e)))?;
    
    if !payment_result.success {
        return Err(error_response(&payment_result.error.unwrap_or_else(|| "Payment failed".to_string())));
    }
    
    // Mark payment as paid
    queries::mark_payment_paid(&state.pool, payment.payment_id)
        .await
        .map_err(|_| error_response("Database error"))?;
    
    Ok(Json(CallbackResponse {
        status: "OK".to_string(),
    }))
}

fn error_response(reason: &str) -> (StatusCode, Json<LnurlwError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(LnurlwError {
            status: "ERROR".to_string(),
            reason: reason.to_string(),
        })
    )
}