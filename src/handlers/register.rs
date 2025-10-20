use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::{
    app_state::AppState,
    crypto::AesKey,
    db::{models::{CreateCardRequest, CardRegistrationResponse}, queries},
};

#[derive(Debug, Deserialize)]
pub struct NewCardQuery {
    a: String,  // one-time authentication code
}

/// GET /new?a={one_time_code}
/// Returns card configuration for NFC programming
pub async fn get_card_registration(
    Query(params): Query<NewCardQuery>,
    State(state): State<AppState>,
) -> Result<Json<CardRegistrationResponse>, StatusCode> {
    let card = queries::get_card_by_one_time_code(&state.pool, &params.a)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Mark the one-time code as used
    queries::mark_one_time_code_used(&state.pool, card.card_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = CardRegistrationResponse {
        protocol_name: "create_bolt_card_response".to_string(),
        protocol_version: 2,
        card_name: card.card_name,
        lnurlw_base: state.config.lnurlw_base_with_card_id(card.card_id),
        k0: card.k0_auth_key,
        k1: card.k1_decrypt_key,
        k2: card.k2_cmac_key,
        k3: card.k3,
        k4: card.k4,
    };

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct CreateCardResponse {
    pub status: String,
    pub url: String,
}

/// POST /api/createboltcard
/// Creates a new card with random keys
pub async fn create_card(
    State(state): State<AppState>,
    Json(req): Json<CreateCardRequest>,
) -> Result<Json<CreateCardResponse>, StatusCode> {
    // Generate all keys
    let k0 = AesKey::generate();
    let k1 = AesKey::generate();
    let k2 = AesKey::generate();
    let k3 = AesKey::generate();
    let k4 = AesKey::generate();

    // Generate one-time code
    let one_time_code = hex::encode(rand::random::<[u8; 16]>());

    // Use defaults from config if not specified
    let tx_limit = req.tx_limit_sats.unwrap_or(state.config.default_tx_limit as i64);
    let day_limit = req.day_limit_sats.unwrap_or(state.config.default_day_limit as i64);
    let enabled = req.enabled.unwrap_or(true);

    // Insert card into database (UID will be set on first use)
    queries::insert_card(
        &state.pool,
        "",  // UID empty initially
        &k0.to_string(),
        &k1.to_string(),
        &k2.to_string(),
        &k3.to_string(),
        &k4.to_string(),
        &req.card_name,
        tx_limit,
        day_limit,
        enabled,
        &one_time_code,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let url = format!("{}?a={}", state.config.registration_base(), one_time_code);

    Ok(Json(CreateCardResponse {
        status: "OK".to_string(),
        url,
    }))
}