//! `MySQL` type definitions.
//!
//! The supported server baseline is MySQL 8.0.31. This module models MySQL's
//! native type families without treating PostgreSQL-only concepts such as
//! arrays, `JSONB`, or named enum types as portable.

pub mod ddl;
mod sql_type;
mod type_category;

/// Zero-sized SQL type markers for the MySQL dialect.
///
/// These markers are used by `drizzle-core` to distinguish MySQL's signed and
/// unsigned integer widths, text and binary families, and temporal types at
/// compile time. They carry no runtime type declaration details such as a
/// length, precision, scale, or inline enum values. Use [`MySQLType`] when
/// schema metadata needs those details.
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

pub use sql_type::MySQLType;
pub use type_category::{MySQLTypeCategory, TypeCategory};
