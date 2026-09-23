//! Date and time values from the `chrono` and `time` crates on SQLite:
//! stored as ISO 8601 text, decoded the same way by models and by selected
//! columns.

#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

#[cfg(feature = "time")]
mod time_values {
    use drizzle::sqlite::prelude::*;
    use time::macros::{date, datetime, time};

    #[SQLiteTable(NAME = "time_events")]
    struct TimeEvent {
        #[column(PRIMARY)]
        id: i32,
        day: time::Date,
        starts: time::Time,
        local: time::PrimitiveDateTime,
        instant: time::OffsetDateTime,
    }

    #[derive(SQLiteSchema)]
    struct TimeSchema {
        events: TimeEvent,
    }

    #[drizzle::test]
    fn time_values_decode_in_models_and_selected_columns(db: &mut TestDb<TimeSchema>) {
        let TimeSchema { events } = schema;
        let day = date!(2026 - 09 - 23);
        let starts = time!(9:30:15.5);
        let local = datetime!(2026-09-23 9:30:15);
        let instant = datetime!(2026-09-23 7:30:15 UTC);

        db.insert(events)
            .values([InsertTimeEvent::new(day, starts, local, instant).with_id(1)])
            .execute();

        let row: SelectTimeEvent = db.select(()).from(events).get();
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

    #[drizzle::test]
    fn time_values_read_sqlite_datetime_text(db: &mut TestDb<TimeSchema>) {
        let TimeSchema { events } = schema;
        // SQLite's own date and time functions put a space between date and
        // time and write no offset (the time is UTC). Versions before 0.1.17
        // wrote a TIME with ISO 8601's `T` prefix.
        result!(db.execute(SQL::raw(
            r#"INSERT INTO "time_events" ("id", "day", "starts", "local", "instant")
               VALUES (1, '2026-09-23', 'T09:30:15.500000000', '2026-09-23 09:30:15', '2026-09-23 07:30:15')"#
        )))?;

        let row: SelectTimeEvent = db.select(()).from(events).get();
        assert_eq!(
            (row.starts, row.local, row.instant),
            (
                time!(9:30:15.5),
                datetime!(2026-09-23 9:30:15),
                datetime!(2026-09-23 7:30:15 UTC)
            )
        );
    }

    #[drizzle::test]
    fn time_values_compare_in_filters(db: &mut TestDb<TimeSchema>) {
        let TimeSchema { events } = schema;
        let day = date!(2026 - 09 - 23);
        let instant = datetime!(2026-09-23 7:30:15 UTC);
        db.insert(events)
            .values([
                InsertTimeEvent::new(day, time!(9:30), datetime!(2026-09-23 9:30), instant)
                    .with_id(1),
            ])
            .execute();

        let matched: Vec<SelectTimeEvent> = db
            .select(())
            .from(events)
            .r#where(drizzle::core::expr::and(
                drizzle::core::expr::eq(events.day, day),
                drizzle::core::expr::eq(events.instant, instant),
            ))
            .all();
        assert_eq!(matched.len(), 1);
    }
}

#[cfg(feature = "chrono")]
mod chrono_values {
    use drizzle::sqlite::prelude::*;

    #[SQLiteTable(NAME = "chrono_events")]
    struct ChronoEvent {
        #[column(PRIMARY)]
        id: i32,
        instant: chrono::DateTime<chrono::Utc>,
    }

    #[derive(SQLiteSchema)]
    struct ChronoSchema {
        events: ChronoEvent,
    }

    #[drizzle::test]
    fn chrono_utc_datetime_decodes_as_a_selected_column(db: &mut TestDb<ChronoSchema>) {
        let ChronoSchema { events } = schema;
        let instant = chrono::DateTime::parse_from_rfc3339("2026-09-23T07:30:15Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        db.insert(events)
            .values([InsertChronoEvent::new(instant).with_id(1)])
            .execute();

        // The value is stored as RFC 3339, which a naive parse rejected.
        let selected: chrono::DateTime<chrono::Utc> = db.select(events.instant).from(events).get();
        assert_eq!(selected, instant);
    }
}
