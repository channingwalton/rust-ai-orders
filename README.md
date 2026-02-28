# rust-orders

A Rust port of [ai-orders](https://github.com/channingwalton/ai-orders), a RESTful order management API originally built with Scala 3 and the Typelevel stack.

## Original → Rust

| Scala | Rust |
|-------|------|
| http4s + Ember | axum + tokio |
| Circe | serde + serde_json |
| Doobie + HikariCP | sqlx + PgPool |
| Cats Effect IO | tokio async/await |
| Flyway | sqlx::migrate! |
| PureConfig | config crate |
| log4cats + Logback | tracing + tracing-subscriber |
| Sealed trait errors | thiserror enum |
| BigDecimal | rust_decimal |
| munit + TestContainers | built-in #[test] + testcontainers-rs |

## API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check with status, timestamp, app info |
| `POST` | `/orders` | Create an order (validates user exists) |
| `GET` | `/orders/user/{userId}` | List orders for a user (newest first) |

## Project Structure

```
src/
├── main.rs              # Entry point, wiring
├── config.rs            # AppConfig with env var overrides (APP__ prefix)
├── models/
│   ├── user.rs          # UserId, User
│   ├── order.rs         # OrderId, ProductId, Order, DTOs
│   ├── health.rs        # HealthCheck, ApplicationInfo
│   └── error.rs         # ServiceError → HTTP responses
├── services/
│   ├── health.rs        # HealthService
│   ├── user.rs          # UserService (user validation + CRUD)
│   └── order.rs         # OrderService (create, list by user)
├── store/
│   ├── mod.rs           # UserRepository + OrderRepository traits
│   ├── user_store.rs    # PgUserStore (PostgreSQL)
│   └── order_store.rs   # PgOrderStore (PostgreSQL)
├── routes/
│   ├── health.rs        # GET /health
│   └── orders.rs        # POST /orders, GET /orders/user/{userId}
└── test_helpers.rs      # In-memory store implementations for tests
migrations/
├── 20240101000000_create_users_table.sql
└── 20240101000001_create_orders_table.sql
```

## Running

```bash
# Start PostgreSQL (e.g. via Docker)
docker run -d --name pg -e POSTGRES_USER=aiorders -e POSTGRES_PASSWORD=password -e POSTGRES_DB=aiorders -p 5432:5432 postgres:15

# Run the server
cargo run

# Override config via environment
APP__SERVER__PORT=9090 APP__DATABASE__URL=postgres://user:pass@host/db cargo run
```

## Testing

```bash
cargo test                       # 27 unit tests (no Docker needed)
cargo test -- --ignored          # 11 integration tests (Docker required)
cargo test -- --include-ignored  # all 38 tests
```

The test suite mirrors the original Scala tests:

- **Service tests** — HealthService, UserService, OrderService using in-memory stores
- **Route tests** — HTTP request/response testing via `tower::ServiceExt::oneshot`
- **Integration tests** — PostgreSQL CRUD via testcontainers (Docker)
