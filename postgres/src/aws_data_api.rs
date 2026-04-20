//! AWS Aurora Serverless Data API driver support.
//!
//! Aurora Data API (`aws-sdk-rdsdata`) is an HTTP-based Postgres/MySQL driver
//! that returns rows as pre-decoded [`Field`] enums rather than the wire format
//! used by `postgres` / `tokio-postgres`. We therefore cannot reuse
//! `postgres-types::FromSql` — this module provides a dedicated [`Row`] type
//! and [`FromDrizzleRow`] leaf impls that match on `Field` variants directly.
//!
//! # Wire model
//!
//! * Requests: each param is encoded as [`SqlParameter`] = `{name, value: Field,
//!   type_hint: Option<TypeHint>}`. [`encode_param`] does this from
//!   [`PostgresValue`].
//! * Responses: each row is `Vec<Field>` matched positionally against
//!   column metadata.
//! * NULLs are signalled by `Field::IsNull(true)` rather than a sentinel value.
//!
//! # Type hints
//!
//! For values that travel as `StringValue` but need server-side coercion
//! (UUID, JSON, TIMESTAMP, DECIMAL, DATE, TIME) we emit a [`TypeHint`]. This
//! mirrors upstream `drizzle-orm/src/aws-data-api/common/index.ts::toValueParam`.

#![cfg(feature = "aws-data-api")]

use crate::prelude::*;
use crate::values::PostgresValue;
use aws_sdk_rdsdata::primitives::Blob;
use aws_sdk_rdsdata::types::{ArrayValue, ColumnMetadata, Field, SqlParameter, TypeHint};
use drizzle_core::error::DrizzleError;
use drizzle_core::row::{FromDrizzleRow, NullProbeRow};

// =============================================================================
// Row
// =============================================================================

/// A row returned from the AWS Aurora Data API.
///
/// Wraps the `Vec<Field>` values (one per column) alongside an `Arc<[ColumnMetadata]>`
/// shared across every row from the same result set. Cloning a [`Row`] only
/// clones the field vector — metadata is ref-counted.
#[derive(Debug, Clone)]
pub struct Row {
    fields: Vec<Field>,
    metadata: Arc<[ColumnMetadata]>,
}

impl Row {
    /// Construct a row from a raw `Vec<Field>` and column metadata.
    #[must_use]
    pub const fn new(fields: Vec<Field>, metadata: Arc<[ColumnMetadata]>) -> Self {
        Self { fields, metadata }
    }

    /// Number of columns in this row.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether the row has no columns.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Raw `Field` slice.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Column metadata (shared).
    #[must_use]
    pub fn metadata(&self) -> &[ColumnMetadata] {
        &self.metadata
    }

    /// Column name at `offset`, if known.
    #[must_use]
    pub fn column_name(&self, offset: usize) -> Option<&str> {
        self.metadata.get(offset).and_then(|m| m.name.as_deref())
    }

    /// Typed accessor — delegates to `FromDrizzleRow`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when the field at `offset`
    /// cannot be decoded into `T`.
    pub fn try_get<T>(&self, offset: usize) -> Result<T, DrizzleError>
    where
        T: FromDrizzleRow<Self>,
    {
        T::from_row_at(self, offset)
    }
}

/// Build a `Row` from the raw AWS Data API response pieces.
#[must_use]
pub const fn row_from_parts(values: Vec<Field>, metadata: Arc<[ColumnMetadata]>) -> Row {
    Row::new(values, metadata)
}

// =============================================================================
// Decode helpers
// =============================================================================

/// Extract `Field` at `offset`, erroring if the row is too short.
#[inline]
fn field_at(row: &Row, offset: usize) -> Result<&Field, DrizzleError> {
    row.fields.get(offset).ok_or_else(|| {
        DrizzleError::ConversionError(
            format!(
                "AWS Data API row has {} columns; tried to read column {}",
                row.fields.len(),
                offset
            )
            .into(),
        )
    })
}

/// `true` if the field is an explicit NULL sentinel.
#[inline]
const fn field_is_null(field: &Field) -> bool {
    matches!(field, Field::IsNull(true))
}

fn unexpected<T>(field: &Field, expected: &'static str) -> Result<T, DrizzleError> {
    Err(DrizzleError::ConversionError(
        format!("AWS Data API: expected {expected}, got {field:?}").into(),
    ))
}

fn null_error<T>(expected: &'static str) -> Result<T, DrizzleError> {
    Err(DrizzleError::ConversionError(
        format!("AWS Data API: NULL value for non-nullable column of type {expected}").into(),
    ))
}

#[inline]
fn expect_long(field: &Field) -> Result<i64, DrizzleError> {
    match field {
        Field::LongValue(v) => Ok(*v),
        Field::IsNull(true) => null_error("integer"),
        _ => unexpected(field, "LongValue"),
    }
}

#[inline]
fn expect_double(field: &Field) -> Result<f64, DrizzleError> {
    match field {
        Field::DoubleValue(v) => Ok(*v),
        // PostgreSQL real/double stored but returned as long when integer-valued.
        Field::LongValue(v) => format!("{v}").parse::<f64>().map_err(|e| {
            DrizzleError::ConversionError(format!("Cannot convert i64 to f64: {e}").into())
        }),
        Field::IsNull(true) => null_error("float"),
        _ => unexpected(field, "DoubleValue"),
    }
}

#[inline]
fn double_to_f32(v: f64) -> Result<f32, DrizzleError> {
    format!("{v}").parse::<f32>().map_err(|e| {
        DrizzleError::ConversionError(format!("Cannot convert f64 to f32: {e}").into())
    })
}

