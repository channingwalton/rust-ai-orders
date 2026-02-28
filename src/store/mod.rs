pub mod order_store;
pub mod user_store;

use async_trait::async_trait;

use crate::models::{Order, OrderId, User, UserId};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> anyhow::Result<()>;
    async fn find_by_id(&self, id: UserId) -> anyhow::Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> anyhow::Result<Option<User>>;
    async fn exists(&self, id: UserId) -> anyhow::Result<bool>;
    async fn update(&self, user: &User) -> anyhow::Result<()>;
    async fn delete(&self, id: UserId) -> anyhow::Result<()>;
}

#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn insert(&self, order: &Order) -> anyhow::Result<()>;
    async fn find_by_id(&self, id: OrderId) -> anyhow::Result<Option<Order>>;
    async fn find_by_user_id(&self, user_id: UserId) -> anyhow::Result<Vec<Order>>;
    async fn exists(&self, id: OrderId) -> anyhow::Result<bool>;
    async fn update(&self, order: &Order) -> anyhow::Result<()>;
    async fn delete(&self, id: OrderId) -> anyhow::Result<()>;
}

pub use order_store::PgOrderStore;
pub use user_store::PgUserStore;
