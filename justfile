# Justfile for drizzle-rs development tasks
# Install just: cargo install just

set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

mod act
mod test

# Default recipe - show available commands
default:
    @just --list --list-submodules

# Start the MySQL integration-test container.
mysql-up:
    docker compose up -d mysql
    @echo "Waiting for MySQL to be ready..."
    @docker compose exec -T mysql sh -c 'until mysqladmin ping -h 127.0.0.1 -uroot -pmysql --silent; do sleep 1; done'
    @echo "MySQL is ready!"

# Run the blocking MySQL adapter integration tests.
test-mysql-sync: mysql-up
    cargo test --test mysql_sync --features mysql-sync

# Stop the MySQL integration-test container.
mysql-down:
    docker compose stop mysql

# Stop and remove MySQL integration-test data.
mysql-clean:
    docker compose stop mysql
    docker compose rm -f -v mysql

# Check the MySQL integration-test container status.
mysql-status:
    docker compose ps mysql

# View MySQL integration-test logs.
mysql-logs:
    docker compose logs -f mysql

# Connect to the integration-test database with the MySQL client.
mysql-shell:
    docker compose exec mysql mysql -uroot -pmysql drizzle_test

# Start PostgreSQL container
pg-up:
    docker compose up -d postgres
    @echo "Waiting for PostgreSQL to be ready..."
    @docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    @echo "PostgreSQL is ready!"

# Stop PostgreSQL container
pg-down:
    docker compose stop postgres

# Stop and remove PostgreSQL data
pg-clean:
    docker compose stop postgres
    docker compose rm -f -v postgres

# Check PostgreSQL container status
pg-status:
    docker compose ps postgres

# View PostgreSQL logs
pg-logs:
    docker compose logs -f postgres

# Connect to PostgreSQL with psql
pg-shell:
    docker compose exec postgres psql -U postgres -d drizzle_test
