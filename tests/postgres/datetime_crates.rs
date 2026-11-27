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

        let row: SelectPgJiffEvent = db.select(()).from(events).get().expect("a row");
        assert_eq!(
            (row.day, row.starts, row.local, row.instant, row.ends),
            (day, starts, local, instant, None)
        );

        // A selected column decodes on its own too.
        let (selected_day, selected_starts, selected_instant): (Date, Time, Timestamp) = db
            .select((events.day, events.starts, events.instant))
            .from(events)
            .get()
            .expect("a row");
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

        let row: SelectPgTimeEvent = db.select(()).from(events).get().expect("a row");
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
            .get()
            .expect("a row");
        assert_eq!(
            (selected_day, selected_local, selected_instant),
            (day, local, instant)
        );
    }
}

/// A relational query reads related rows from `json_build_object`, which
/// writes dates and times as PostgreSQL's ISO text: a year past 9999 with
/// more digits, a year before 1 AD with ` BC`, and a `timestamptz` in the
/// session's time zone, whose offset has seconds in local mean time (before
/// 1900 in `Asia/Calcutta`, `+05:53:28`).
#[cfg(all(feature = "query", feature = "chrono"))]
mod chrono_relations {
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
    use drizzle::core::asc;
    use drizzle::postgres::prelude::*;

    #[PostgresTable(name = "pg_chrono_owners")]
    struct PgChronoOwner {
        #[column(primary)]
        id: i32,
    }

    #[PostgresTable(name = "pg_chrono_moments")]
    struct PgChronoMoment {
        #[column(primary)]
        id: i32,
        #[column(references = PgChronoOwner::id)]
        owner_id: i32,
        day: NaiveDate,
        local: NaiveDateTime,
        instant: DateTime<Utc>,
        zoned: DateTime<FixedOffset>,
        ends: Option<NaiveDate>,
    }

    #[derive(PostgresSchema)]
    struct PgChronoSchema {
        owner: PgChronoOwner,
        moment: PgChronoMoment,
    }

    #[drizzle::test]
    fn chrono_relations_read_years_outside_1_to_9999(db: &mut TestDb<PgChronoSchema>) {
        let PgChronoSchema { owner, moment } = schema;
        let days = [
            (1, 1, 1),
            (99, 12, 31),
            (9999, 12, 31),
            (10000, 1, 1),
            (0, 1, 1),
            (1800, 1, 1),
        ]
        .map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap());

        db.insert(owner)
            .values([InsertPgChronoOwner::new(1)])
            .execute();
        for (id, day) in (1..).zip(days) {
            let local = day.and_hms_milli_opt(1, 2, 3, 250).unwrap();
            db.insert(moment)
                .values([InsertPgChronoMoment::new(
                    id,
                    1,
                    day,
                    local,
                    local.and_utc(),
                    local.and_utc().fixed_offset(),
                )])
                .execute();
        }
        db.execute(SQL::raw("SET TIME ZONE 'Asia/Calcutta'"));

        let owners = db
            .query(owner)
            .with(owner.pg_chrono_moments().order_by(asc(moment.id)))
            .find_many();
        let moments = &owners[0].pg_chrono_moments;
        assert_eq!(moments.len(), days.len());
        for (row, day) in moments.iter().zip(days) {
            let local = day.and_hms_milli_opt(1, 2, 3, 250).unwrap();
            assert_eq!(
                (row.day, row.local, row.instant),
                (day, local, local.and_utc())
            );
            assert_eq!(row.zoned, local.and_utc());
            assert_eq!(row.ends, None);
        }
        // Local mean time: the session's offset has seconds.
        assert_eq!(
            moments[5].zoned.offset().local_minus_utc(),
            5 * 3600 + 53 * 60 + 28
        );
    }
}

#[cfg(all(feature = "query", feature = "time"))]
mod time_relations {
    use drizzle::core::asc;
    use drizzle::postgres::prelude::*;
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

    #[PostgresTable(name = "pg_time_owners")]
    struct PgTimeOwner {
        #[column(primary)]
        id: i32,
    }

    #[PostgresTable(name = "pg_time_moments")]
    struct PgTimeMoment {
        #[column(primary)]
        id: i32,
        #[column(references = PgTimeOwner::id)]
        owner_id: i32,
        day: Date,
        starts: Time,
        local: PrimitiveDateTime,
        instant: OffsetDateTime,
    }

