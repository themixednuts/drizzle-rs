# Justfile for drizzle-rs development tasks
# Install just: cargo install just

set windows-shell := ["pwsh", "-NoLogo", "-Command"]

# Default recipe - show available commands
default:
    @just --list

# Start PostgreSQL container
pg-up:
    docker compose up -d postgres
    @echo "Waiting for PostgreSQL to be ready..."
    @docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    @echo "PostgreSQL is ready!"

# Stop PostgreSQL container
pg-down:
    docker compose down

# Stop and remove PostgreSQL data
pg-clean:
    docker compose down -v

# Run PostgreSQL tests (starts container, runs tests, stops container)
# Tests run in parallel - each test gets its own isolated schema
test-pg *ARGS: pg-up
    -cargo test --features postgres-sync,tokio-postgres,uuid {{ARGS}}
    just pg-down

# Run PostgreSQL tests without stopping container (useful for development)
test-pg-dev *ARGS: pg-up
    cargo test --features postgres-sync,tokio-postgres,uuid {{ARGS}}

# Run all SQLite tests
test-sqlite *ARGS:
    cargo test --features rusqlite,uuid {{ARGS}}

# Run all tests (SQLite + PostgreSQL)
test-all *ARGS: pg-up
    -cargo test --features rusqlite,postgres-sync,tokio-postgres,uuid {{ARGS}}
    just pg-down

# SQLite core matrix: driver + uuid + serde combos
test-sqlite-matrix-core:
    cargo clippy --features rusqlite -- -D warnings
    cargo clippy --features rusqlite,uuid -- -D warnings
    cargo clippy --features rusqlite,uuid,serde -- -D warnings
    cargo clippy --features libsql -- -D warnings
    cargo clippy --features libsql,uuid -- -D warnings
    cargo clippy --features libsql,uuid,serde -- -D warnings
    cargo test --lib --tests --features rusqlite
    cargo test --lib --tests --features rusqlite,uuid
    cargo test --lib --tests --features rusqlite,uuid,serde
    cargo test --lib --tests --features libsql -- --test-threads=1
    cargo test --lib --tests --features libsql,uuid -- --test-threads=1
    cargo test --lib --tests --features libsql,uuid,serde -- --test-threads=1

# SQLite ext matrix: optional type features
test-sqlite-matrix-ext:
    cargo clippy --features rusqlite,arrayvec -- -D warnings
    cargo clippy --features rusqlite,compact-str -- -D warnings
    cargo clippy --features rusqlite,bytes -- -D warnings
    cargo clippy --features rusqlite,smallvec-types -- -D warnings
    cargo test --lib --tests --features rusqlite,arrayvec
    cargo test --lib --tests --features rusqlite,compact-str
    cargo test --lib --tests --features rusqlite,bytes
    cargo test --lib --tests --features rusqlite,smallvec-types

# Run full SQLite matrix (core + ext)
test-sqlite-matrix: test-sqlite-matrix-core test-sqlite-matrix-ext

# PostgreSQL core matrix: driver + uuid + serde combos
test-pg-matrix-core: pg-up
    cargo clippy --features postgres-sync -- -D warnings
    cargo clippy --features postgres-sync,uuid -- -D warnings
    cargo clippy --features postgres-sync,uuid,serde -- -D warnings
    cargo clippy --features tokio-postgres -- -D warnings
    cargo clippy --features tokio-postgres,uuid -- -D warnings
    cargo clippy --features tokio-postgres,uuid,serde -- -D warnings
    cargo test --lib --tests --features postgres-sync
    cargo test --lib --tests --features postgres-sync,uuid
    cargo test --lib --tests --features postgres-sync,uuid,serde
    cargo test --lib --tests --features tokio-postgres
    cargo test --lib --tests --features tokio-postgres,uuid
    cargo test --lib --tests --features tokio-postgres,uuid,serde

# PostgreSQL ext matrix: optional type features
test-pg-matrix-ext: pg-up
    cargo clippy --features tokio-postgres,arrayvec -- -D warnings
    cargo clippy --features tokio-postgres,compact-str -- -D warnings
    cargo clippy --features tokio-postgres,bytes -- -D warnings
    cargo clippy --features tokio-postgres,smallvec-types -- -D warnings
    cargo clippy --features tokio-postgres,chrono -- -D warnings
    cargo clippy --features tokio-postgres,cidr -- -D warnings
    cargo clippy --features tokio-postgres,geo-types -- -D warnings
    cargo clippy --features tokio-postgres,bit-vec -- -D warnings
    cargo test --lib --tests --features tokio-postgres,arrayvec
    cargo test --lib --tests --features tokio-postgres,compact-str
    cargo test --lib --tests --features tokio-postgres,bytes
    cargo test --lib --tests --features tokio-postgres,smallvec-types
    cargo test --lib --tests --features tokio-postgres,chrono
    cargo test --lib --tests --features tokio-postgres,cidr
    cargo test --lib --tests --features tokio-postgres,geo-types
    cargo test --lib --tests --features tokio-postgres,bit-vec

# Run full PostgreSQL matrix (core + ext)
test-pg-matrix: test-pg-matrix-core test-pg-matrix-ext

# Check PostgreSQL container status
pg-status:
    docker compose ps

# View PostgreSQL logs
pg-logs:
    docker compose logs -f postgres

# Connect to PostgreSQL with psql
pg-shell:
    docker compose exec postgres psql -U postgres -d drizzle_test