#[inline]
fn expect_bool(field: &Field) -> Result<bool, DrizzleError> {
    match field {
        Field::BooleanValue(v) => Ok(*v),
        Field::IsNull(true) => null_error("bool"),
        _ => unexpected(field, "BooleanValue"),
    }
}

#[inline]
fn expect_string(field: &Field) -> Result<&str, DrizzleError> {
    match field {
        Field::StringValue(s) => Ok(s.as_str()),
        Field::IsNull(true) => null_error("string"),
        _ => unexpected(field, "StringValue"),
    }
}

#[inline]
fn expect_blob(field: &Field) -> Result<&[u8], DrizzleError> {
    match field {
        Field::BlobValue(b) => Ok(b.as_ref()),
        Field::IsNull(true) => null_error("blob"),
        _ => unexpected(field, "BlobValue"),
    }
}

#[inline]
fn expect_array(field: &Field) -> Result<&ArrayValue, DrizzleError> {
    match field {
        Field::ArrayValue(a) => Ok(a),
        Field::IsNull(true) => null_error("array"),
        _ => unexpected(field, "ArrayValue"),
    }
}

// =============================================================================
// NullProbeRow — required for composite Option<T> (LEFT JOIN support)
// =============================================================================

/// Null-probe at the given offset — checks just the first field of a composite.
fn null_probe(row: &Row, offset: usize) -> Result<bool, DrizzleError> {
    let field = field_at(row, offset)?;
    Ok(field_is_null(field))
}

// =============================================================================
// FromDrizzleRow leaf impls — scalar types
// =============================================================================

/// Generate `FromDrizzleRow<Row>` for a numeric type using `expect_long` with
/// `TryFrom<i64>` narrowing (so i8/i16/i32/u8/.. all share the same body).
macro_rules! impl_int_leaf {
    ($($ty:ty),+ $(,)?) => { $(
        impl FromDrizzleRow<Row> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
                let v = expect_long(field_at(row, offset)?)?;
                <$ty>::try_from(v).map_err(|e| {
                    DrizzleError::ConversionError(
                        format!("AWS Data API: int {v} does not fit {}: {e}", stringify!($ty))
                            .into(),
                    )
                })
            }
        }

        impl FromDrizzleRow<Row> for Option<$ty> {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
                let field = field_at(row, offset)?;
                if field_is_null(field) {
                    return Ok(None);
                }
                let v = expect_long(field)?;
                <$ty>::try_from(v)
                    .map(Some)
                    .map_err(|e| {
                        DrizzleError::ConversionError(
                            format!(
                                "AWS Data API: int {v} does not fit {}: {e}",
                                stringify!($ty)
                            )
                            .into(),
                        )
                    })
            }
        }
    )+ };
}

impl_int_leaf!(i8, i16, i32, i64, u8, u16, u32);

// f32/f64

impl FromDrizzleRow<Row> for f64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        expect_double(field_at(row, offset)?)
    }
}

impl FromDrizzleRow<Row> for Option<f64> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        let field = field_at(row, offset)?;
        if field_is_null(field) {
            return Ok(None);
        }
        expect_double(field).map(Some)
    }
}

impl FromDrizzleRow<Row> for f32 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        double_to_f32(expect_double(field_at(row, offset)?)?)
    }
}

impl FromDrizzleRow<Row> for Option<f32> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        let field = field_at(row, offset)?;
        if field_is_null(field) {
            return Ok(None);
        }
        expect_double(field).and_then(double_to_f32).map(Some)
    }
}

// bool

impl FromDrizzleRow<Row> for bool {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        expect_bool(field_at(row, offset)?)
    }
}

impl FromDrizzleRow<Row> for Option<bool> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        let field = field_at(row, offset)?;
        if field_is_null(field) {
            return Ok(None);
        }
        expect_bool(field).map(Some)
    }
}

// String

impl FromDrizzleRow<Row> for String {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        expect_string(field_at(row, offset)?).map(ToString::to_string)
    }
}

impl FromDrizzleRow<Row> for Option<String> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        let field = field_at(row, offset)?;
        if field_is_null(field) {
            return Ok(None);
        }
        expect_string(field).map(|s| Some(s.to_string()))
    }
}

// Vec<u8>

impl FromDrizzleRow<Row> for Vec<u8> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        expect_blob(field_at(row, offset)?).map(<[u8]>::to_vec)
    }
}

impl FromDrizzleRow<Row> for Option<Vec<u8>> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        let field = field_at(row, offset)?;
        if field_is_null(field) {
            return Ok(None);
        }
        expect_blob(field).map(|b| Some(b.to_vec()))
    }
}

// =============================================================================
// Feature-gated scalar leaves
// =============================================================================

