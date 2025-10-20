mod app_state;
mod config;
mod crypto;
mod db;
mod handlers;
mod lightning;
mod validation;

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use app_state::AppState;
use config::Config;
use db::init_pool;
use handlers::{lnurlw, register};
use lightning::MockLightning;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lnurlw_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse configuration
    let config = Arc::new(Config::parse());

    // Initialize database
    let pool = init_pool(&config.database_url).await?;

    // Initialize Lightning backend (using mock for now)
    let lightning: Arc<dyn lightning::LightningBackend> = Arc::new(MockLightning);

    // Create shared state
    let state = AppState {
        pool,
        config: config.clone(),
        lightning,
    };

    // Build router
    let app = Router::new()
        // LNURLw endpoints
        .route("/ln", get(lnurlw::lnurlw_request))
        .route("/ln/callback", get(lnurlw::lnurlw_callback))
        // Card registration endpoints
        .route("/new", get(register::get_card_registration))
        .route("/api/createboltcard", post(register::create_card))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
        )
        // Add shared state
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&config.socket_addr()).await?;

    tracing::info!("Server running on {}", config.socket_addr());
    tracing::info!("Domain: {}", config.domain);
    tracing::info!("LNURLw base: {}", config.lnurlw_base());

    axum::serve(listener, app).await?;

    Ok(())
}
