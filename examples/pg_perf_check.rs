//! End-to-end perf verification for the tokio-postgres driver fixes:
//! statement caching, bound pagination params, and the paginated relational
//! query shape. Requires the docker Postgres from `just pg-up`.
//!
//! Run: cargo run --release --example pg_perf_check --features "tokio-postgres,query"

#[cfg(all(feature = "tokio-postgres", feature = "query"))]
#[tokio::main]
async fn main() -> drizzle::Result<()> {
    use std::time::Instant;

    use drizzle::core::expr::eq;
    use drizzle::postgres::prelude::*;
    use drizzle::postgres::tokio::Drizzle;

    #[PostgresTable(name = "orders")]
    struct Order {
        #[column(serial, primary)]
        id: i32,
        ship_name: String,
    }

    #[PostgresTable(name = "order_details")]
    struct Detail {
        quantity: i32,
        #[column(references = Order::id)]
        order_id: i32,
    }

    #[derive(PostgresSchema)]
    struct Schema {
        order: Order,
        detail: Detail,
    }

    const ADMIN_URL: &str = "host=localhost user=postgres password=postgres dbname=postgres";
    const URL: &str = "host=localhost user=postgres password=postgres dbname=perf_check";

    // Fresh scratch database.
    let (admin, conn) = tokio_postgres::connect(ADMIN_URL, tokio_postgres::NoTls)
        .await
        .expect("connect admin");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    admin
        .execute("DROP DATABASE IF EXISTS perf_check", &[])
        .await
        .expect("drop perf_check db");
    admin
        .execute("CREATE DATABASE perf_check", &[])
        .await
        .expect("create perf_check db");

    let (client, conn) = tokio_postgres::connect(URL, tokio_postgres::NoTls)
        .await
        .expect("connect perf_check");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    client
        .batch_execute(
            "CREATE TABLE orders (id serial PRIMARY KEY, ship_name text NOT NULL);
             CREATE TABLE order_details (quantity int NOT NULL, order_id int NOT NULL REFERENCES orders(id));
             INSERT INTO orders (ship_name) SELECT 'ship-' || g FROM generate_series(1, 50000) g;
             INSERT INTO order_details (quantity, order_id)
               SELECT (o.id + p) % 40 + 1, o.id FROM orders o, generate_series(1, 3) p;
             CREATE INDEX idx_details_order ON order_details(order_id);
             ANALYZE;",
        )
        .await
        .expect("seed");

    let (db, Schema { order, detail: _ }) = Drizzle::new(client, Schema::new());

    // --- Scenario 1: by-id select — drizzle builder vs raw prepared vs raw text ---
    const ITERS: i32 = 500;

    let warm = |i: i32| (i % 50000) + 1;

    // drizzle builder (statement cache should make this match raw-prepared)
    for i in 0..10 {
        let _rows: Vec<SelectOrder> = db
            .select(())
            .from(order)
            .r#where(eq(order.id, warm(i)))
            .all()
            .await?;
    }
    let start = Instant::now();
    for i in 0..ITERS {
        let _rows: Vec<SelectOrder> = db
            .select(())
            .from(order)
            .r#where(eq(order.id, warm(i)))
            .all()
            .await?;
    }
    let drizzle_us = start.elapsed().as_micros() as f64 / f64::from(ITERS);

    // drizzle public prepared-statement API (registration-based cache)
    let id_ph = order.id.placeholder("id");
    let prepared = db
        .select(())
        .from(order)
        .r#where(eq(order.id, id_ph))
        .prepare()
        .into_owned();
    for i in 0..10 {
        let _rows: Vec<SelectOrder> = prepared.all(db.conn(), [id_ph.bind(warm(i))]).await?;
    }
    let start = Instant::now();
    for i in 0..ITERS {
        let _rows: Vec<SelectOrder> = prepared.all(db.conn(), [id_ph.bind(warm(i))]).await?;
    }
    let drizzle_prepared_us = start.elapsed().as_micros() as f64 / f64::from(ITERS);

    let raw_sql = "SELECT id, ship_name FROM orders WHERE id = $1";
    let stmt = db.conn().prepare(raw_sql).await.expect("prepare");
    let start = Instant::now();
    for i in 0..ITERS {
        let _rows = db
            .conn()
            .query(&stmt, &[&warm(i)])
            .await
            .expect("raw prepared");
    }
    let raw_prepared_us = start.elapsed().as_micros() as f64 / f64::from(ITERS);

    let start = Instant::now();
    for i in 0..ITERS {
        let _rows = db
            .conn()
            .query(raw_sql, &[&warm(i)])
            .await
            .expect("raw text");
    }
    let raw_text_us = start.elapsed().as_micros() as f64 / f64::from(ITERS);

    println!("by-id select, mean per query over {ITERS} iters:");
    println!("  drizzle builder (cached): {drizzle_us:8.1} us");
    println!("  drizzle prepared stmt:    {drizzle_prepared_us:8.1} us");
    println!("  raw prepared:             {raw_prepared_us:8.1} us");
    println!("  raw one-shot text:        {raw_text_us:8.1} us");

    // --- Scenario 2: paginated select across many offsets — one cached statement ---
    for page in 0..50u32 {
        let _rows: Vec<SelectOrder> = db
            .select(())
            .from(order)
            .order_by([asc(order.id)])
            .limit(50)
            .offset(page as usize * 50)
            .all()
            .await?;
    }
    let row = db
        .conn()
        .query_one(
            "SELECT count(*) FROM pg_prepared_statements WHERE statement LIKE '%LIMIT%'",
            &[],
        )
        .await
        .expect("pg_prepared_statements");
    let cached: i64 = row.get(0);
    println!(
        "paginated select over 50 distinct offsets -> {cached} prepared LIMIT statement(s) on the connection"
    );

    // --- Scenario 3: relational query with large offset (bench pathology) ---
    for _ in 0..3 {
        let _ = db
            .query(order)
            .with(order.details())
            .order_by(asc(order.id))
            .limit(50)
            .offset(25_000)
            .find_many()
            .await?;
    }
    const QITERS: u32 = 20;
    let start = Instant::now();
    let mut last_len = 0;
    for _ in 0..QITERS {
        let rows = db
            .query(order)
            .with(order.details())
            .order_by(asc(order.id))
            .limit(50)
            .offset(25_000)
            .find_many()
            .await?;
        last_len = rows.len();
    }
    let query_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(QITERS);
    println!(
        "relational find_many (LIMIT 50 OFFSET 25000, 50k orders x3 details): {query_ms:.2} ms/query ({last_len} rows)"
    );

    Ok(())
}

#[cfg(not(all(feature = "tokio-postgres", feature = "query")))]
fn main() {
    eprintln!("requires --features \"tokio-postgres,query\"");
}
