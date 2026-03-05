# Test module — `just test::sqlite`, `just test::pg`, `just test`
#
# Usage:
#   just test                                        — run all test matrixes
#   just test::sqlite                                — SQLite with default features (rusqlite,uuid)
#   just test::sqlite "rusqlite,libsql,uuid"         — SQLite with custom features
#   just test::pg                                    — PostgreSQL with default features
#   just test::pg "postgres-sync,tokio-postgres,uuid" — PostgreSQL with custom features

set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# Additive features shared across all SQLite drivers
_sqlite-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types"
# Additive features shared across all PostgreSQL drivers
_pg-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types,chrono,cidr,geo-types,bit-vec"

# Run all test matrixes (SQLite + PostgreSQL)
default: sqlite-matrix pg-matrix

# ---------------------------------------------------------------------------
# Quick runs — single feature set
# ---------------------------------------------------------------------------

# Run SQLite tests with the given features.
sqlite features="rusqlite,uuid" *ARGS:
    cargo test --workspace --features "{{features}}" {{ARGS}}

# Run PostgreSQL tests. Starts the container automatically.
pg features="postgres-sync,tokio-postgres,uuid" *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres
    echo "Waiting for PostgreSQL to be ready..."
    docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    echo "PostgreSQL is ready!"
    cargo test --workspace --features "{{features}}" {{ARGS}}

# ---------------------------------------------------------------------------
# Full matrixes — bare driver + all additive features per driver
# ---------------------------------------------------------------------------

# Run full SQLite matrix
sqlite-matrix:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== SQLite matrix ==="
    # rusqlite: bare then full
    cargo test --workspace --lib --tests --features rusqlite
    cargo test --workspace --lib --tests --features "rusqlite,{{_sqlite-ext}}"
    # libsql: bare then full
    cargo test --workspace --lib --tests --features libsql
    cargo test --workspace --lib --tests --features "libsql,{{_sqlite-ext}}"

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
    cargo test --workspace --lib --tests --features postgres-sync
    cargo test --workspace --lib --tests --features "postgres-sync,{{_pg-ext}}"
    # tokio-postgres: bare then full
    cargo test --workspace --lib --tests --features tokio-postgres
    cargo test --workspace --lib --tests --features "tokio-postgres,{{_pg-ext}}"

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
