use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod config;
mod models;
mod routes;
mod services;
mod store;

#[cfg(test)]
mod test_helpers;

use config::AppConfig;
use services::{HealthService, OrderService, UserService};
use store::{PgOrderStore, PgUserStore};

#[derive(Clone)]
pub struct AppState {
    pub health_service: HealthService,
    pub order_service: OrderService,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = AppConfig::load()?;

    tracing::info!(
        "Starting {} v{}",
        config.application.name,
        config.application.version
    );

    // Database setup
    let pool = PgPoolOptions::new()
        .max_connections(32)
        .connect(&config.database.url)
        .await?;

    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Build services
    let user_store = Arc::new(PgUserStore::new(pool.clone()));
    let order_store = Arc::new(PgOrderStore::new(pool.clone()));

    let health_service = HealthService::new(config.application.clone());
    let user_service = UserService::new(user_store);
    let order_service = OrderService::new(order_store, user_service);

    let state = AppState {
        health_service,
        order_service,
    };

    // Build router
    let app = axum::Router::new()
        .merge(routes::health::router())
        .merge(routes::orders::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::new(config.server.host.parse()?, config.server.port);
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Server listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
