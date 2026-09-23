//! Date and time values from the `jiff` crate on MySQL: each type infers its
//! column type and decodes the same way in models and in selected columns.

#![cfg(all(any(feature = "mysql-sync", feature = "mysql-async"), feature = "jiff"))]

use drizzle::core::expr::eq;
use drizzle::mysql::prelude::*;
use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Time};

#[MySQLTable(NAME = "mysql_jiff_events")]
struct MySQLJiffEvent {
    #[column(PRIMARY, DEFAULT = 0)]
    id: i32,
    day: Date,
    #[column(TIME(6))]
    starts: Time,
    #[column(DATETIME(6))]
    local: DateTime,
    #[column(TIMESTAMP(6))]
    instant: Timestamp,
}

#[derive(MySQLSchema)]
struct MySQLJiffSchema {
    events: MySQLJiffEvent,
}

#[drizzle::test(mysql)]
fn jiff_values_round_trip(db: &mut TestDb<MySQLJiffSchema>) {
    let MySQLJiffSchema { events } = schema;
    let day = jiff::civil::date(2026, 9, 23);
    let starts = jiff::civil::time(9, 30, 15, 500_000);
    let local = day.at(9, 30, 15, 789_000);
    let instant: Timestamp = "2026-09-23T07:30:15.125Z".parse().unwrap();

    db.insert(events)
        .values([InsertMySQLJiffEvent::new(day, starts, local, instant).with_id(1)])
        .execute();

    let row: SelectMySQLJiffEvent = db.select(()).from(events).get();
    assert_eq!(
        (row.day, row.starts, row.local, row.instant),
        (day, starts, local, instant)
    );

    // A selected column decodes on its own too.
    let (selected_day, selected_local, selected_instant): (Date, DateTime, Timestamp) = db
        .select((events.day, events.local, events.instant))
        .from(events)
        .get();
    assert_eq!(
        (selected_day, selected_local, selected_instant),
        (day, local, instant)
    );

    let matched: Vec<SelectMySQLJiffEvent> = db
        .select(())
        .from(events)
        .r#where(eq(events.instant, instant))
        .all();
    assert_eq!(matched.len(), 1);
}
