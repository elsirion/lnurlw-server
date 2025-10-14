use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use crate::{config::Config, lightning::LightningBackend};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Sqlite>,
    pub config: Arc<Config>,
    pub lightning: Arc<dyn LightningBackend>,
}