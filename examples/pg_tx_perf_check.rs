//! Perf verification for statement caching inside Postgres transactions.
//!
//! Times the same by-id lookup three ways inside one transaction:
//!
//! - `tx_uncached` — raw `tx.query(&str, ..)`, which is what the drizzle
//!   transaction runner did before the cache was wired through. Every call
//!   pays a Parse round trip.
//! - `drizzle_tx` — the drizzle transaction builder, now serving statements
//!   from the connection's cache.
//! - `raw_prepared` — raw `tx.query(&Statement, ..)` against a statement
//!   prepared once up front. This is the floor.
//!
//! `drizzle_tx` should land at `raw_prepared` parity, well under `tx_uncached`.
//! Requires the docker Postgres from `just pg-up`.
//!
//! Run: cargo run --release --example pg_tx_perf_check --features tokio-postgres

#[cfg(feature = "tokio-postgres")]
#[tokio::main]
async fn main() -> drizzle::Result<()> {
    use std::time::{Duration, Instant};

    use drizzle::core::expr::eq;
    use drizzle::postgres::common::PostgresTransactionType;
    use drizzle::postgres::prelude::*;
    use drizzle::postgres::tokio::Drizzle;

    #[PostgresTable(name = "orders")]
    struct Order {
        #[column(serial, primary)]
        id: i32,
        ship_name: String,
    }

    #[derive(PostgresSchema)]
    struct Schema {
        order: Order,
    }

    const ADMIN_URL: &str = "host=localhost user=postgres password=postgres dbname=postgres";
    const URL: &str = "host=localhost user=postgres password=postgres dbname=tx_perf_check";
    const ITERATIONS: usize = 400;

    let (admin, conn) = tokio_postgres::connect(ADMIN_URL, tokio_postgres::NoTls)
        .await
        .expect("connect admin");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    admin
        .execute("DROP DATABASE IF EXISTS tx_perf_check", &[])
        .await
        .expect("drop scratch db");
    admin
        .execute("CREATE DATABASE tx_perf_check", &[])
        .await
        .expect("create scratch db");

    let (client, conn) = tokio_postgres::connect(URL, tokio_postgres::NoTls)
        .await
        .expect("connect scratch");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client
        .batch_execute(
            "CREATE TABLE orders (id serial PRIMARY KEY, ship_name text NOT NULL);
             INSERT INTO orders (ship_name) SELECT 'ship-' || g FROM generate_series(1, 20000) g;
             ANALYZE;",
        )
        .await
        .expect("seed");

    let (mut db, Schema { order }) = Drizzle::new(client, Schema::new());

    // Raw reference client, so the uncached and prepared baselines run in a
    // transaction of their own against the same server.
    let (mut raw, conn) = tokio_postgres::connect(URL, tokio_postgres::NoTls)
        .await
        .expect("connect raw");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let mut uncached = Vec::with_capacity(ITERATIONS);
    let mut drizzle_tx = Vec::with_capacity(ITERATIONS);
    let mut raw_prepared = Vec::with_capacity(ITERATIONS);

    let raw_tx = raw.transaction().await?;
    let stmt = raw_tx
        .prepare("SELECT id, ship_name FROM orders WHERE id = $1")
        .await?;
    for i in 0..ITERATIONS {
        let id = (i % 20000) as i32 + 1;

        let start = Instant::now();
        let rows = raw_tx
            .query("SELECT id, ship_name FROM orders WHERE id = $1", &[&id])
            .await?;
        uncached.push(start.elapsed());
        assert_eq!(rows.len(), 1);

        let start = Instant::now();
        let rows = raw_tx.query(&stmt, &[&id]).await?;
        raw_prepared.push(start.elapsed());
        assert_eq!(rows.len(), 1);
    }
    raw_tx.commit().await?;

    db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
        for i in 0..ITERATIONS {
            let id = (i % 20000) as i32 + 1;
            let start = Instant::now();
            let rows: Vec<SelectOrder> = tx
                .select(())
                .from(order)
                .r#where(eq(order.id, id))
                .all()
                .await?;
            drizzle_tx.push(start.elapsed());
            assert_eq!(rows.len(), 1);
        }
        Ok(())
    })
    .await?;

    fn report(label: &str, mut samples: Vec<Duration>) -> Duration {
        samples.sort_unstable();
        let min = samples[0];
        let median = samples[samples.len() / 2];
        println!("{label:<14} min {min:>9.3?}   median {median:>9.3?}");
        min
    }

    println!("by-id lookup inside one transaction, {ITERATIONS} iterations each\n");
    let uncached_min = report("tx_uncached", uncached);
    let drizzle_min = report("drizzle_tx", drizzle_tx);
    let prepared_min = report("raw_prepared", raw_prepared);

    println!(
        "\ndrizzle_tx vs raw_prepared: {:.2}x    tx_uncached vs raw_prepared: {:.2}x",
        drizzle_min.as_secs_f64() / prepared_min.as_secs_f64(),
        uncached_min.as_secs_f64() / prepared_min.as_secs_f64(),
    );

    Ok(())
}

#[cfg(not(feature = "tokio-postgres"))]
fn main() {
    eprintln!("enable the tokio-postgres feature to run this example");
}
