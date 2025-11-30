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

# Check PostgreSQL container status
pg-status:
    docker compose ps

# View PostgreSQL logs
pg-logs:
    docker compose logs -f postgres

# Connect to PostgreSQL with psql
pg-shell:
    docker compose exec postgres psql -U postgres -d drizzle_test
