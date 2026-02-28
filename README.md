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

## Transaction Boundary

The Scala original uses a two-functor pattern (`OrderStore[F[_], G[_]]`) with `store.commit(...)` to wrap service calls in a database transaction at the route level.

The Rust port mirrors this with `ServiceFactory::commit()`:

```rust
// Route handler — all service operations run in a single transaction
state.service_factory
    .commit(|svc| async move { svc.create_order(req).await })
    .await
```

`ServiceFactory` has two variants:

- **`Pg(PgPool)`** — begins a transaction, creates stores and services bound to it, executes the closure, and commits. If the closure returns an error, the transaction is rolled back on drop (sqlx semantics).
- **`InMemory`** — delegates directly to in-memory services (used in tests).

The underlying `DbConn` enum (`Pool | Tx`) allows stores to work transparently against either a connection pool or a shared transaction.

### Difference from Scala

The Scala version enforces transactionality through the type system. Services return `G[A]` (`ConnectionIO[A]`), and the only way to obtain an `F[A]` (`IO[A]`) — which routes need — is through `store.commit(g: G[A]): F[A]`. The compiler makes it a type error to run a service operation without a transaction.

The Rust version relies on convention rather than enforcement. `ServiceFactory::commit()` is the intended entry point, and `AppState` only exposes `ServiceFactory` (not `OrderService` directly), making misuse harder but not impossible. Rust lacks higher-kinded types, so the Scala pattern of separating `ConnectionIO` from `IO` at the type level cannot be directly replicated.

## Project Structure

```
src/
├── main.rs              # Entry point, AppState, ServiceFactory
├── config.rs            # AppConfig with env var overrides (APP__ prefix)
├── db.rs                # DbConn (Pool | Tx) transaction abstraction
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
APP__SERVER__PORT=9090 APP__DATABASE__URL=postgres://user:pass@host/db APP__DATABASE__MAX_CONNECTIONS=16 cargo run
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
