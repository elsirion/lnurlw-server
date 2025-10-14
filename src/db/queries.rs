use sqlx::{Pool, Sqlite};
use anyhow::Result;
use chrono;
use crate::db::models::{Card, CardPayment};

pub async fn get_card_by_uid(pool: &Pool<Sqlite>, uid: &str) -> Result<Option<Card>> {
    let card = sqlx::query_as::<_, Card>(
        "SELECT * FROM cards WHERE uid = ? AND enabled = 1"
    )
    .bind(uid)
    .fetch_optional(pool)
    .await?;
    
    Ok(card)
}

pub async fn get_card_by_one_time_code(pool: &Pool<Sqlite>, code: &str) -> Result<Option<Card>> {
    let card = sqlx::query_as::<_, Card>(
        "SELECT * FROM cards WHERE one_time_code = ? AND one_time_code_used = 0 
         AND one_time_code_expiry > datetime('now')"
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;
    
    Ok(card)
}

pub async fn mark_one_time_code_used(pool: &Pool<Sqlite>, card_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE cards SET one_time_code_used = 1 WHERE card_id = ?"
    )
    .bind(card_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn update_card_counter(pool: &Pool<Sqlite>, card_id: i64, counter: i64) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE cards SET last_counter = ? WHERE card_id = ? AND last_counter < ?"
    )
    .bind(counter)
    .bind(card_id)
    .bind(counter)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected() > 0)
}

pub async fn insert_card(
    pool: &Pool<Sqlite>,
    uid: &str,
    k0: &str,
    k1: &str,
    k2: &str,
    k3: &str,
    k4: &str,
    card_name: &str,
    tx_limit: i64,
    day_limit: i64,
    enabled: bool,
    one_time_code: &str,
) -> Result<i64> {
    // SQLite datetime in UTC format
    let expiry = chrono::Utc::now() + chrono::Duration::days(1);
    let expiry_str = expiry.format("%Y-%m-%d %H:%M:%S").to_string();
    
    let result = sqlx::query(
        "INSERT INTO cards (uid, k0_auth_key, k1_decrypt_key, k2_cmac_key, k3, k4, 
         card_name, tx_limit_sats, day_limit_sats, enabled, one_time_code, 
         one_time_code_expiry, one_time_code_used)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)"
    )
    .bind(uid)
    .bind(k0)
    .bind(k1)
    .bind(k2)
    .bind(k3)
    .bind(k4)
    .bind(card_name)
    .bind(tx_limit)
    .bind(day_limit)
    .bind(enabled)
    .bind(one_time_code)
    .bind(expiry_str)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

pub async fn create_payment(
    pool: &Pool<Sqlite>,
    card_id: i64,
    k1: &str,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO card_payments (card_id, k1) VALUES (?, ?)"
    )
    .bind(card_id)
    .bind(k1)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

pub async fn get_payment_by_k1(pool: &Pool<Sqlite>, k1: &str) -> Result<Option<CardPayment>> {
    let payment = sqlx::query_as::<_, CardPayment>(
        "SELECT * FROM card_payments WHERE k1 = ?"
    )
    .bind(k1)
    .fetch_optional(pool)
    .await?;
    
    Ok(payment)
}

pub async fn update_payment_with_invoice(
    pool: &Pool<Sqlite>,
    payment_id: i64,
    invoice: &str,
    amount_msats: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE card_payments SET invoice = ?, amount_msats = ? WHERE payment_id = ?"
    )
    .bind(invoice)
    .bind(amount_msats)
    .bind(payment_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn mark_payment_paid(pool: &Pool<Sqlite>, payment_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE card_payments SET paid = 1, payment_time = datetime('now') WHERE payment_id = ?"
    )
    .bind(payment_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn get_daily_total_msats(pool: &Pool<Sqlite>, card_id: i64) -> Result<i64> {
    let row: (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(amount_msats) FROM card_payments 
         WHERE card_id = ? AND paid = 1 AND payment_time >= datetime('now', '-1 day')"
    )
    .bind(card_id)
    .fetch_one(pool)
    .await?;
    
    Ok(row.0.unwrap_or(0))
}