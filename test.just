# Test module — `just test::sqlite`, `just test::pg`, `just test`
#
# Usage:
#   just test                                        — run all test matrixes
#   just test::sqlite                                — SQLite with default features (rusqlite,uuid)
#   just test::sqlite "rusqlite,libsql,uuid"         — SQLite with custom features (libsql auto-adds --test-threads=1)
#   just test::pg                                    — PostgreSQL with default features
#   just test::pg "postgres-sync,tokio-postgres,uuid" — PostgreSQL with custom features

set windows-shell := ["pwsh", "-NoLogo", "-Command"]

# Run all test matrixes (SQLite + PostgreSQL)
default: sqlite-matrix pg-matrix

# ---------------------------------------------------------------------------
# Quick runs — single feature set
# ---------------------------------------------------------------------------

# Run SQLite tests. libsql in the feature list automatically adds --test-threads=1.
sqlite features="rusqlite,uuid" *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "{{features}}" == *"libsql"* ]]; then
        cargo test --features "{{features}}" {{ARGS}} -- --test-threads=1
    else
        cargo test --features "{{features}}" {{ARGS}}
    fi

# Run PostgreSQL tests. Starts the container automatically.
pg features="postgres-sync,tokio-postgres,uuid" *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres
    echo "Waiting for PostgreSQL to be ready..."
    docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    echo "PostgreSQL is ready!"
    cargo test --features "{{features}}" {{ARGS}}

# ---------------------------------------------------------------------------
# Full matrixes
# ---------------------------------------------------------------------------

# Run full SQLite matrix (core drivers + extension types)
sqlite-matrix: _sqlite-matrix-core _sqlite-matrix-ext

# Run full PostgreSQL matrix (core drivers + extension types)
pg-matrix: _pg-matrix-core _pg-matrix-ext

# ---------------------------------------------------------------------------
# SQLite matrix internals
# ---------------------------------------------------------------------------

_sqlite-matrix-core:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== SQLite core matrix ==="
    # rusqlite combos
    cargo clippy --features rusqlite -- -D warnings
    cargo clippy --features rusqlite,uuid -- -D warnings
    cargo clippy --features rusqlite,uuid,serde -- -D warnings
    cargo test --lib --tests --features rusqlite
    cargo test --lib --tests --features rusqlite,uuid
    cargo test --lib --tests --features rusqlite,uuid,serde
    # libsql combos (single-threaded)
    cargo clippy --features libsql -- -D warnings
    cargo clippy --features libsql,uuid -- -D warnings
    cargo clippy --features libsql,uuid,serde -- -D warnings
    cargo test --lib --tests --features libsql -- --test-threads=1
    cargo test --lib --tests --features libsql,uuid -- --test-threads=1
    cargo test --lib --tests --features libsql,uuid,serde -- --test-threads=1

_sqlite-matrix-ext:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== SQLite ext matrix ==="
    for feat in arrayvec compact-str bytes smallvec-types; do
        cargo clippy --features "rusqlite,$feat" -- -D warnings
        cargo test --lib --tests --features "rusqlite,$feat"
    done

# ---------------------------------------------------------------------------
# PostgreSQL matrix internals
# ---------------------------------------------------------------------------

_pg-matrix-core:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres
    echo "Waiting for PostgreSQL to be ready..."
    docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    echo "PostgreSQL is ready!"
    echo "=== PostgreSQL core matrix ==="
    # postgres-sync combos
    cargo clippy --features postgres-sync -- -D warnings
    cargo clippy --features postgres-sync,uuid -- -D warnings
    cargo clippy --features postgres-sync,uuid,serde -- -D warnings
    cargo test --lib --tests --features postgres-sync
    cargo test --lib --tests --features postgres-sync,uuid
    cargo test --lib --tests --features postgres-sync,uuid,serde
    # tokio-postgres combos
    cargo clippy --features tokio-postgres -- -D warnings
    cargo clippy --features tokio-postgres,uuid -- -D warnings
    cargo clippy --features tokio-postgres,uuid,serde -- -D warnings
    cargo test --lib --tests --features tokio-postgres
    cargo test --lib --tests --features tokio-postgres,uuid
    cargo test --lib --tests --features tokio-postgres,uuid,serde

_pg-matrix-ext:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres
    docker compose exec -T postgres sh -c 'until pg_isready -U postgres -d drizzle_test; do sleep 1; done'
    echo "=== PostgreSQL ext matrix ==="
    for feat in arrayvec compact-str bytes smallvec-types chrono cidr geo-types bit-vec; do
        cargo clippy --features "tokio-postgres,$feat" -- -D warnings
        cargo test --lib --tests --features "tokio-postgres,$feat"
    done
