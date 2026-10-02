//! `MySQL` types. The supported server baseline is MySQL 8.0.31.
//!
//! - [`types`]: zero-sized SQL type markers used for compile-time checks.
//! - [`MySQLType`]: column types as written in DDL, including signedness and
//!   inline `ENUM`/`SET` values.
//! - [`TypeCategory`]: how a Rust field type maps to a `MySQL` column.
//! - [`MySQLTypeCategory`]: families of SQL type declarations, used when parsing.
//! - [`ddl`]: schema objects (tables, columns, indexes, ...) for migrations.
//! - [`default`]: the canonical `DEFAULT` clause spelling shared by every
//!   schema producer.
//!
//! PostgreSQL-only concepts such as arrays, `JSONB` and named enum types have
//! no `MySQL` equivalent here.

pub mod ddl;
pub mod default;
mod sql_type;
mod type_category;

/// Zero-sized SQL type markers for the `MySQL` dialect.
///
/// Each marker stands for one SQL type at compile time, named after it
/// (`IntUnsigned` is `INT UNSIGNED`, `Any` is untyped SQL such as a raw `SQL`
/// fragment). Markers carry no length, precision or `ENUM` values; see
/// [`MySQLType`] for those. The traits in [`crate::sql`] say which markers
/// can be compared, assigned, or used in arithmetic.
pub mod types {
    macro_rules! mysql_markers {
        ($($name:ident),+ $(,)?) => {
            $(
                #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
                pub struct $name;
            )+
        };
    }

    mysql_markers!(
        TinyInt,
        TinyIntUnsigned,
        SmallInt,
        SmallIntUnsigned,
        MediumInt,
        MediumIntUnsigned,
        Int,
        IntUnsigned,
        BigInt,
        BigIntUnsigned,
        Float,
        Double,
        Decimal,
        Boolean,
        Char,
        Varchar,
        TinyText,
        Text,
        MediumText,
        LongText,
        Binary,
        Varbinary,
        TinyBlob,
        Blob,
        MediumBlob,
        LongBlob,
        Json,
        Date,
        Time,
        DateTime,
        Timestamp,
        Year,
        Enum,
        Set,
        Bit,
        Any,
    );

    /// MySQL accepts `INTEGER` as an alias for `INT`.
    pub type Integer = Int;
    /// MySQL accepts `INTEGER UNSIGNED` as an alias for `INT UNSIGNED`.
    pub type IntegerUnsigned = IntUnsigned;
    /// MySQL accepts `NUMERIC` as an alias for `DECIMAL`.
    pub type Numeric = Decimal;
}

pub use default::canonical_default;
pub use sql_type::MySQLType;
pub use type_category::{MySQLTypeCategory, TypeCategory};
