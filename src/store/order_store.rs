use async_trait::async_trait;
use sqlx::PgPool;

use super::OrderRepository;
use crate::models::{Order, OrderId, UserId};

#[derive(Clone)]
pub struct PgOrderStore {
    pool: PgPool,
}

impl PgOrderStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderRepository for PgOrderStore {
    async fn insert(&self, order: &Order) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO orders (id, user_id, product_id, quantity, total_amount, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(order.id.0)
        .bind(order.user_id.0)
        .bind(&order.product_id.0)
        .bind(order.quantity)
        .bind(order.total_amount)
        .bind(order.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: OrderId) -> anyhow::Result<Option<Order>> {
        Ok(sqlx::query_as::<_, Order>(
            "SELECT id, user_id, product_id, quantity, total_amount, created_at \
             FROM orders WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn find_by_user_id(&self, user_id: UserId) -> anyhow::Result<Vec<Order>> {
        Ok(sqlx::query_as::<_, Order>(
            "SELECT id, user_id, product_id, quantity, total_amount, created_at \
             FROM orders WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn exists(&self, id: OrderId) -> anyhow::Result<bool> {
        let row: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM orders WHERE id = $1)")
            .bind(id.0)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn update(&self, order: &Order) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE orders SET product_id = $1, quantity = $2, total_amount = $3 WHERE id = $4",
        )
        .bind(&order.product_id.0)
        .bind(order.quantity)
        .bind(order.total_amount)
        .bind(order.id.0)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: OrderId) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM orders WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::order::ProductId;
    use crate::models::User;
    use crate::store::user_store::PgUserStore;
    use crate::store::UserRepository;
    use chrono::{SubsecRound, Utc};
    use rust_decimal_macros::dec;
    use sqlx::postgres::PgPoolOptions;
    use testcontainers_modules::postgres::Postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    async fn setup() -> (impl std::any::Any, PgUserStore, PgOrderStore) {
        let container = Postgres::default().start().await.unwrap();
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (
            container,
            PgUserStore::new(pool.clone()),
            PgOrderStore::new(pool),
        )
    }

    fn test_user(email: &str) -> User {
        User {
            id: UserId::new(),
            email: email.to_string(),
            name: "Test User".to_string(),
            created_at: Utc::now().trunc_subsecs(0),
        }
    }

    fn test_order(user_id: UserId, product: &str, qty: i32, amount: rust_decimal::Decimal) -> Order {
        Order {
            id: OrderId::new(),
            user_id,
            product_id: ProductId(product.to_string()),
            quantity: qty,
            total_amount: amount,
            created_at: Utc::now().trunc_subsecs(0),
        }
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn create_and_find_order_by_id() {
        let (_container, user_store, order_store) = setup().await;
        let user = test_user("test@example.com");
        user_store.create(&user).await.unwrap();

        let order = test_order(user.id, "test-product", 2, dec!(29.99));
        order_store.insert(&order).await.unwrap();
        let found = order_store.find_by_id(order.id).await.unwrap();

        assert_eq!(found, Some(order));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn find_orders_by_user_id() {
        let (_container, user_store, order_store) = setup().await;
        let user = test_user("orders@example.com");
        user_store.create(&user).await.unwrap();

        let order1 = test_order(user.id, "product-1", 1, dec!(19.99));
        let order2 = test_order(user.id, "product-2", 3, dec!(59.97));
        order_store.insert(&order1).await.unwrap();
        order_store.insert(&order2).await.unwrap();

        let orders = order_store.find_by_user_id(user.id).await.unwrap();
        assert_eq!(orders.len(), 2);
        assert!(orders.contains(&order1));
        assert!(orders.contains(&order2));
        // Should be sorted DESC by created_at
        assert!(orders[0].created_at >= orders[1].created_at);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn order_exists() {
        let (_container, user_store, order_store) = setup().await;
        let user = test_user("exists@example.com");
        user_store.create(&user).await.unwrap();

        let order = test_order(user.id, "exists-product", 1, dec!(9.99));
        let exists_before = order_store.exists(order.id).await.unwrap();
        order_store.insert(&order).await.unwrap();
        let exists_after = order_store.exists(order.id).await.unwrap();

        assert!(!exists_before);
        assert!(exists_after);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn update_order() {
        let (_container, user_store, order_store) = setup().await;
        let user = test_user("update@example.com");
        user_store.create(&user).await.unwrap();

        let order = test_order(user.id, "update-product", 1, dec!(19.99));
        order_store.insert(&order).await.unwrap();

        let updated = Order {
            product_id: ProductId("updated-product".to_string()),
            quantity: 5,
            total_amount: dec!(99.95),
            ..order.clone()
        };
        order_store.update(&updated).await.unwrap();
        let found = order_store.find_by_id(order.id).await.unwrap();

        assert_eq!(found, Some(updated));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn delete_order() {
        let (_container, user_store, order_store) = setup().await;
        let user = test_user("delete@example.com");
        user_store.create(&user).await.unwrap();

        let order = test_order(user.id, "delete-product", 1, dec!(14.99));
        order_store.insert(&order).await.unwrap();

        let found_before = order_store.find_by_id(order.id).await.unwrap();
        order_store.delete(order.id).await.unwrap();
        let found_after = order_store.find_by_id(order.id).await.unwrap();

        assert_eq!(found_before, Some(order));
        assert_eq!(found_after, None);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn find_orders_by_user_id_returns_empty_for_user_with_no_orders() {
        let (_container, user_store, order_store) = setup().await;
        let user = test_user("noorders@example.com");
        user_store.create(&user).await.unwrap();

        let orders = order_store.find_by_user_id(user.id).await.unwrap();
        assert!(orders.is_empty());
    }
}
