use std::sync::Arc;

use chrono::Utc;

use crate::models::{CreateOrderRequest, Order, OrderId, OrderListResponse, ServiceError, UserId};
use crate::store::OrderRepository;

use super::user::UserService;

#[derive(Clone)]
pub struct OrderService {
    order_store: Arc<dyn OrderRepository>,
    user_service: UserService,
}

impl OrderService {
    pub fn new(order_store: Arc<dyn OrderRepository>, user_service: UserService) -> Self {
        Self {
            order_store,
            user_service,
        }
    }

    pub async fn create_order(&self, req: CreateOrderRequest) -> Result<Order, ServiceError> {
        // Validate user exists
        if !self.user_service.user_exists(req.user_id).await? {
            return Err(ServiceError::UserNotFound(req.user_id));
        }

        let now = Utc::now();
        let order = Order {
            id: OrderId::new(),
            user_id: req.user_id,
            product_id: req.product_id,
            quantity: req.quantity,
            total_amount: req.total_amount,
            created_at: now,
        };

        self.order_store
            .insert(&order)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        Ok(order)
    }

    pub async fn get_orders_by_user(
        &self,
        user_id: UserId,
    ) -> Result<OrderListResponse, ServiceError> {
        // Validate user exists
        if !self.user_service.user_exists(user_id).await? {
            return Err(ServiceError::UserNotFound(user_id));
        }

        let orders = self
            .order_store
            .find_by_user_id(user_id)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        Ok(OrderListResponse { orders })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::order::ProductId;
    use crate::test_helpers;
    use rust_decimal_macros::dec;

    fn test_request(user_id: UserId) -> CreateOrderRequest {
        CreateOrderRequest {
            user_id,
            product_id: ProductId("test-product".to_string()),
            quantity: 2,
            total_amount: dec!(29.99),
        }
    }

    async fn setup() -> (UserService, OrderService, UserId) {
        let (user_service, order_service) = test_helpers::create_services();
        let user = user_service
            .create_user("test@example.com", "Test User")
            .await
            .unwrap();
        (user_service, order_service, user.id)
    }

    #[tokio::test]
    async fn create_order_with_existing_user() {
        let (_, order_service, user_id) = setup().await;
        let order = order_service
            .create_order(test_request(user_id))
            .await
            .unwrap();
        assert_eq!(order.user_id, user_id);
        assert_eq!(order.product_id, ProductId("test-product".to_string()));
        assert_eq!(order.quantity, 2);
        assert_eq!(order.total_amount, dec!(29.99));
    }

    #[tokio::test]
    async fn create_order_fails_for_non_existent_user() {
        let (_, order_service, _) = setup().await;
        let fake_user = UserId::new();
        let result = order_service.create_order(test_request(fake_user)).await;
        assert!(matches!(result, Err(ServiceError::UserNotFound(id)) if id == fake_user));
    }

    #[tokio::test]
    async fn get_orders_returns_empty_list_for_user_with_no_orders() {
        let (_, order_service, user_id) = setup().await;
        let result = order_service.get_orders_by_user(user_id).await.unwrap();
        assert!(result.orders.is_empty());
    }

    #[tokio::test]
    async fn get_orders_fails_for_non_existent_user() {
        let (_, order_service, _) = setup().await;
        let fake_user = UserId::new();
        let result = order_service.get_orders_by_user(fake_user).await;
        assert!(matches!(result, Err(ServiceError::UserNotFound(id)) if id == fake_user));
    }

    #[tokio::test]
    async fn get_orders_returns_orders_for_specific_user() {
        let (user_service, order_service, user1_id) = setup().await;
        let user2 = user_service
            .create_user("user2@example.com", "User 2")
            .await
            .unwrap();

        let order1 = order_service
            .create_order(test_request(user1_id))
            .await
            .unwrap();
        let order2 = order_service
            .create_order(test_request(user2.id))
            .await
            .unwrap();
        let order3 = order_service
            .create_order(test_request(user1_id))
            .await
            .unwrap();

        let user1_orders = order_service.get_orders_by_user(user1_id).await.unwrap();
        let user2_orders = order_service.get_orders_by_user(user2.id).await.unwrap();

        assert_eq!(user1_orders.orders.len(), 2);
        assert_eq!(user2_orders.orders.len(), 1);
        assert!(user1_orders.orders.contains(&order1));
        assert!(user1_orders.orders.contains(&order3));
        assert!(user2_orders.orders.contains(&order2));
    }

    #[tokio::test]
    async fn get_orders_returns_sorted_by_creation_time_newest_first() {
        let (_, order_service, user_id) = setup().await;

        let order1 = order_service
            .create_order(test_request(user_id))
            .await
            .unwrap();
        // Small delay to ensure different timestamps
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        let order2 = order_service
            .create_order(test_request(user_id))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        let order3 = order_service
            .create_order(test_request(user_id))
            .await
            .unwrap();

        let orders = order_service.get_orders_by_user(user_id).await.unwrap();
        assert_eq!(orders.orders.len(), 3);
        assert_eq!(orders.orders[0], order3);
        assert_eq!(orders.orders[1], order2);
        assert_eq!(orders.orders[2], order1);
    }
}