#[cfg(feature = "uuid")]
mod uuid_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
    };

    impl FromDrizzleRow<Row> for uuid::Uuid {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            Self::parse_str(s).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API uuid: {e}").into())
            })
        }
    }

    impl FromDrizzleRow<Row> for Option<uuid::Uuid> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            uuid::Uuid::parse_str(s).map(Some).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API uuid: {e}").into())
            })
        }
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
    };

    impl FromDrizzleRow<Row> for serde_json::Value {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            serde_json::from_str(s).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API json: {e}").into())
            })
        }
    }

    impl FromDrizzleRow<Row> for Option<serde_json::Value> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            serde_json::from_str(s).map(Some).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API json: {e}").into())
            })
        }
    }
}

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
        parse_interval_seconds,
    };
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    fn parse<T, E>(
        s: &str,
        kind: &'static str,
        f: impl FnOnce(&str) -> Result<T, E>,
    ) -> Result<T, DrizzleError>
    where
        E: core::fmt::Display,
    {
        f(s).map_err(|e| DrizzleError::ConversionError(format!("AWS Data API {kind}: {e}").into()))
    }

    impl FromDrizzleRow<Row> for NaiveDate {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            parse(s, "date", |v| Self::parse_from_str(v, "%Y-%m-%d"))
        }
    }

    impl FromDrizzleRow<Row> for Option<NaiveDate> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            parse(s, "date", |v| NaiveDate::parse_from_str(v, "%Y-%m-%d")).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for NaiveTime {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            parse(s, "time", |v| Self::parse_from_str(v, "%H:%M:%S%.f"))
        }
    }

    impl FromDrizzleRow<Row> for Option<NaiveTime> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            parse(s, "time", |v| NaiveTime::parse_from_str(v, "%H:%M:%S%.f")).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for NaiveDateTime {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            // AWS Data API returns timestamps without TZ as "YYYY-MM-DD HH:MM:SS[.fff]"
            parse(s, "timestamp", |v| {
                Self::parse_from_str(v, "%Y-%m-%d %H:%M:%S%.f")
                    .or_else(|_| Self::parse_from_str(v, "%Y-%m-%d %H:%M:%S"))
            })
        }
    }

    impl FromDrizzleRow<Row> for Option<NaiveDateTime> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            parse(s, "timestamp", |v| {
                NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S%.f")
                    .or_else(|_| NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S"))
            })
            .map(Some)
        }
    }

    impl FromDrizzleRow<Row> for DateTime<Utc> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            // AWS returns ISO-8601 for timestamptz. Fall back to naive + assume UTC
            // when no timezone is present (mirroring tokio-postgres behaviour).
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Ok(dt.with_timezone(&Utc));
            }
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
                return Ok(naive.and_utc());
            }
            Err(DrizzleError::ConversionError(
                format!("AWS Data API timestamptz: unparseable {s:?}").into(),
            ))
        }
    }

    impl FromDrizzleRow<Row> for Option<DateTime<Utc>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <DateTime<Utc> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }

    use chrono::FixedOffset;

    impl FromDrizzleRow<Row> for DateTime<FixedOffset> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            if let Ok(dt) = Self::parse_from_rfc3339(s) {
                return Ok(dt);
            }
            // Postgres can return "YYYY-MM-DD HH:MM:SS[.fff]+HH" — try common forms.
            for fmt in &[
                "%Y-%m-%d %H:%M:%S%.f%:z",
                "%Y-%m-%d %H:%M:%S%.f%z",
                "%Y-%m-%d %H:%M:%S%:z",
                "%Y-%m-%d %H:%M:%S%z",
            ] {
                if let Ok(dt) = Self::parse_from_str(s, fmt) {
                    return Ok(dt);
                }
            }
            // Tz-less fallback → assume UTC (mirrors tokio-postgres behaviour).
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
                return Ok(naive.and_utc().fixed_offset());
            }
            Err(DrizzleError::ConversionError(
                format!("AWS Data API timestamptz: unparseable {s:?}").into(),
            ))
        }
    }

    impl FromDrizzleRow<Row> for Option<DateTime<FixedOffset>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <DateTime<FixedOffset> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for chrono::Duration {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            parse_interval_seconds(s).map(Self::seconds)
        }
    }

    impl FromDrizzleRow<Row> for Option<chrono::Duration> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            parse_interval_seconds(s)
                .map(chrono::Duration::seconds)
                .map(Some)
        }
    }
}

/// Parse a Postgres INTERVAL string into total seconds.
///
/// Accepts the canonical `{n} seconds` form we emit in [`encode_field`], plus
/// the `HH:MM:SS[.fff]` / `DD HH:MM:SS` / `N days HH:MM:SS` shapes that Aurora
/// returns through the Data API. Years/months are rejected (ambiguous length)
/// — users needing full interval parsing should decode as `String`.
#[cfg(any(feature = "chrono", feature = "time"))]
fn parse_interval_seconds(s: &str) -> Result<i64, DrizzleError> {
    let s = s.trim();
    // Fast path: "{n} seconds" / "{n} second"
    if let Some(rest) = s
        .strip_suffix(" seconds")
        .or_else(|| s.strip_suffix(" second"))
    {
        return rest.trim().parse::<i64>().map_err(|e| {
            DrizzleError::ConversionError(format!("AWS Data API interval: {e}").into())
        });
    }

    let mut total: i64 = 0;
    let mut rest = s;
    // Handle "N days [HH:MM:SS]"
    if let Some(idx) = rest.find(" day") {
        let (days_part, after) = rest.split_at(idx);
        let days: i64 = days_part.trim().parse().map_err(|e| {
            DrizzleError::ConversionError(format!("AWS Data API interval days: {e}").into())
        })?;
        total += days * 86_400;
        rest = after
            .trim_start_matches(" days")
            .trim_start_matches(" day")
            .trim();
    }
    if rest.is_empty() {
        return Ok(total);
    }
    // Remaining must be HH:MM:SS[.fff], with optional leading '-'
    let (sign, body): (i64, &str) = rest.strip_prefix('-').map_or((1, rest), |r| (-1, r));
    let mut parts = body.split(':');
    let h: i64 = parts
        .next()
        .ok_or_else(|| DrizzleError::ConversionError("interval: missing hours".into()))?
        .parse()
        .map_err(|e| DrizzleError::ConversionError(format!("interval hours: {e}").into()))?;
    let m: i64 = parts
        .next()
        .ok_or_else(|| DrizzleError::ConversionError("interval: missing minutes".into()))?
        .parse()
        .map_err(|e| DrizzleError::ConversionError(format!("interval minutes: {e}").into()))?;
    let sec_field = parts
        .next()
        .ok_or_else(|| DrizzleError::ConversionError("interval: missing seconds".into()))?;
    // Drop any fractional part — we return whole seconds.
    let sec_int = sec_field.split('.').next().unwrap_or("0");
    let sec: i64 = sec_int
        .parse()
        .map_err(|e| DrizzleError::ConversionError(format!("interval seconds: {e}").into()))?;
    total += sign * (h * 3600 + m * 60 + sec);
    Ok(total)
}

