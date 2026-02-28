use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::models::*;
use crate::services::{OrderService, UserService};
use crate::store::{OrderRepository, UserRepository};

// -- In-memory UserRepository --

#[derive(Clone, Default)]
pub struct InMemoryUserStore {
    users: Arc<RwLock<Vec<User>>>,
}

#[async_trait]
impl UserRepository for InMemoryUserStore {
    async fn create(&self, user: &User) -> anyhow::Result<()> {
        self.users.write().await.push(user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: UserId) -> anyhow::Result<Option<User>> {
        Ok(self.users.read().await.iter().find(|u| u.id == id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> anyhow::Result<Option<User>> {
        Ok(self
            .users
            .read()
            .await
            .iter()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn exists(&self, id: UserId) -> anyhow::Result<bool> {
        Ok(self.users.read().await.iter().any(|u| u.id == id))
    }

    async fn update(&self, user: &User) -> anyhow::Result<()> {
        let mut users = self.users.write().await;
        if let Some(existing) = users.iter_mut().find(|u| u.id == user.id) {
            *existing = user.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: UserId) -> anyhow::Result<()> {
        self.users.write().await.retain(|u| u.id != id);
        Ok(())
    }
}

// -- In-memory OrderRepository --

#[derive(Clone, Default)]
pub struct InMemoryOrderStore {
    orders: Arc<RwLock<Vec<Order>>>,
}

#[async_trait]
impl OrderRepository for InMemoryOrderStore {
    async fn insert(&self, order: &Order) -> anyhow::Result<()> {
        self.orders.write().await.push(order.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: OrderId) -> anyhow::Result<Option<Order>> {
        Ok(self
            .orders
            .read()
            .await
            .iter()
            .find(|o| o.id == id)
            .cloned())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> anyhow::Result<Vec<Order>> {
        let orders = self.orders.read().await;
        let mut result: Vec<Order> = orders
            .iter()
            .filter(|o| o.user_id == user_id)
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn exists(&self, id: OrderId) -> anyhow::Result<bool> {
        Ok(self.orders.read().await.iter().any(|o| o.id == id))
    }

    async fn update(&self, order: &Order) -> anyhow::Result<()> {
        let mut orders = self.orders.write().await;
        if let Some(existing) = orders.iter_mut().find(|o| o.id == order.id) {
            *existing = order.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: OrderId) -> anyhow::Result<()> {
        self.orders.write().await.retain(|o| o.id != id);
        Ok(())
    }
}

// -- Test setup helpers --

pub fn create_services() -> (UserService, OrderService) {
    let user_store: Arc<dyn UserRepository> = Arc::new(InMemoryUserStore::default());
    let order_store: Arc<dyn OrderRepository> = Arc::new(InMemoryOrderStore::default());

    let user_service = UserService::new(user_store);
    let order_service = OrderService::new(order_store, user_service.clone());

    (user_service, order_service)
}
