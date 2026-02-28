use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use tokio::sync::Mutex;

use crate::models::ServiceError;

type SharedTx = Arc<Mutex<Option<Transaction<'static, Postgres>>>>;

/// Database connection that can be either a pool or a shared transaction.
///
/// When using a transaction, all stores sharing the same `DbConn` execute
/// within the same database transaction — providing the transaction boundary
/// at the route level, mirroring Scala's `store.commit(...)` pattern.
#[derive(Clone)]
pub enum DbConn {
    Pool(PgPool),
    Tx(SharedTx),
}

impl DbConn {
    /// Begin a new transaction from a pool.
    pub async fn begin(pool: &PgPool) -> Result<Self, ServiceError> {
        let tx = pool
            .begin()
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;
        Ok(Self::Tx(Arc::new(Mutex::new(Some(tx)))))
    }

    /// Commit the transaction. No-op for pool connections.
    pub async fn commit(&self) -> Result<(), ServiceError> {
        if let Self::Tx(tx) = self {
            let tx = tx
                .lock()
                .await
                .take()
                .ok_or_else(|| ServiceError::DatabaseError("transaction already committed".into()))?;
            tx.commit()
                .await
                .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    /// Execute a function with a database connection reference.
    /// For pool mode, acquires a connection; for tx mode, locks the shared transaction.
    pub async fn with_conn<T, F>(&self, f: F) -> anyhow::Result<T>
    where
        T: Send,
        F: for<'c> FnOnce(&'c mut PgConnection) -> Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'c>>,
    {
        match self {
            Self::Pool(pool) => {
                let mut conn = pool.acquire().await?;
                f(&mut conn).await
            }
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx_ref = guard.as_mut().expect("transaction already committed");
                f(tx_ref).await
            }
        }
    }
}