    #[derive(PostgresSchema)]
    struct PgTimeRelationSchema {
        owner: PgTimeOwner,
        moment: PgTimeMoment,
    }

    #[drizzle::test]
    fn time_relations_read_bc_years_and_offsets_with_seconds(
        db: &mut TestDb<PgTimeRelationSchema>,
    ) {
        let PgTimeRelationSchema { owner, moment } = schema;
        let days = [
            (1, Month::January, 1),
            (99, Month::December, 31),
            (9999, Month::December, 31),
            (0, Month::January, 1),
            (1800, Month::January, 1),
        ]
        .map(|(y, m, d)| Date::from_calendar_date(y, m, d).unwrap());
        let starts = Time::from_hms_micro(1, 2, 3, 4).unwrap();

        db.insert(owner)
            .values([InsertPgTimeOwner::new(1)])
            .execute();
        for (id, day) in (1..).zip(days) {
            let local = PrimitiveDateTime::new(day, starts);
            db.insert(moment)
                .values([InsertPgTimeMoment::new(
                    id,
                    1,
                    day,
                    starts,
                    local,
                    local.assume_utc(),
                )])
                .execute();
        }
        db.execute(SQL::raw("SET TIME ZONE 'Asia/Calcutta'"));

        let owners = db
            .query(owner)
            .with(owner.pg_time_moments().order_by(asc(moment.id)))
            .find_many();
        let moments = &owners[0].pg_time_moments;
        assert_eq!(moments.len(), days.len());
        for (row, day) in moments.iter().zip(days) {
            let local = PrimitiveDateTime::new(day, starts);
            assert_eq!(
                (row.day, row.starts, row.local, row.instant),
                (day, starts, local, local.assume_utc())
            );
        }
        assert_eq!(
            moments[4].instant.offset(),
            UtcOffset::from_hms(5, 53, 28).unwrap()
        );
    }
}

#[cfg(all(feature = "query", feature = "jiff"))]
mod jiff_relations {
    use drizzle::core::asc;
    use drizzle::postgres::prelude::*;
    use jiff::Timestamp;
    use jiff::civil::{Date, DateTime, date};

    #[PostgresTable(name = "pg_jiff_owners")]
    struct PgJiffOwner {
        #[column(primary)]
        id: i32,
    }

    #[PostgresTable(name = "pg_jiff_moments")]
    struct PgJiffMoment {
        #[column(primary)]
        id: i32,
        #[column(references = PgJiffOwner::id)]
        owner_id: i32,
        day: Date,
        local: DateTime,
        instant: Timestamp,
    }

    #[derive(PostgresSchema)]
    struct PgJiffRelationSchema {
        owner: PgJiffOwner,
        moment: PgJiffMoment,
    }

    #[drizzle::test]
    fn jiff_relations_read_bc_years_and_offsets_with_seconds(
        db: &mut TestDb<PgJiffRelationSchema>,
    ) {
        let PgJiffRelationSchema { owner, moment } = schema;
        let days = [
            date(1, 1, 1),
            date(99, 12, 31),
            // `Timestamp` ends at 9999-12-30T22:00Z.
            date(9999, 12, 30),
            date(0, 1, 1),
            date(-43, 3, 15),
            date(1800, 1, 1),
        ];

        db.insert(owner)
            .values([InsertPgJiffOwner::new(1)])
            .execute();
        for (id, day) in (1..).zip(days) {
            let local = day.at(1, 2, 3, 500_000_000);
            let instant = jiff::tz::Offset::UTC.to_timestamp(local).unwrap();
            db.insert(moment)
                .values([InsertPgJiffMoment::new(id, 1, day, local, instant)])
                .execute();
        }
        db.execute(SQL::raw("SET TIME ZONE 'Asia/Calcutta'"));

        let owners = db
            .query(owner)
            .with(owner.pg_jiff_moments().order_by(asc(moment.id)))
            .find_many();
        let moments = &owners[0].pg_jiff_moments;
        assert_eq!(moments.len(), days.len());
        for (row, day) in moments.iter().zip(days) {
            let local = day.at(1, 2, 3, 500_000_000);
            let instant = jiff::tz::Offset::UTC.to_timestamp(local).unwrap();
            assert_eq!((row.day, row.local, row.instant), (day, local, instant));
        }
    }
}
