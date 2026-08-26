use std::borrow::Cow;

use drizzle_core::expr::{Expr, collate, concat, is_distinct_from, is_not_distinct_from};
use drizzle_core::row::SQLTypeToRust;
use drizzle_core::{Dialect, MySQLDialect, SQL, SQLParam, ValueTypeForDialect};
use drizzle_types::mysql::types;
use drizzle_types::sql::Assignable;

#[derive(Clone, Debug)]
struct MySQLTestValue;

impl SQLParam for MySQLTestValue {
    const DIALECT: Dialect = Dialect::MySQL;
    type DialectMarker = MySQLDialect;
}

impl From<MySQLTestValue> for Cow<'_, MySQLTestValue> {
    fn from(value: MySQLTestValue) -> Self {
        Cow::Owned(value)
    }
}

impl From<i32> for MySQLTestValue {
    fn from(_: i32) -> Self {
        Self
    }
}

impl From<u32> for MySQLTestValue {
    fn from(_: u32) -> Self {
        Self
    }
}

impl From<u64> for MySQLTestValue {
    fn from(_: u64) -> Self {
        Self
    }
}

impl From<&str> for MySQLTestValue {
    fn from(_: &str) -> Self {
        Self
    }
}

drizzle_core::impl_cte_types!(value_type: MySQLTestValue);

#[test]
fn mysql_uses_positional_question_mark_parameters() {
    assert_eq!(SQL::param(MySQLTestValue).sql(), "?");
}

#[test]
fn mysql_string_expressions_use_mysql_syntax_and_safe_identifiers() {
    let concatenated = concat::<MySQLTestValue, _, _>("first", "last");
    let collated = collate::<MySQLTestValue, _>("name", "utf8mb4_0900_ai`ci");

    assert_eq!(concatenated.into_sql().sql(), "CONCAT (?, ?)");
    assert_eq!(
        collated.into_sql().sql(),
        "(? COLLATE `utf8mb4_0900_ai``ci`)"
    );
}

#[test]
fn mysql_null_safe_comparisons_use_spaceship_operator() {
    let distinct = is_distinct_from::<MySQLTestValue, _, _>(1_i32, 2_i32);
    let same = is_not_distinct_from::<MySQLTestValue, _, _>(1_i32, 2_i32);

    assert_eq!(distinct.into_sql().sql(), "NOT (? <=> ?)");
    assert_eq!(same.into_sql().sql(), "? <=> ?");
}

#[test]
fn mysql_cte_names_are_quoted_as_identifiers() {
    let cte = CTEView::new((), "recent`users", SQL::raw("SELECT 1"));

    assert_eq!(cte.cte_definition().sql(), "`recent``users` AS (SELECT 1)");
}

#[test]
fn mysql_aliases_escape_embedded_backticks() {
    let sql = SQL::<MySQLTestValue>::raw("1").alias("value`alias");
    assert_eq!(sql.sql(), "1 AS `value``alias`");
}

fn assert_bind_type<T, SQLType>()
where
    T: ValueTypeForDialect<MySQLDialect, SQLType = SQLType>,
    SQLType: drizzle_core::types::DataType,
{
}

fn assert_row_type<SQLType, RustType>()
where
    SQLType: drizzle_core::types::DataType + SQLTypeToRust<MySQLDialect, RustType = RustType>,
{
}

fn assert_row_bind_round_trip<SQLType>()
where
    SQLType: drizzle_core::types::DataType + SQLTypeToRust<MySQLDialect>,
    <SQLType as SQLTypeToRust<MySQLDialect>>::RustType: ValueTypeForDialect<MySQLDialect>,
    SQLType:
        Assignable<
            <<SQLType as SQLTypeToRust<MySQLDialect>>::RustType as ValueTypeForDialect<
                MySQLDialect,
            >>::SQLType,
        >,
{
}

fn assert_expr_type<'a, E, SQLType>(expr: &'a E)
where
    E: Expr<'a, MySQLTestValue, SQLType = SQLType>,
    SQLType: drizzle_core::types::DataType,
{
    let _ = expr;
}

#[test]
fn mysql_bind_and_row_mappings_preserve_integer_signedness() {
    assert_bind_type::<i8, types::TinyInt>();
    assert_bind_type::<u8, types::TinyIntUnsigned>();
    assert_bind_type::<i32, types::Int>();
    assert_bind_type::<u32, types::IntUnsigned>();
    assert_bind_type::<i64, types::BigInt>();
    assert_bind_type::<u64, types::BigIntUnsigned>();

    assert_row_type::<types::TinyInt, i8>();
    assert_row_type::<types::TinyIntUnsigned, u8>();
    assert_row_type::<types::Int, i32>();
    assert_row_type::<types::IntUnsigned, u32>();
    assert_row_type::<types::BigInt, i64>();
    assert_row_type::<types::BigIntUnsigned, u64>();

    assert_expr_type::<_, types::IntUnsigned>(&42_u32);
    assert_expr_type::<_, types::BigIntUnsigned>(&u64::MAX);
}

#[test]
fn mysql_selected_row_mappings_cover_native_families() {
    assert_row_type::<types::Boolean, bool>();
    assert_row_type::<types::Double, f64>();
    assert_row_type::<types::Text, String>();
    assert_row_type::<types::Blob, Vec<u8>>();
    assert_row_type::<types::Year, u16>();
}

#[test]
fn mysql_selected_values_can_be_bound_back_to_their_source_type() {
    assert_row_bind_round_trip::<types::TinyInt>();
    assert_row_bind_round_trip::<types::TinyIntUnsigned>();
    assert_row_bind_round_trip::<types::SmallInt>();
    assert_row_bind_round_trip::<types::SmallIntUnsigned>();
    assert_row_bind_round_trip::<types::MediumInt>();
    assert_row_bind_round_trip::<types::MediumIntUnsigned>();
    assert_row_bind_round_trip::<types::Int>();
    assert_row_bind_round_trip::<types::IntUnsigned>();
    assert_row_bind_round_trip::<types::BigInt>();
    assert_row_bind_round_trip::<types::BigIntUnsigned>();
    assert_row_bind_round_trip::<types::Float>();
    assert_row_bind_round_trip::<types::Double>();
    assert_row_bind_round_trip::<types::Decimal>();
    assert_row_bind_round_trip::<types::Boolean>();
    assert_row_bind_round_trip::<types::Char>();
    assert_row_bind_round_trip::<types::Varchar>();
    assert_row_bind_round_trip::<types::TinyText>();
    assert_row_bind_round_trip::<types::Text>();
    assert_row_bind_round_trip::<types::MediumText>();
    assert_row_bind_round_trip::<types::LongText>();
    assert_row_bind_round_trip::<types::Binary>();
    assert_row_bind_round_trip::<types::Varbinary>();
    assert_row_bind_round_trip::<types::TinyBlob>();
    assert_row_bind_round_trip::<types::Blob>();
    assert_row_bind_round_trip::<types::MediumBlob>();
    assert_row_bind_round_trip::<types::LongBlob>();
    assert_row_bind_round_trip::<types::Json>();
    assert_row_bind_round_trip::<types::Date>();
    assert_row_bind_round_trip::<types::Time>();
    assert_row_bind_round_trip::<types::DateTime>();
    assert_row_bind_round_trip::<types::Timestamp>();
    assert_row_bind_round_trip::<types::Year>();
    assert_row_bind_round_trip::<types::Enum>();
    assert_row_bind_round_trip::<types::Set>();
    assert_row_bind_round_trip::<types::Bit>();
    assert_row_bind_round_trip::<types::Any>();
}
