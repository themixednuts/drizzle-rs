# Justfile for drizzle-rs development tasks
# Install just: cargo install just

set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

mod test

# Default recipe - show available commands
default:
    @just --list --list-submodules

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

# Check PostgreSQL container status
pg-status:
    docker compose ps

# View PostgreSQL logs
pg-logs:
    docker compose logs -f postgres

# Connect to PostgreSQL with psql
pg-shell:
    docker compose exec postgres psql -U postgres -d drizzle_test