#[cfg(feature = "time")]
mod time_impls {
    //! Hand-rolled parsers for `time` types. We intentionally avoid
    //! `time::macros::format_description!` so we don't have to depend on the
    //! optional `macros` feature of the `time` crate — the workspace only
    //! enables `formatting` + `parsing`. The formats AWS emits are simple
    //! enough that manual digit-splitting is fine.
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
        parse_interval_seconds,
    };
    use time::format_description::well_known::Rfc3339;
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

    fn digits<T>(s: &str, kind: &'static str) -> Result<T, DrizzleError>
    where
        T: core::str::FromStr,
        T::Err: core::fmt::Display,
    {
        s.parse::<T>()
            .map_err(|e| DrizzleError::ConversionError(format!("AWS Data API {kind}: {e}").into()))
    }

    fn month_from_u8(m: u8) -> Result<Month, DrizzleError> {
        Month::try_from(m)
            .map_err(|e| DrizzleError::ConversionError(format!("AWS Data API month: {e}").into()))
    }

    /// Parse `YYYY-MM-DD`.
    fn parse_date(s: &str) -> Result<Date, DrizzleError> {
        let mut parts = s.splitn(3, '-');
        let y = digits::<i32>(
            parts
                .next()
                .ok_or_else(|| DrizzleError::ConversionError("date: empty".into()))?,
            "date year",
        )?;
        let m = digits::<u8>(
            parts
                .next()
                .ok_or_else(|| DrizzleError::ConversionError("date: missing month".into()))?,
            "date month",
        )?;
        let d = digits::<u8>(
            parts
                .next()
                .ok_or_else(|| DrizzleError::ConversionError("date: missing day".into()))?,
            "date day",
        )?;
        Date::from_calendar_date(y, month_from_u8(m)?, d)
            .map_err(|e| DrizzleError::ConversionError(format!("AWS Data API date: {e}").into()))
    }

    /// Parse `HH:MM:SS[.fff]`.
    fn parse_time(s: &str) -> Result<Time, DrizzleError> {
        let mut parts = s.splitn(3, ':');
        let h = digits::<u8>(
            parts
                .next()
                .ok_or_else(|| DrizzleError::ConversionError("time: empty".into()))?,
            "time hour",
        )?;
        let m = digits::<u8>(
            parts
                .next()
                .ok_or_else(|| DrizzleError::ConversionError("time: missing minute".into()))?,
            "time minute",
        )?;
        let sec_field = parts
            .next()
            .ok_or_else(|| DrizzleError::ConversionError("time: missing second".into()))?;
        let (sec_str, nanos) = match sec_field.split_once('.') {
            Some((sec, frac)) => {
                // Pad/truncate fractional to 9 digits (nanoseconds).
                let mut buf = [b'0'; 9];
                for (i, b) in frac.bytes().take(9).enumerate() {
                    buf[i] = b;
                }
                let nanos: u32 = core::str::from_utf8(&buf)
                    .map_err(|e| {
                        DrizzleError::ConversionError(format!("time frac utf8: {e}").into())
                    })?
                    .parse()
                    .map_err(|e: core::num::ParseIntError| {
                        DrizzleError::ConversionError(format!("time frac: {e}").into())
                    })?;
                (sec, nanos)
            }
            None => (sec_field, 0u32),
        };
        let sec = digits::<u8>(sec_str, "time second")?;
        Time::from_hms_nano(h, m, sec, nanos)
            .map_err(|e| DrizzleError::ConversionError(format!("AWS Data API time: {e}").into()))
    }

    /// Parse `YYYY-MM-DD HH:MM:SS[.fff]`.
    fn parse_primitive(s: &str) -> Result<PrimitiveDateTime, DrizzleError> {
        let (date, time) = s.split_once(' ').ok_or_else(|| {
            DrizzleError::ConversionError(format!("timestamp: missing space in {s:?}").into())
        })?;
        Ok(PrimitiveDateTime::new(parse_date(date)?, parse_time(time)?))
    }

    impl FromDrizzleRow<Row> for Date {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            parse_date(expect_string(field_at(row, offset)?)?)
        }
    }

    impl FromDrizzleRow<Row> for Option<Date> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            parse_date(expect_string(field)?).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for Time {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            parse_time(expect_string(field_at(row, offset)?)?)
        }
    }

    impl FromDrizzleRow<Row> for Option<Time> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            parse_time(expect_string(field)?).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for PrimitiveDateTime {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            parse_primitive(expect_string(field_at(row, offset)?)?)
        }
    }

    impl FromDrizzleRow<Row> for Option<PrimitiveDateTime> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            parse_primitive(expect_string(field)?).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for OffsetDateTime {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            if let Ok(dt) = Self::parse(s, &Rfc3339) {
                return Ok(dt);
            }
            // No TZ → assume UTC, mirroring tokio-postgres behaviour.
            if let Ok(naive) = parse_primitive(s) {
                return Ok(naive.assume_utc());
            }
            Err(DrizzleError::ConversionError(
                format!("AWS Data API timestamptz: unparseable {s:?}").into(),
            ))
        }
    }

    impl FromDrizzleRow<Row> for Option<OffsetDateTime> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <OffsetDateTime as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for time::Duration {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            parse_interval_seconds(s).map(Self::seconds)
        }
    }

    impl FromDrizzleRow<Row> for Option<time::Duration> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            parse_interval_seconds(expect_string(field)?)
                .map(time::Duration::seconds)
                .map(Some)
        }
    }
}

