use chrono::Utc;

use crate::config::ApplicationConfig;
use crate::models::{ApplicationInfo, HealthCheck};

#[derive(Clone)]
pub struct HealthService {
    app_config: ApplicationConfig,
}

impl HealthService {
    pub fn new(app_config: ApplicationConfig) -> Self {
        Self { app_config }
    }

    pub fn check(&self) -> HealthCheck {
        HealthCheck {
            status: "healthy".to_string(),
            timestamp: Utc::now(),
            application: ApplicationInfo {
                name: self.app_config.name.clone(),
                version: self.app_config.version.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_service() -> HealthService {
        HealthService::new(ApplicationConfig {
            name: "test-app".to_string(),
            version: "1.0.0".to_string(),
        })
    }

    #[test]
    fn check_returns_healthy_status() {
        let service = test_service();
        let result = service.check();
        assert_eq!(result.status, "healthy");
        assert_eq!(result.application.name, "test-app");
        assert_eq!(result.application.version, "1.0.0");
    }

    #[test]
    fn check_returns_current_timestamp() {
        let service = test_service();
        let before = Utc::now();
        let result = service.check();
        let after = Utc::now();
        assert!(result.timestamp >= before);
        assert!(result.timestamp <= after);
    }
}
