use std::borrow::Cow;

use drizzle_core::cte::{CTEDefinition, CTEView};
use drizzle_core::expr::{
    Agg, AggregateKind, CastTarget, Expr, GreatestLeastPolicy, NonNull, Null, Nullability, Scalar,
    cast, ceil, collate, concat, count, floor, greatest, group_concat, instr, is_distinct_from,
    is_not_distinct_from, least, left, ln, log, log2, log10, lpad, pi, random, raw_non_null,
    raw_nullable, repeat, reverse, right, round, round_to, rpad, sqrt, trunc,
};
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

#[test]
fn mysql_uses_positional_question_mark_parameters() {
    assert_eq!(SQL::param(MySQLTestValue).sql(), "?");
}

#[test]
fn mysql_string_expressions_use_mysql_syntax_and_safe_identifiers() {
    let concatenated = concat::<MySQLTestValue, _, _>("first", "last");
    let collated = collate::<MySQLTestValue, _>("name", "utf8mb4_0900_ai`ci");

    assert_eq!(concatenated.into_sql().sql(), "CONCAT(?, ?)");
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
    let cte = CTEView::<MySQLTestValue, _, _>::new((), "recent`users", SQL::raw("SELECT 1"));

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

fn assert_expr_shape<'a, E, SQLType, Nullable, Aggregate>(expr: E)
where
    E: Expr<'a, MySQLTestValue, SQLType = SQLType, Nullable = Nullable, Aggregate = Aggregate>,
    SQLType: drizzle_core::types::DataType,
    Nullable: Nullability,
    Aggregate: AggregateKind,
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

#[test]
fn mysql_rounding_uses_server_result_types_and_truncate_syntax() {
    let rounded = round::<MySQLTestValue, _>(42_u32);
    let rounded_to = round_to::<MySQLTestValue, _, _>(42_u32, 2_i32);
    let truncated = trunc::<MySQLTestValue, _>(42_u32);
    let decimal_floor =
        floor::<MySQLTestValue, _>(raw_non_null::<MySQLTestValue, types::Decimal>("price"));
    let float_ceil =
        ceil::<MySQLTestValue, _>(raw_non_null::<MySQLTestValue, types::Float>("ratio"));

    assert_expr_shape::<_, types::BigIntUnsigned, NonNull, Scalar>(rounded.clone());
    assert_expr_shape::<_, types::BigIntUnsigned, NonNull, Scalar>(rounded_to.clone());
    assert_expr_shape::<_, types::BigIntUnsigned, NonNull, Scalar>(truncated.clone());
    assert_expr_shape::<_, types::Decimal, NonNull, Scalar>(decimal_floor);
    assert_expr_shape::<_, types::Double, NonNull, Scalar>(float_ceil);

    assert_eq!(rounded.into_sql().sql(), "ROUND(?)");
    assert_eq!(rounded_to.into_sql().sql(), "ROUND(?, ?)");
    assert_eq!(truncated.into_sql().sql(), "TRUNCATE(?, 0)");
}

#[test]
fn mysql_cast_targets_match_cast_result_types() {
    fn assert_target<T, C>(target: C)
    where
        T: drizzle_core::types::DataType,
        C: CastTarget<'static, T, MySQLDialect>,
    {
        let _ = target;
    }

    assert_target::<types::BigInt, _>(types::BigInt);
    assert_target::<types::BigIntUnsigned, _>(types::BigIntUnsigned);
    assert_target::<types::Float, _>(types::Float);
    assert_target::<types::Double, _>(types::Double);
    assert_target::<types::Decimal, _>(types::Decimal);
    assert_target::<types::Varchar, _>(types::Varchar);
    assert_target::<types::Varbinary, _>(types::Varbinary);
    assert_target::<types::Json, _>(types::Json);
    assert_target::<types::Date, _>(types::Date);
    assert_target::<types::Time, _>(types::Time);
    assert_target::<types::DateTime, _>(types::DateTime);
    assert_target::<types::Year, _>(types::Year);

    let signed = cast::<MySQLTestValue, _, types::BigInt>(42_u32, types::BigInt);
    let text = cast::<MySQLTestValue, _, types::Varchar>(42_u32, types::Varchar);
    let binary = cast::<MySQLTestValue, _, types::Varbinary>("abc", types::Varbinary);
    let date = cast::<MySQLTestValue, _, types::Date>("not-a-date", types::Date);
    let year = cast::<MySQLTestValue, _, types::Year>("not-a-year", types::Year);

    assert_expr_shape::<_, types::BigInt, NonNull, Scalar>(signed.clone());
    assert_expr_shape::<_, types::Varchar, NonNull, Scalar>(text.clone());
    assert_expr_shape::<_, types::Varbinary, NonNull, Scalar>(binary.clone());
    assert_expr_shape::<_, types::Date, Null, Scalar>(date.clone());
    assert_expr_shape::<_, types::Year, Null, Scalar>(year.clone());
    assert_eq!(signed.into_sql().sql(), "CAST(? AS SIGNED)");
    assert_eq!(text.into_sql().sql(), "CAST(? AS CHAR)");
    assert_eq!(binary.into_sql().sql(), "CAST(? AS BINARY)");
    assert_eq!(date.into_sql().sql(), "CAST(? AS DATE)");
    assert_eq!(year.into_sql().sql(), "CAST(? AS YEAR)");
}

#[test]
fn mysql_native_math_string_and_aggregate_functions_are_available() {
    let pi_value = pi::<MySQLTestValue>();
    let random_value = random::<MySQLTestValue>();
    let log_value = log2::<MySQLTestValue, _>(8_i32);
    let square_root = sqrt::<MySQLTestValue, _>(4_i32);
    let natural_log = ln::<MySQLTestValue, _>(1_i32);
    let base_10_log = log10::<MySQLTestValue, _>(10_i32);
    let based_log = log::<MySQLTestValue, _, _>(2_i32, 8_i32);
    let position = instr::<MySQLTestValue, _, _>("foobar", "bar");
    let prefix = left::<MySQLTestValue, _, _>("foobar", 3_i32);
    let suffix = right::<MySQLTestValue, _, _>("foobar", 3_i32);
    let left_padded = lpad::<MySQLTestValue, _, _, _>("7", 3_i32, "0");
    let right_padded = rpad::<MySQLTestValue, _, _, _>("7", 3_i32, "0");
    let reversed = reverse::<MySQLTestValue, _>("abc");
    let repeated = repeat::<MySQLTestValue, _, _>("ab", 2_i32);
    let concatenated = group_concat::<MySQLTestValue, _>("name");
    let counted = count::<MySQLTestValue, _>(raw_non_null::<MySQLTestValue, types::Int>("1"));
    let nullable_position = instr::<MySQLTestValue, _, _>(
        "foobar",
        raw_nullable::<MySQLTestValue, types::Text>("needle"),
    );
    let nullable_prefix = left::<MySQLTestValue, _, _>(
        "foobar",
        raw_nullable::<MySQLTestValue, types::Int>("length"),
    );
    let nullable_padding = lpad::<MySQLTestValue, _, _, _>(
        "7",
        3_i32,
        raw_nullable::<MySQLTestValue, types::Text>("fill"),
    );
    let nullable_repeat =
        repeat::<MySQLTestValue, _, _>("ab", raw_nullable::<MySQLTestValue, types::Int>("times"));

    assert_expr_shape::<_, types::Double, NonNull, Scalar>(pi_value.clone());
    assert_expr_shape::<_, types::Double, NonNull, Scalar>(random_value.clone());
    assert_expr_shape::<_, types::Double, Null, Scalar>(log_value.clone());
    assert_expr_shape::<_, types::Double, Null, Scalar>(square_root);
    assert_expr_shape::<_, types::Double, Null, Scalar>(natural_log);
    assert_expr_shape::<_, types::Double, Null, Scalar>(base_10_log);
    assert_expr_shape::<_, types::Double, Null, Scalar>(based_log);
    assert_expr_shape::<_, types::BigInt, NonNull, Scalar>(position.clone());
    assert_expr_shape::<_, types::Text, NonNull, Scalar>(prefix.clone());
    assert_expr_shape::<_, types::Text, NonNull, Scalar>(suffix.clone());
    assert_expr_shape::<_, types::Text, NonNull, Scalar>(left_padded.clone());
    assert_expr_shape::<_, types::Text, NonNull, Scalar>(right_padded.clone());
    assert_expr_shape::<_, types::Text, NonNull, Scalar>(reversed.clone());
    assert_expr_shape::<_, types::Text, NonNull, Scalar>(repeated.clone());
    assert_expr_shape::<_, types::Text, Null, Agg>(concatenated.clone());
    assert_expr_shape::<_, types::BigInt, NonNull, Agg>(counted.clone());
    assert_expr_shape::<_, types::BigInt, Null, Scalar>(nullable_position);
    assert_expr_shape::<_, types::Text, Null, Scalar>(nullable_prefix);
    assert_expr_shape::<_, types::Text, Null, Scalar>(nullable_padding);
    assert_expr_shape::<_, types::Text, Null, Scalar>(nullable_repeat);

    assert_eq!(pi_value.into_sql().sql(), "PI()");
    assert_eq!(random_value.into_sql().sql(), "RAND()");
    assert_eq!(log_value.into_sql().sql(), "LOG2(?)");
    assert_eq!(position.into_sql().sql(), "INSTR(?, ?)");
    assert_eq!(prefix.into_sql().sql(), "LEFT(?, ?)");
    assert_eq!(suffix.into_sql().sql(), "RIGHT(?, ?)");
    assert_eq!(left_padded.into_sql().sql(), "LPAD(?, ?, ?)");
    assert_eq!(right_padded.into_sql().sql(), "RPAD(?, ?, ?)");
    assert_eq!(reversed.into_sql().sql(), "REVERSE(?)");
    assert_eq!(repeated.into_sql().sql(), "REPEAT(?, ?)");
    assert_eq!(concatenated.into_sql().sql(), "GROUP_CONCAT(?)");
    assert_eq!(counted.into_sql().sql(), "COUNT(1)");
}

#[test]
fn greatest_and_least_use_mysql_null_propagation() {
    fn assert_nullability_policy<D, L, R, N>()
    where
        D: GreatestLeastPolicy<L, R, Nullable = N>,
        L: Nullability,
        R: Nullability,
        N: Nullability,
    {
    }

    assert_nullability_policy::<MySQLDialect, NonNull, Null, Null>();
    assert_nullability_policy::<drizzle_core::PostgresDialect, NonNull, Null, NonNull>();

    let greatest_value = greatest::<MySQLTestValue, _, _>(
        1_i32,
        raw_nullable::<MySQLTestValue, types::Int>("nullable_score"),
    );
    let least_value = least::<MySQLTestValue, _, _>(1_i32, 2_i32);

    assert_expr_shape::<_, types::Int, Null, Scalar>(greatest_value.clone());
    assert_expr_shape::<_, types::Int, NonNull, Scalar>(least_value.clone());
    assert_eq!(
        greatest_value.into_sql().sql(),
        "GREATEST(?, nullable_score)"
    );
    assert_eq!(least_value.into_sql().sql(), "LEAST(?, ?)");
}
