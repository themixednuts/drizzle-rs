//! Date and time values from the `time` crate on PostgreSQL:
//! each type infers its column type, binds through the driver's codec, and
//! decodes the same way in models and in selected columns.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

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
