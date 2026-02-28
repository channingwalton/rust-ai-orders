use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/health", get(health_check))
}

async fn health_check(State(state): State<crate::AppState>) -> Json<serde_json::Value> {
    let health = state.health_service.check();
    Json(serde_json::to_value(health).unwrap())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http::header::CONTENT_TYPE;
    use tower::ServiceExt;

    use crate::config::ApplicationConfig;
    use crate::services::HealthService;
    use crate::test_helpers;
    use crate::{AppState, ServiceFactory};

    fn test_app() -> axum::Router {
        let (_, order_service) = test_helpers::create_services();
        let health_service = HealthService::new(ApplicationConfig {
            name: "test-app".to_string(),
            version: "1.0.0".to_string(),
        });
        let state = AppState {
            health_service,
            service_factory: ServiceFactory::InMemory { order_service },
        };
        crate::routes::health::router()
            .merge(crate::routes::orders::router())
            .with_state(state)
    }

    #[tokio::test]
    async fn get_health_returns_200_ok() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_health_returns_correct_json() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "healthy");
        assert_eq!(json["application"]["name"], "test-app");
        assert_eq!(json["application"]["version"], "1.0.0");
        assert!(json["timestamp"].is_string());
    }

    #[tokio::test]
    async fn other_paths_return_404() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/other")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn health_endpoint_integration() {
        let (_, order_service) = test_helpers::create_services();
        let health_service = HealthService::new(ApplicationConfig {
            name: "ai-orders".to_string(),
            version: "0.1.0-SNAPSHOT".to_string(),
        });
        let state = AppState {
            health_service,
            service_factory: ServiceFactory::InMemory { order_service },
        };
        let app = crate::routes::health::router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "healthy");
        assert_eq!(json["application"]["name"], "ai-orders");
        assert_eq!(json["application"]["version"], "0.1.0-SNAPSHOT");
        assert!(json["timestamp"].as_str().is_some_and(|t| !t.is_empty()));
    }
}
