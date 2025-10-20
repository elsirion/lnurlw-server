use sqlx::{Pool, Sqlite};
use anyhow::Result;
use crate::{
    db::models::Card,
    validation::CardRepository,
};

/// Database implementation of CardRepository
pub struct DatabaseCardRepository {
    pool: Pool<Sqlite>,
}

impl DatabaseCardRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CardRepository for DatabaseCardRepository {
    async fn get_card_by_id(&self, card_id: i64) -> Result<Option<Card>> {
        let card = sqlx::query_as::<_, Card>(
            "SELECT * FROM cards WHERE card_id = ? AND enabled = 1"
        )
        .bind(card_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(card)
    }

    async fn update_card_uid(&self, card_id: i64, uid: &str) -> Result<()> {
        sqlx::query("UPDATE cards SET uid = ? WHERE card_id = ?")
            .bind(uid)
            .bind(card_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_card_counter(&self, card_id: i64, counter: i64) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE cards SET last_counter = ? WHERE card_id = ? AND last_counter < ?"
        )
        .bind(counter)
        .bind(card_id)
        .bind(counter)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