#[cfg(feature = "rust-decimal")]
mod decimal_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
    };
    use core::str::FromStr;

    impl FromDrizzleRow<Row> for rust_decimal::Decimal {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            Self::from_str(s).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API decimal: {e}").into())
            })
        }
    }

    impl FromDrizzleRow<Row> for Option<rust_decimal::Decimal> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            rust_decimal::Decimal::from_str(s).map(Some).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API decimal: {e}").into())
            })
        }
    }
}

// =============================================================================
// Array impls — decode Field::ArrayValue into Vec<T>
// =============================================================================

/// Decode a single `ArrayValue` variant into `Vec<T>`. Nested arrays are not
/// supported here (flat arrays only — the common case for Aurora Data API).
fn decode_array_i64(array: &ArrayValue) -> Result<Vec<i64>, DrizzleError> {
    match array {
        ArrayValue::LongValues(v) => Ok(v.clone()),
        other => Err(DrizzleError::ConversionError(
            format!("AWS Data API array: expected LongValues, got {other:?}").into(),
        )),
    }
}

fn decode_array_f64(array: &ArrayValue) -> Result<Vec<f64>, DrizzleError> {
    match array {
        ArrayValue::DoubleValues(v) => Ok(v.clone()),
        other => Err(DrizzleError::ConversionError(
            format!("AWS Data API array: expected DoubleValues, got {other:?}").into(),
        )),
    }
}

fn decode_array_bool(array: &ArrayValue) -> Result<Vec<bool>, DrizzleError> {
    match array {
        ArrayValue::BooleanValues(v) => Ok(v.clone()),
        other => Err(DrizzleError::ConversionError(
            format!("AWS Data API array: expected BooleanValues, got {other:?}").into(),
        )),
    }
}

fn decode_array_string(array: &ArrayValue) -> Result<Vec<String>, DrizzleError> {
    match array {
        ArrayValue::StringValues(v) => Ok(v.clone()),
        other => Err(DrizzleError::ConversionError(
            format!("AWS Data API array: expected StringValues, got {other:?}").into(),
        )),
    }
}

macro_rules! impl_array_leaf {
    ($ty:ty, $decode:ident, $transform:expr) => {
        impl FromDrizzleRow<Row> for Vec<$ty> {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
                let field = field_at(row, offset)?;
                if field_is_null(field) {
                    return null_error::<Self>("array");
                }
                let arr = expect_array(field)?;
                let raw = $decode(arr)?;
                raw.into_iter().map($transform).collect()
            }
        }

        impl FromDrizzleRow<Row> for Option<Vec<$ty>> {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
                let field = field_at(row, offset)?;
                if field_is_null(field) {
                    return Ok(None);
                }
                let arr = expect_array(field)?;
                let raw = $decode(arr)?;
                let out: Result<Vec<$ty>, DrizzleError> = raw.into_iter().map($transform).collect();
                out.map(Some)
            }
        }
    };
}

// Narrowed integer arrays reuse decode_array_i64 + TryFrom
impl_array_leaf!(i64, decode_array_i64, |v: i64| Ok::<i64, DrizzleError>(v));
impl_array_leaf!(i32, decode_array_i64, |v: i64| i32::try_from(v).map_err(
    |e| DrizzleError::ConversionError(format!("i32 narrow: {e}").into())
));
impl_array_leaf!(i16, decode_array_i64, |v: i64| i16::try_from(v).map_err(
    |e| DrizzleError::ConversionError(format!("i16 narrow: {e}").into())
));

impl_array_leaf!(f64, decode_array_f64, |v: f64| Ok::<f64, DrizzleError>(v));
impl_array_leaf!(f32, decode_array_f64, |v: f64| double_to_f32(v));
impl_array_leaf!(bool, decode_array_bool, |v: bool| Ok::<bool, DrizzleError>(
    v
));
impl_array_leaf!(String, decode_array_string, |v: String| Ok::<
    String,
    DrizzleError,
>(v));

// =============================================================================
// Wrapper-type leaf impls — delegate through String / Vec<u8>
// =============================================================================
//
// The base `postgres` / `tokio-postgres` backends pick up FromSql impls from
// the respective crates (compact-str, arrayvec, smallvec, etc.) automatically.
// The Data API returns pre-decoded `Field` variants, so we decode to the
// canonical scalar then convert.

#[cfg(feature = "arrayvec")]
mod arrayvec_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_blob, expect_string, field_at, field_is_null,
        format,
    };

    impl<const N: usize> FromDrizzleRow<Row> for arrayvec::ArrayString<N> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            Self::from(s).map_err(|_| {
                DrizzleError::ConversionError(
                    format!(
                        "AWS Data API: string length {} exceeds ArrayString capacity {}",
                        s.len(),
                        N
                    )
                    .into(),
                )
            })
        }
    }

    impl<const N: usize> FromDrizzleRow<Row> for Option<arrayvec::ArrayString<N>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <arrayvec::ArrayString<N> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }

    impl<const N: usize> FromDrizzleRow<Row> for arrayvec::ArrayVec<u8, N> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let bytes = expect_blob(field_at(row, offset)?)?;
            Self::try_from(bytes).map_err(|_| {
                DrizzleError::ConversionError(
                    format!(
                        "AWS Data API: byte length {} exceeds ArrayVec capacity {}",
                        bytes.len(),
                        N
                    )
                    .into(),
                )
            })
        }
    }

    impl<const N: usize> FromDrizzleRow<Row> for Option<arrayvec::ArrayVec<u8, N>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <arrayvec::ArrayVec<u8, N> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }
}

