# Test module — `just test::sqlite`, `just test::pg`, `just test`
#
# Usage:
#   just test                                        — run all test matrixes
#   just test::sqlite                                — SQLite with default features (rusqlite,uuid)
#   just test::sqlite "rusqlite,libsql,uuid"         — SQLite with custom features
#   just test::pg                                    — PostgreSQL with default features
#   just test::pg "postgres-sync,tokio-postgres,uuid" — PostgreSQL with custom features
#   just test::mysql-sync                            — MySQL blocking adapter + CLI parity
#   just test::mysql-async                           — MySQL async adapter + CLI parity

set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# Additive features shared across all SQLite drivers
_sqlite-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types,query"
# Additive features shared across all PostgreSQL drivers
_pg-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types,chrono,cidr,geo-types,bit-vec,query"
# Additive features supported by the MySQL dialect and both adapters.
_mysql-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types,chrono,time,rust-decimal,query"

# Run all test matrixes plus bench-runner smoke tests
default: sqlite-matrix pg-matrix mysql-matrix bench-runner

# ---------------------------------------------------------------------------
# Quick runs — single feature set
# ---------------------------------------------------------------------------

# Run SQLite tests with the given features.
sqlite features="rusqlite,uuid" *ARGS:
    cargo test --workspace --exclude bench-runner --features "{{features}}" {{ARGS}}

# Run PostgreSQL tests. Starts the container automatically.
pg features="postgres-sync,tokio-postgres,uuid" *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres
    echo "Waiting for PostgreSQL to be ready..."
    docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    echo "PostgreSQL is ready!"
    cargo test --workspace --exclude bench-runner --features "{{features}}" {{ARGS}}

# Run the blocking MySQL adapter and the shared CLI parity contract.
mysql-sync *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d mysql
    echo "Waiting for MySQL to be ready..."
    docker compose exec -T mysql sh -c 'until mysql -h 127.0.0.1 -udrizzle -pdrizzle drizzle_test -e "SELECT 1" >/dev/null 2>&1; do sleep 1; done'
    echo "MySQL is ready!"
    cargo test -p drizzle --no-default-features --features mysql-sync {{ARGS}}
    cargo test -p drizzle --no-default-features --features mysql-sync,serde --test compile_fail mysql_builder_ui
    cargo test -p drizzle --no-default-features --features "mysql-sync,{{_mysql-ext}}" {{ARGS}}
    cargo test -p drizzle-cli --no-default-features --features mysql-sync --test parity_integration {{ARGS}}

# Run the async MySQL adapter and the shared CLI parity contract.
mysql-async *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d mysql
    echo "Waiting for MySQL to be ready..."
    docker compose exec -T mysql sh -c 'until mysql -h 127.0.0.1 -udrizzle -pdrizzle drizzle_test -e "SELECT 1" >/dev/null 2>&1; do sleep 1; done'
    echo "MySQL is ready!"
    cargo test -p drizzle --no-default-features --features mysql-async {{ARGS}}
    cargo test -p drizzle --no-default-features --features "mysql-async,{{_mysql-ext}}" {{ARGS}}
    cargo test -p drizzle-cli --no-default-features --features mysql-async --test parity_integration {{ARGS}}

# Run bench-runner tests once outside feature-matrix unification
bench-runner *ARGS:
    cargo test -p bench-runner {{ARGS}}

# ---------------------------------------------------------------------------
# Full matrixes — bare driver + all additive features per driver
# ---------------------------------------------------------------------------

# Run full SQLite matrix
sqlite-matrix:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== SQLite matrix ==="
    # rusqlite: bare then full
    cargo test --workspace --exclude bench-runner --features rusqlite
    cargo test --workspace --exclude bench-runner --features "rusqlite,{{_sqlite-ext}}"
    # libsql: bare then full
    cargo test --workspace --exclude bench-runner --features libsql
    cargo test --workspace --exclude bench-runner --features "libsql,{{_sqlite-ext}}"

# Run full PostgreSQL matrix
pg-matrix:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres
    echo "Waiting for PostgreSQL to be ready..."
    docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    echo "PostgreSQL is ready!"
    echo "=== PostgreSQL matrix ==="
    # postgres-sync: bare then full
    cargo test --workspace --exclude bench-runner --features postgres-sync
    cargo test --workspace --exclude bench-runner --features "postgres-sync,{{_pg-ext}}"
    # tokio-postgres: bare then full
    cargo test --workspace --exclude bench-runner --features tokio-postgres
    cargo test --workspace --exclude bench-runner --features "tokio-postgres,{{_pg-ext}}"

# Run both MySQL adapters separately, then verify feature unification.
mysql-matrix: mysql-sync mysql-async
    cargo test -p drizzle --no-default-features --features mysql-sync,mysql-async
    cargo test -p drizzle --no-default-features --features "mysql-sync,mysql-async,{{_mysql-ext}}"
    cargo test -p drizzle-core -p drizzle-mysql -p drizzle-migrations --all-features --tests
    cargo test -p drizzle-cli --no-default-features --features mysql-sync,mysql-async --test parity_integration

# ---------------------------------------------------------------------------
# CI helpers — output feature matrices as JSON for GitHub Actions
# ---------------------------------------------------------------------------

# Output SQLite feature matrix as JSON array
[no-exit-message]
sqlite-matrix-json:
    @echo '["rusqlite","rusqlite,{{_sqlite-ext}}","libsql","libsql,{{_sqlite-ext}}"]'

# Output PostgreSQL feature matrix as JSON array
[no-exit-message]
pg-matrix-json:
    @echo '["postgres-sync","postgres-sync,{{_pg-ext}}","tokio-postgres","tokio-postgres,{{_pg-ext}}"]'
