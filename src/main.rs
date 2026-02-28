use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod config;
mod db;
mod models;
mod routes;
mod services;
mod store;

#[cfg(test)]
mod test_helpers;

use config::AppConfig;
use db::DbConn;
use models::ServiceError;
use services::{HealthService, OrderService, UserService};
use store::{PgOrderStore, PgUserStore};

#[derive(Clone)]
pub struct AppState {
    pub health_service: HealthService,
    pub service_factory: ServiceFactory,
}

/// Creates services within a transactional boundary, mirroring Scala's
/// `store.commit(orderService.createOrder(request))` pattern.
#[derive(Clone)]
pub enum ServiceFactory {
    Pg(PgPool),
    InMemory { order_service: OrderService },
}

impl ServiceFactory {
    /// Execute a service operation within a database transaction.
    /// For `Pg`, begins a transaction, creates stores and services bound to it,
    /// runs the closure, and commits. For `InMemory`, delegates directly.
    ///
    /// If the closure returns `Err`, the early return skips `commit()` and the
    /// transaction is automatically rolled back when dropped (sqlx semantics).
    pub async fn commit<T, F, Fut>(&self, f: F) -> Result<T, ServiceError>
    where
        F: FnOnce(OrderService) -> Fut,
        Fut: Future<Output = Result<T, ServiceError>>,
    {
        match self {
            Self::Pg(pool) => {
                let db = DbConn::begin(pool).await?;
                let user_store: Arc<dyn store::UserRepository> =
                    Arc::new(PgUserStore::new(db.clone()));
                let order_store: Arc<dyn store::OrderRepository> =
                    Arc::new(PgOrderStore::new(db.clone()));
                let user_service = UserService::new(user_store);
                let order_service = OrderService::new(order_store, user_service);
                let result = f(order_service).await?;
                db.commit().await?;
                Ok(result)
            }
            Self::InMemory { order_service } => f(order_service.clone()).await,
        }
    }
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
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let health_service = HealthService::new(config.application.clone());

    let state = AppState {
        health_service,
        service_factory: ServiceFactory::Pg(pool),
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