#[cfg(feature = "bytes")]
mod bytes_impls {
    use super::{DrizzleError, FromDrizzleRow, Row, expect_blob, field_at, field_is_null};

    impl FromDrizzleRow<Row> for bytes::Bytes {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            expect_blob(field_at(row, offset)?).map(Self::copy_from_slice)
        }
    }

    impl FromDrizzleRow<Row> for Option<bytes::Bytes> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            expect_blob(field).map(|b| Some(bytes::Bytes::copy_from_slice(b)))
        }
    }

    impl FromDrizzleRow<Row> for bytes::BytesMut {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            expect_blob(field_at(row, offset)?).map(Self::from)
        }
    }

    impl FromDrizzleRow<Row> for Option<bytes::BytesMut> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            expect_blob(field).map(|b| Some(bytes::BytesMut::from(b)))
        }
    }
}

#[cfg(feature = "compact-str")]
mod compact_str_impls {
    use super::{DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null};

    impl FromDrizzleRow<Row> for compact_str::CompactString {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            Ok(Self::new(s))
        }
    }

    impl FromDrizzleRow<Row> for Option<compact_str::CompactString> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            Ok(Some(compact_str::CompactString::new(s)))
        }
    }
}

#[cfg(feature = "smallvec")]
mod smallvec_impls {
    use super::{DrizzleError, FromDrizzleRow, Row, expect_blob, field_at, field_is_null};

    // SmallVec<[u8; N]> — byte array backed. Stack-allocated up to N, spills
    // to heap beyond. We mirror the tokio-postgres impl by reading as Vec<u8>.
    impl<const N: usize> FromDrizzleRow<Row> for smallvec::SmallVec<[u8; N]> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let bytes = expect_blob(field_at(row, offset)?)?;
            Ok(Self::from_slice(bytes))
        }
    }

    impl<const N: usize> FromDrizzleRow<Row> for Option<smallvec::SmallVec<[u8; N]>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let bytes = expect_blob(field)?;
            Ok(Some(smallvec::SmallVec::from_slice(bytes)))
        }
    }
}

#[cfg(feature = "bit-vec")]
mod bit_vec_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
    };

    /// Parse a bit string of the form "010110..." into a [`bit_vec::BitVec`].
    /// The Data API returns BIT / BIT VARYING columns as `StringValues` in this
    /// ASCII form (one char per bit).
    fn parse_bits(s: &str) -> Result<bit_vec::BitVec, DrizzleError> {
        let mut out = bit_vec::BitVec::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '0' => out.push(false),
                '1' => out.push(true),
                other => {
                    return Err(DrizzleError::ConversionError(
                        format!("AWS Data API bit: unexpected char {other:?} in {s:?}").into(),
                    ));
                }
            }
        }
        Ok(out)
    }

    impl FromDrizzleRow<Row> for bit_vec::BitVec {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            parse_bits(s)
        }
    }

    impl FromDrizzleRow<Row> for Option<bit_vec::BitVec> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            parse_bits(expect_string(field)?).map(Some)
        }
    }
}

#[cfg(feature = "cidr")]
mod cidr_impls {
    use super::{
        DrizzleError, FromDrizzleRow, Row, expect_string, field_at, field_is_null, format,
    };
    use core::str::FromStr;

    impl FromDrizzleRow<Row> for cidr::IpInet {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            Self::from_str(s).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API inet: {e}").into())
            })
        }
    }

    impl FromDrizzleRow<Row> for Option<cidr::IpInet> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            cidr::IpInet::from_str(s).map(Some).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API inet: {e}").into())
            })
        }
    }

    impl FromDrizzleRow<Row> for cidr::IpCidr {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            Self::from_str(s).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API cidr: {e}").into())
            })
        }
    }

    impl FromDrizzleRow<Row> for Option<cidr::IpCidr> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            let s = expect_string(field)?;
            cidr::IpCidr::from_str(s).map(Some).map_err(|e| {
                DrizzleError::ConversionError(format!("AWS Data API cidr: {e}").into())
            })
        }
    }
}

#[cfg(feature = "geo-types")]
mod geo_types_impls {
    //! Geo-types leaf impls. The Data API returns `PostGIS` geometries as
    //! `StringValues` containing either the Postgres literal (e.g. `(x,y)` for
    //! POINT) or WKT. We parse the simple `(x,y)` / `[(x,y),...]` / `((x1,y1),(x2,y2))`
    //! forms emitted by our own `encode_field`, falling back to an error for
    //! anything else — users that need WKB / WKT can implement their own wrappers.
    use super::{
        DrizzleError, FromDrizzleRow, Row, Vec, expect_string, field_at, field_is_null, format,
    };
    use geo_types::{Coord, LineString, Point, Rect};

    fn parse_xy(s: &str) -> Result<(f64, f64), DrizzleError> {
        let trimmed = s.trim_matches(|c: char| c == '(' || c == ')');
        let mut parts = trimmed.split(',');
        let x = parts
            .next()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .ok_or_else(|| {
                DrizzleError::ConversionError(format!("AWS Data API point: bad x in {s:?}").into())
            })?;
        let y = parts
            .next()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .ok_or_else(|| {
                DrizzleError::ConversionError(format!("AWS Data API point: bad y in {s:?}").into())
            })?;
        Ok((x, y))
    }

