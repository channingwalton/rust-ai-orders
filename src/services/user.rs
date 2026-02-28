use std::sync::Arc;

use chrono::Utc;

use crate::models::{ServiceError, User, UserId};
use crate::store::UserRepository;

#[derive(Clone)]
pub struct UserService {
    store: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(store: Arc<dyn UserRepository>) -> Self {
        Self { store }
    }

    pub async fn create_user(&self, email: &str, name: &str) -> Result<User, ServiceError> {
        let user = User {
            id: UserId::new(),
            email: email.to_string(),
            name: name.to_string(),
            created_at: Utc::now(),
        };
        self.store
            .create(&user)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: UserId) -> Result<User, ServiceError> {
        self.store
            .find_by_id(id)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?
            .ok_or(ServiceError::UserNotFound(id))
    }

    pub async fn user_exists(&self, id: UserId) -> Result<bool, ServiceError> {
        self.store
            .exists(id)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))
    }

    pub async fn get_user(&self, id: UserId) -> Result<Option<User>, ServiceError> {
        self.store
            .find_by_id(id)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    #[tokio::test]
    async fn user_exists_returns_false_for_non_existent_user() {
        let (user_service, _) = test_helpers::create_services();
        let result = user_service.user_exists(UserId::new()).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn create_user_creates_a_new_user() {
        let (user_service, _) = test_helpers::create_services();
        let user = user_service
            .create_user("test@example.com", "Test User")
            .await
            .unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Test User");
    }

    #[tokio::test]
    async fn user_exists_returns_true_for_existing_user() {
        let (user_service, _) = test_helpers::create_services();
        let user = user_service
            .create_user("test@example.com", "Test User")
            .await
            .unwrap();
        let exists = user_service.user_exists(user.id).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn get_user_returns_none_for_non_existent_user() {
        let (user_service, _) = test_helpers::create_services();
        let result = user_service.get_user(UserId::new()).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn get_user_returns_some_for_existing_user() {
        let (user_service, _) = test_helpers::create_services();
        let created = user_service
            .create_user("test@example.com", "Test User")
            .await
            .unwrap();
        let found = user_service.get_user(created.id).await.unwrap();
        assert_eq!(found, Some(created));
    }
}
