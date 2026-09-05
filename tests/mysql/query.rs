#![cfg(all(
    any(feature = "mysql-sync", feature = "mysql-async"),
    feature = "query"
))]

use drizzle::core::asc;
use drizzle::core::expr::eq;
use drizzle::mysql::prelude::*;

crate::common::query::shared_relational_query_suite!(
    mysql,
    MySQLTable,
    MySQLSchema,
    drizzle::mysql::types::Int,
    drizzle::mysql::TransactionConfig::default()
);
crate::common::query::shared_view_query_suite!(mysql, MySQLTable, MySQLView, MySQLSchema);
crate::common::relational::shared_relational_api_suite!(
    mysql,
    MySQLTable,
    MySQLView,
    MySQLSchema,
    drizzle::mysql::types::Int,
    drizzle::mysql::TransactionConfig::default()
);

#[MySQLTable(NAME = "mysql_query_codec_values")]
struct MySQLQueryCodecValue {
    #[column(PRIMARY, DEFAULT = 0)]
    id: i32,
    #[column(BIT(64))]
    bits: u64,
    #[column(BIT)]
    flag: bool,
    #[column(BIT(8))]
    raw_bits: Vec<u8>,
    payload: Vec<u8>,
    code: [char; 3],
}

#[derive(MySQLSchema)]
struct MySQLQueryCodecSchema {
    values: MySQLQueryCodecValue,
}

#[drizzle::test(mysql)]
fn mysql_relational_projection_preserves_bit_binary_and_fixed_text(
    db: &mut TestDb<MySQLQueryCodecSchema>,
) {
    let MySQLQueryCodecSchema { values } = schema;
    let payload = vec![0, 1, 2, 0xfe, 0xff];
    db.insert(values)
        .value(
            InsertMySQLQueryCodecValue::new(
                u64::MAX,
                true,
                vec![0xa5],
                payload.clone(),
                ['R', 'S', '!'],
            )
            .with_id(1),
        )
        .execute();

    let direct: Vec<SelectMySQLQueryCodecValue> = db.select(()).from(values).all();
    assert_eq!(direct[0].bits, u64::MAX);
    assert!(direct[0].flag);
    assert_eq!(direct[0].raw_bits, [0xa5]);
    assert_eq!(direct[0].payload, payload);
    assert_eq!(direct[0].code, ['R', 'S', '!']);

    let projected = db
        .query(values)
        .columns(values.columns().bits().flag().raw_bits().payload().code())
        .find_first()
        .unwrap();
    assert_eq!(projected.bits, Some(u64::MAX));
    assert_eq!(projected.flag, Some(true));
    assert_eq!(projected.raw_bits, Some(vec![0xa5]));
    assert_eq!(projected.payload, Some(payload));
    assert_eq!(projected.code, Some(['R', 'S', '!']));
}

#[cfg(feature = "rust-decimal")]
#[MySQLTable(NAME = "mysql_query_decimal_values")]
struct MySQLQueryDecimalValue {
    #[column(PRIMARY, DEFAULT = 0)]
    id: i32,
    #[column(DECIMAL(38, 18))]
    amount: rust_decimal::Decimal,
}

#[cfg(feature = "rust-decimal")]
#[derive(MySQLSchema)]
struct MySQLQueryDecimalSchema {
    values: MySQLQueryDecimalValue,
}

#[cfg(feature = "rust-decimal")]
#[drizzle::test(mysql)]
fn mysql_relational_projection_preserves_exact_decimal(db: &mut TestDb<MySQLQueryDecimalSchema>) {
    use core::str::FromStr as _;

    let MySQLQueryDecimalSchema { values } = schema;
    let amount = rust_decimal::Decimal::from_str("1234567890.123456789012345678").unwrap();
    let _comparison = eq(values.amount, amount);
    let _arithmetic = values.amount + amount;
    db.insert(values)
        .value(InsertMySQLQueryDecimalValue::new(amount).with_id(1))
        .execute();

    let projected = db
        .query(values)
        .columns(values.columns().amount())
        .find_first()
        .unwrap();
    assert_eq!(projected.amount, Some(amount));
}

#[cfg(feature = "chrono")]
#[MySQLTable(NAME = "mysql_query_chrono_values")]
struct MySQLQueryChronoValue {
    #[column(PRIMARY, DEFAULT = 0)]
    id: i32,
    #[column(DATETIME(6))]
    moment: chrono::NaiveDateTime,
}

#[cfg(feature = "chrono")]
#[derive(MySQLSchema)]
struct MySQLQueryChronoSchema {
    values: MySQLQueryChronoValue,
}

#[cfg(feature = "chrono")]
#[drizzle::test(mysql)]
fn mysql_relational_projection_decodes_chrono_without_serde(
    db: &mut TestDb<MySQLQueryChronoSchema>,
) {
    let MySQLQueryChronoSchema { values } = schema;
    let moment = chrono::NaiveDate::from_ymd_opt(2026, 8, 26)
        .unwrap()
        .and_hms_micro_opt(12, 34, 56, 789)
        .unwrap();
    db.insert(values)
        .value(InsertMySQLQueryChronoValue::new(moment).with_id(1))
        .execute();

    let projected = db
        .query(values)
        .columns(values.columns().moment())
        .find_first()
        .unwrap();
    assert_eq!(projected.moment, Some(moment));
}

#[cfg(feature = "time")]
#[MySQLTable(NAME = "mysql_query_time_values")]
struct MySQLQueryTimeValue {
    #[column(PRIMARY, DEFAULT = 0)]
    id: i32,
    #[column(DATETIME(6))]
    moment: time::PrimitiveDateTime,
}

#[cfg(feature = "time")]
#[derive(MySQLSchema)]
struct MySQLQueryTimeSchema {
    values: MySQLQueryTimeValue,
}

#[cfg(feature = "time")]
#[drizzle::test(mysql)]
fn mysql_relational_projection_decodes_time_without_serde(db: &mut TestDb<MySQLQueryTimeSchema>) {
    let MySQLQueryTimeSchema { values } = schema;
    let moment = time::Date::from_calendar_date(2026, time::Month::August, 26)
        .unwrap()
        .with_hms_micro(12, 34, 56, 789)
        .unwrap();
    db.insert(values)
        .value(InsertMySQLQueryTimeValue::new(moment).with_id(1))
        .execute();

    let projected = db
        .query(values)
        .columns(values.columns().moment())
        .find_first()
        .unwrap();
    assert_eq!(projected.moment, Some(moment));
}
