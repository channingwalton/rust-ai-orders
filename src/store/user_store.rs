use async_trait::async_trait;
use sqlx::PgPool;

use super::UserRepository;
use crate::models::{User, UserId};

#[derive(Clone)]
pub struct PgUserStore {
    pool: PgPool,
}

impl PgUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserStore {
    async fn create(&self, user: &User) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO users (id, email, name, created_at) VALUES ($1, $2, $3, $4)")
            .bind(user.id.0)
            .bind(&user.email)
            .bind(&user.name)
            .bind(user.created_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: UserId) -> anyhow::Result<Option<User>> {
        Ok(
            sqlx::query_as::<_, User>(
                "SELECT id, email, name, created_at FROM users WHERE id = $1",
            )
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await?,
        )
    }

    async fn find_by_email(&self, email: &str) -> anyhow::Result<Option<User>> {
        Ok(
            sqlx::query_as::<_, User>(
                "SELECT id, email, name, created_at FROM users WHERE email = $1",
            )
            .bind(email)
            .fetch_optional(&self.pool)
            .await?,
        )
    }

    async fn exists(&self, id: UserId) -> anyhow::Result<bool> {
        let row: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(id.0)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn update(&self, user: &User) -> anyhow::Result<()> {
        sqlx::query("UPDATE users SET email = $1, name = $2 WHERE id = $3")
            .bind(&user.email)
            .bind(&user.name)
            .bind(user.id.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete(&self, id: UserId) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{SubsecRound, Utc};
    use sqlx::postgres::PgPoolOptions;
    use testcontainers_modules::postgres::Postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    async fn setup() -> (impl std::any::Any, PgUserStore) {
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
        (container, PgUserStore::new(pool))
    }

    fn test_user(email: &str, name: &str) -> User {
        User {
            id: UserId::new(),
            email: email.to_string(),
            name: name.to_string(),
            created_at: Utc::now().trunc_subsecs(0),
        }
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn create_and_find_user_by_id() {
        let (_container, store) = setup().await;
        let user = test_user("test@example.com", "Test User");

        store.create(&user).await.unwrap();
        let found = store.find_by_id(user.id).await.unwrap();

        assert_eq!(found, Some(user));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn find_user_by_email() {
        let (_container, store) = setup().await;
        let user = test_user("email@example.com", "Email User");

        store.create(&user).await.unwrap();
        let found = store.find_by_email(&user.email).await.unwrap();

        assert_eq!(found, Some(user));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn user_exists() {
        let (_container, store) = setup().await;
        let user = test_user("exists@example.com", "Exists User");

        let exists_before = store.exists(user.id).await.unwrap();
        store.create(&user).await.unwrap();
        let exists_after = store.exists(user.id).await.unwrap();

        assert!(!exists_before);
        assert!(exists_after);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn update_user() {
        let (_container, store) = setup().await;
        let user = test_user("update@example.com", "Update User");

        store.create(&user).await.unwrap();
        let updated = User {
            name: "Updated Name".to_string(),
            email: "updated@example.com".to_string(),
            ..user
        };
        store.update(&updated).await.unwrap();
        let found = store.find_by_id(user.id).await.unwrap();

        assert_eq!(found, Some(updated));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn delete_user() {
        let (_container, store) = setup().await;
        let user = test_user("delete@example.com", "Delete User");

        store.create(&user).await.unwrap();
        let found_before = store.find_by_id(user.id).await.unwrap();
        store.delete(user.id).await.unwrap();
        let found_after = store.find_by_id(user.id).await.unwrap();

        assert_eq!(found_before, Some(user));
        assert_eq!(found_after, None);
    }
}
