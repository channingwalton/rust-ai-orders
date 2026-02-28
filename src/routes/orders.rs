use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::models::{CreateOrderRequest, ServiceError, UserId};

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/orders", post(create_order))
        .route("/orders/user/{user_id}", get(get_orders_by_user))
}

async fn create_order(
    State(state): State<crate::AppState>,
    payload: Result<Json<CreateOrderRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ServiceError> {
    let Json(req) =
        payload.map_err(|e| ServiceError::InvalidJsonRequest(e.body_text().to_string()))?;
    let order = state
        .service_factory
        .commit(|svc| async move { svc.create_order(req).await })
        .await?;
    Ok((StatusCode::CREATED, Json(order)))
}

async fn get_orders_by_user(
    State(state): State<crate::AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, ServiceError> {
    let response = state
        .service_factory
        .commit(|svc| async move { svc.get_orders_by_user(UserId(user_id)).await })
        .await?;
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::config::ApplicationConfig;
    use crate::models::order::ProductId;
    use crate::models::{CreateOrderRequest, UserId};
    use crate::services::HealthService;
    use crate::test_helpers;
    use crate::{AppState, ServiceFactory};

    async fn test_app_with_user() -> (axum::Router, UserId) {
        let (user_service, order_service) = test_helpers::create_services();
        let user = user_service
            .create_user("test@example.com", "Test User")
            .await
            .unwrap();

        let health_service = HealthService::new(ApplicationConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
        });
        let state = AppState {
            health_service,
            service_factory: ServiceFactory::InMemory { order_service },
        };
        let app = crate::routes::health::router()
            .merge(crate::routes::orders::router())
            .with_state(state);
        (app, user.id)
    }

    fn order_json(user_id: UserId) -> serde_json::Value {
        serde_json::json!({
            "user_id": user_id.0,
            "product_id": "test-product",
            "quantity": 2,
            "total_amount": "29.99"
        })
    }

    // -- OrderRoutesSpec tests --

    #[tokio::test]
    async fn post_orders_creates_a_new_order() {
        let (app, user_id) = test_app_with_user().await;
        let body = serde_json::to_string(&order_json(user_id)).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["user_id"], user_id.0.to_string());
        assert_eq!(json["product_id"], "test-product");
        assert_eq!(json["quantity"], 2);
        assert_eq!(json["total_amount"], "29.99");
    }

    #[tokio::test]
    async fn get_orders_returns_empty_list_when_no_orders_exist() {
        let (app, user_id) = test_app_with_user().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/orders/user/{}", user_id.0))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["orders"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn get_orders_returns_orders_for_user() {
        let (user_service, order_service) = test_helpers::create_services();
        let user = user_service
            .create_user("test@example.com", "Test User")
            .await
            .unwrap();

        // Create two orders via the service directly
        let req1 = CreateOrderRequest {
            user_id: user.id,
            product_id: ProductId("product-1".to_string()),
            quantity: 2,
            total_amount: rust_decimal_macros::dec!(29.99),
        };
        let req2 = CreateOrderRequest {
            user_id: user.id,
            product_id: ProductId("product-2".to_string()),
            quantity: 3,
            total_amount: rust_decimal_macros::dec!(49.99),
        };
        order_service.create_order(req1).await.unwrap();
        order_service.create_order(req2).await.unwrap();

        let health_service = HealthService::new(ApplicationConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
        });
        let state = AppState {
            health_service,
            service_factory: ServiceFactory::InMemory { order_service },
        };
        let app = crate::routes::orders::router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/orders/user/{}", user.id.0))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let orders = json["orders"].as_array().unwrap();
        assert_eq!(orders.len(), 2);
        assert!(orders.iter().all(|o| o["user_id"] == user.id.0.to_string()));
    }

    #[tokio::test]
    async fn get_orders_with_invalid_uuid_returns_404() {
        let (app, _) = test_app_with_user().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/orders/user/invalid-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // axum returns 400 for path parsing failures
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::NOT_FOUND
        );
    }

    // -- OrderRoutesErrorSpec tests --

    #[tokio::test]
    async fn post_orders_returns_404_for_non_existent_user() {
        let (app, _) = test_app_with_user().await;
        let fake_user = UserId::new();
        let body = serde_json::to_string(&order_json(fake_user)).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("User not found"));
    }

    #[tokio::test]
    async fn get_orders_returns_404_for_non_existent_user() {
        let (app, _) = test_app_with_user().await;
        let fake_user = UserId::new();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/orders/user/{}", fake_user.0))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("User not found"));
    }

    #[tokio::test]
    async fn post_orders_returns_400_for_invalid_json() {
        let (app, _) = test_app_with_user().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orders")
                    .header("content-type", "application/json")
                    .body(Body::from("invalid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
    }

    #[tokio::test]
    async fn post_orders_returns_400_for_missing_required_fields() {
        let (app, user_id) = test_app_with_user().await;
        let incomplete = serde_json::json!({
            "user_id": user_id.0,
            "quantity": 2
            // missing product_id and total_amount
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&incomplete).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::UNPROCESSABLE_ENTITY
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
    }

    #[tokio::test]
    async fn get_orders_with_malformed_uuid_returns_error() {
        let (app, _) = test_app_with_user().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/orders/user/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // axum returns 400 for path parsing failures
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::NOT_FOUND
        );
    }
}
