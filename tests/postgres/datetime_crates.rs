//! Date and time values from the `time` and `jiff` crates on PostgreSQL:
//! each type infers its column type, binds through the driver's codec, and
//! decodes the same way in models and in selected columns.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

#[cfg(feature = "jiff")]
mod jiff_values {
    use drizzle::core::expr::*;
    use drizzle::postgres::prelude::*;
    use jiff::Timestamp;
    use jiff::civil::{Date, DateTime, Time};

    // `civil::DateTime` infers TIMESTAMP and `Timestamp` TIMESTAMPTZ; the
    // driver's jiff codecs reject any other column type, so the round trip
    // below also checks the inference.
    #[PostgresTable(name = "pg_jiff_events")]
    struct PgJiffEvent {
        #[column(serial, primary)]
        id: i32,
        day: Date,
        starts: Time,
        local: DateTime,
        instant: Timestamp,
        ends: Option<Timestamp>,
    }

    #[derive(PostgresSchema)]
    struct PgJiffSchema {
        events: PgJiffEvent,
    }

    #[drizzle::test]
    fn jiff_values_round_trip(db: &mut TestDb<PgJiffSchema>) {
        let PgJiffSchema { events } = schema;
        let day = jiff::civil::date(2026, 9, 23);
        let starts = jiff::civil::time(9, 30, 15, 500_000_000);
        let local = day.at(9, 30, 15, 250_000_000);
        let instant: Timestamp = "2026-09-23T07:30:15.125Z".parse().unwrap();

        db.insert(events)
            .values([InsertPgJiffEvent::new(day, starts, local, instant)])
            .execute();

        let row: SelectPgJiffEvent = db.select(()).from(events).get();
        assert_eq!(
            (row.day, row.starts, row.local, row.instant, row.ends),
            (day, starts, local, instant, None)
        );

        // A selected column decodes on its own too.
        let (selected_day, selected_starts, selected_instant): (Date, Time, Timestamp) = db
            .select((events.day, events.starts, events.instant))
            .from(events)
            .get();
        assert_eq!(
            (selected_day, selected_starts, selected_instant),
            (day, starts, instant)
        );

        let matched: Vec<SelectPgJiffEvent> = db
            .select(())
            .from(events)
            .r#where(and(eq(events.local, local), eq(events.instant, instant)))
            .all();
        assert_eq!(matched.len(), 1);
    }
}

#[cfg(feature = "time")]
mod time_values {
    use drizzle::postgres::prelude::*;
    use time::macros::{date, datetime, time};

    #[PostgresTable(name = "pg_time_events")]
    struct PgTimeEvent {
        #[column(serial, primary)]
        id: i32,
        day: time::Date,
        starts: time::Time,
        local: time::PrimitiveDateTime,
        instant: time::OffsetDateTime,
    }

    #[derive(PostgresSchema)]
    struct PgTimeSchema {
        events: PgTimeEvent,
    }

    #[drizzle::test]
    fn time_values_decode_in_models_and_selected_columns(db: &mut TestDb<PgTimeSchema>) {
        let PgTimeSchema { events } = schema;
        let day = date!(2026 - 09 - 23);
        let starts = time!(9:30:15.5);
        let local = datetime!(2026-09-23 9:30:15.25);
        let instant = datetime!(2026-09-23 7:30:15.125 UTC);

        db.insert(events)
            .values([InsertPgTimeEvent::new(day, starts, local, instant)])
            .execute();

        let row: SelectPgTimeEvent = db.select(()).from(events).get();
        assert_eq!(
            (row.day, row.starts, row.local, row.instant),
            (day, starts, local, instant)
        );

        let (selected_day, selected_local, selected_instant): (
            time::Date,
            time::PrimitiveDateTime,
            time::OffsetDateTime,
        ) = db
            .select((events.day, events.local, events.instant))
            .from(events)
            .get();
        assert_eq!(
            (selected_day, selected_local, selected_instant),
            (day, local, instant)
        );
    }
}