    impl FromDrizzleRow<Row> for Point<f64> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            let (x, y) = parse_xy(s)?;
            Ok(Self::new(x, y))
        }
    }

    impl FromDrizzleRow<Row> for Option<Point<f64>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <Point<f64> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for LineString<f64> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            // Accept either `[(x,y),(x,y)]` or Postgres `(p1,p2,...)` style.
            let inner = s
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim_start_matches('(')
                .trim_end_matches(')');
            let mut coords = Vec::new();
            // Split on "),(" to get each point; manual scan keeps us zero-dep.
            let mut depth = 0i32;
            let mut start = 0usize;
            let bytes = inner.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                match b {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b',' if depth == 0 => {
                        let part = &inner[start..i];
                        if !part.trim().is_empty() {
                            let (x, y) = parse_xy(part)?;
                            coords.push(Coord { x, y });
                        }
                        start = i + 1;
                    }
                    _ => {}
                }
            }
            let tail = &inner[start..];
            if !tail.trim().is_empty() {
                let (x, y) = parse_xy(tail)?;
                coords.push(Coord { x, y });
            }
            Ok(Self::from(coords))
        }
    }

    impl FromDrizzleRow<Row> for Option<LineString<f64>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <LineString<f64> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }

    impl FromDrizzleRow<Row> for Rect<f64> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let s = expect_string(field_at(row, offset)?)?;
            // Expect "((x1,y1),(x2,y2))" from encode_field.
            let stripped = s.trim().trim_start_matches('(').trim_end_matches(')');
            let mid = stripped.find("),(").ok_or_else(|| {
                DrizzleError::ConversionError(format!("AWS Data API rect: bad format {s:?}").into())
            })?;
            let (a, b) = stripped.split_at(mid);
            let b = &b[3..]; // skip "),("
            let (x1, y1) = parse_xy(a)?;
            let (x2, y2) = parse_xy(b)?;
            Ok(Self::new(Coord { x: x1, y: y1 }, Coord { x: x2, y: y2 }))
        }
    }

    impl FromDrizzleRow<Row> for Option<Rect<f64>> {
        const COLUMN_COUNT: usize = 1;
        fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
            let field = field_at(row, offset)?;
            if field_is_null(field) {
                return Ok(None);
            }
            <Rect<f64> as FromDrizzleRow<Row>>::from_row_at(row, offset).map(Some)
        }
    }
}

// =============================================================================
// Composite Option<T> via NullProbeRow
// =============================================================================

impl<T> FromDrizzleRow<Row> for Option<T>
where
    T: NullProbeRow<Row>,
{
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;

    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError> {
        if T::is_null_at(row, offset)? {
            return Ok(None);
        }
        T::from_row_at(row, offset).map(Some)
    }
}

// =============================================================================
// NullProbeRow helper — proc macros emit this for select models.
// =============================================================================

/// Public helper used by proc-macro-generated `NullProbeRow` impls. A row is
/// "null at offset" if the field at that offset is `Field::IsNull(true)`.
///
/// # Errors
///
/// Returns [`DrizzleError::ConversionError`] when `offset` is out of bounds
/// for the row.
pub fn is_null_at(row: &Row, offset: usize) -> Result<bool, DrizzleError> {
    null_probe(row, offset)
}

// =============================================================================
// SqlParameter encoding — PostgresValue → SqlParameter
// =============================================================================

/// Encode a [`PostgresValue`] as an AWS Data API [`SqlParameter`] with the
/// correct [`Field`] variant and optional [`TypeHint`].
///
/// Mirrors upstream `drizzle-orm/src/aws-data-api/common/index.ts::toValueParam`.
pub fn encode_param(name: impl Into<String>, value: &PostgresValue<'_>) -> SqlParameter {
    let (field, hint) = encode_field(value);
    let mut builder = SqlParameter::builder().name(name).value(field);
    if let Some(h) = hint {
        builder = builder.type_hint(h);
    }
    // `.build()` returns plain SqlParameter in the rdsdata types module.
    builder.build()
}

#[cfg(feature = "chrono")]
fn encode_chrono_field(value: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    match value {
        PostgresValue::Date(d) => Some((Field::StringValue(d.to_string()), Some(TypeHint::Date))),
        PostgresValue::Time(t) => Some((Field::StringValue(t.to_string()), Some(TypeHint::Time))),
        PostgresValue::Timestamp(ts) => Some((
            Field::StringValue(ts.format("%Y-%m-%d %H:%M:%S%.f").to_string()),
            Some(TypeHint::Timestamp),
        )),
        PostgresValue::TimestampTz(ts) => Some((
            Field::StringValue(ts.to_rfc3339()),
            Some(TypeHint::Timestamp),
        )),
        PostgresValue::Interval(dur) => Some((
            Field::StringValue(format!("{} seconds", dur.num_seconds())),
            None,
        )),
        _ => None,
    }
}

#[cfg(not(feature = "chrono"))]
const fn encode_chrono_field(_: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    None
}

#[cfg(feature = "time")]
fn encode_time_field(value: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    match value {
        PostgresValue::TimeDate(d) => {
            Some((Field::StringValue(d.to_string()), Some(TypeHint::Date)))
        }
        PostgresValue::TimeTime(t) => {
            Some((Field::StringValue(t.to_string()), Some(TypeHint::Time)))
        }
        PostgresValue::TimeTimestamp(ts) => Some((
            Field::StringValue(ts.to_string()),
            Some(TypeHint::Timestamp),
        )),
        PostgresValue::TimeTimestampTz(ts) => Some((
            Field::StringValue(ts.to_string()),
            Some(TypeHint::Timestamp),
        )),
        PostgresValue::TimeInterval(dur) => Some((
            Field::StringValue(format!("{} seconds", dur.whole_seconds())),
            None,
        )),
        _ => None,
    }
}

#[cfg(not(feature = "time"))]
const fn encode_time_field(_: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    None
}

