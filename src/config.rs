use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub application: ApplicationConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApplicationConfig {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("application.name", "rust-orders")?
            .set_default("application.version", "0.1.0-SNAPSHOT")?
            .set_default(
                "database.url",
                "postgres://aiorders:password@localhost:5432/aiorders",
            )?
            .set_default("database.max_connections", 32)?
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_configuration() {
        let config = AppConfig::load().unwrap();
        assert_eq!(config.application.name, "rust-orders");
        assert_eq!(config.application.version, "0.1.0-SNAPSHOT");
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
    }
}
