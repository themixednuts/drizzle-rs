# ORM comparison examples

Shared schema: `schema.sql` and `seed.sql`.

Each crate is standalone (separate `Cargo.lock`) so SQLite native deps do not clash between tools.

```bash
cd examples/orm-comparison/drizzle && cargo run
cd examples/orm-comparison/sqlx && cargo run
cd examples/orm-comparison/diesel && cargo run
cd examples/orm-comparison/seaorm && cargo run
cd examples/orm-comparison/toasty && cargo run
```

`drizzle` applies `include_migrations!("./drizzle")` and seeds with generated `Insert*` models. Other demos load `schema.sql` / `seed.sql` directly.