#[cfg(feature = "cidr")]
fn encode_cidr_field(value: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    match value {
        PostgresValue::Inet(net) => Some((Field::StringValue(net.to_string()), None)),
        PostgresValue::Cidr(net) => Some((Field::StringValue(net.to_string()), None)),
        PostgresValue::MacAddr(mac) => Some((
            Field::StringValue(format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            )),
            None,
        )),
        PostgresValue::MacAddr8(mac) => Some((
            Field::StringValue(format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[6], mac[7]
            )),
            None,
        )),
        _ => None,
    }
}

#[cfg(not(feature = "cidr"))]
const fn encode_cidr_field(_: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    None
}

#[cfg(feature = "geo-types")]
fn encode_geo_field(value: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    match value {
        PostgresValue::Point(p) => {
            Some((Field::StringValue(format!("({},{})", p.x(), p.y())), None))
        }
        PostgresValue::LineString(line) => {
            let coords: Vec<String> = line
                .coords()
                .map(|c| format!("({},{})", c.x, c.y))
                .collect();
            Some((Field::StringValue(format!("[{}]", coords.join(","))), None))
        }
        PostgresValue::Rect(r) => Some((
            Field::StringValue(format!(
                "(({},{}),({},{}))",
                r.min().x,
                r.min().y,
                r.max().x,
                r.max().y
            )),
            None,
        )),
        _ => None,
    }
}

#[cfg(not(feature = "geo-types"))]
const fn encode_geo_field(_: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    None
}

#[cfg(feature = "bit-vec")]
fn encode_bitvec_field(value: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    match value {
        PostgresValue::BitVec(bv) => {
            let s: String = bv.iter().map(|b| if b { '1' } else { '0' }).collect();
            Some((Field::StringValue(s), None))
        }
        _ => None,
    }
}

#[cfg(not(feature = "bit-vec"))]
const fn encode_bitvec_field(_: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    None
}

fn encode_core_field(value: &PostgresValue<'_>) -> Option<(Field, Option<TypeHint>)> {
    match value {
        PostgresValue::Null => Some((Field::IsNull(true), None)),
        PostgresValue::Smallint(v) => Some((Field::LongValue(i64::from(*v)), None)),
        PostgresValue::Integer(v) => Some((Field::LongValue(i64::from(*v)), None)),
        PostgresValue::Bigint(v) => Some((Field::LongValue(*v), None)),
        PostgresValue::Real(v) => Some((Field::DoubleValue(f64::from(*v)), None)),
        PostgresValue::DoublePrecision(v) => Some((Field::DoubleValue(*v), None)),
        PostgresValue::Boolean(v) => Some((Field::BooleanValue(*v), None)),
        PostgresValue::Text(s) => Some((Field::StringValue(s.to_string()), None)),
        PostgresValue::Bytea(b) => Some((Field::BlobValue(Blob::new(b.to_vec())), None)),
        #[cfg(feature = "rust-decimal")]
        PostgresValue::Numeric(d) => {
            Some((Field::StringValue(d.to_string()), Some(TypeHint::Decimal)))
        }
        #[cfg(feature = "uuid")]
        PostgresValue::Uuid(u) => Some((Field::StringValue(u.to_string()), Some(TypeHint::Uuid))),
        #[cfg(feature = "serde")]
        PostgresValue::Json(v) | PostgresValue::Jsonb(v) => {
            Some((Field::StringValue(v.to_string()), Some(TypeHint::Json)))
        }
        PostgresValue::Enum(e) => Some((Field::StringValue(e.variant_name().to_string()), None)),
        PostgresValue::Array(items) => Some((encode_array(items), None)),
        _ => None,
    }
}

/// Split out so nested [`PostgresValue::Array`] can recurse.
fn encode_field(value: &PostgresValue<'_>) -> (Field, Option<TypeHint>) {
    encode_core_field(value)
        .or_else(|| encode_chrono_field(value))
        .or_else(|| encode_time_field(value))
        .or_else(|| encode_cidr_field(value))
        .or_else(|| encode_geo_field(value))
        .or_else(|| encode_bitvec_field(value))
        .unwrap_or((Field::IsNull(true), None))
}

/// Collapse a `Vec<PostgresValue>` into `Field::ArrayValue(ArrayValue::...)`.
///
/// Uses the variant of the first element to decide the array shape. Mixed
/// arrays fall back to `StringValues` (values `Display`-formatted). Empty
/// arrays collapse to `StringValues(vec![])`.
fn encode_array(items: &[PostgresValue<'_>]) -> Field {
    use PostgresValue as V;

    if items.is_empty() {
        return Field::ArrayValue(ArrayValue::StringValues(Vec::new()));
    }

    // Detect homogeneous shape from the first element.
    let array = match &items[0] {
        V::Smallint(_) | V::Integer(_) | V::Bigint(_) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                let i = match v {
                    V::Smallint(i) => i64::from(*i),
                    V::Integer(i) => i64::from(*i),
                    V::Bigint(i) => *i,
                    _ => return fallback_string_array(items),
                };
                out.push(i);
            }
            ArrayValue::LongValues(out)
        }
        V::Real(_) | V::DoublePrecision(_) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                let f = match v {
                    V::Real(f) => f64::from(*f),
                    V::DoublePrecision(f) => *f,
                    _ => return fallback_string_array(items),
                };
                out.push(f);
            }
            ArrayValue::DoubleValues(out)
        }
        V::Boolean(_) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                match v {
                    V::Boolean(b) => out.push(*b),
                    _ => return fallback_string_array(items),
                }
            }
            ArrayValue::BooleanValues(out)
        }
        _ => return fallback_string_array(items),
    };

    Field::ArrayValue(array)
}

fn fallback_string_array(items: &[PostgresValue<'_>]) -> Field {
    let out: Vec<String> = items.iter().map(std::string::ToString::to_string).collect();
    Field::ArrayValue(ArrayValue::StringValues(out))
}
