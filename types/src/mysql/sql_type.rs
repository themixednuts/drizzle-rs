//! MySQL column type declarations.
//!
//! [`MySQLType`] retains the MySQL information that cannot be reconstructed
//! from a Rust marker, notably integer signedness and inline `ENUM`/`SET`
//! values. Length, precision, scale, character set, collation, and column
//! constraints belong to schema DDL metadata rather than this type enum.

use crate::alloc_prelude::{Cow, Vec};

/// A MySQL column type declaration.
///
/// This models MySQL 8.0.31's common native scalar, character, binary, JSON,
/// temporal, and inline collection types. `ENUM` and `SET` values are kept as
/// data because they are part of the column declaration, not named schema
/// objects as they are in PostgreSQL.
///
/// [`Self::sql`] returns a type keyword. DDL renderers must append
/// [`Self::inline_values`] for `ENUM` and `SET`, because correct literal
/// escaping belongs to the DDL renderer's SQL-mode policy.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MySQLType {
    /// `TINYINT`, a signed 1-byte integer.
    Tinyint,
    /// `TINYINT UNSIGNED`, an unsigned 1-byte integer.
    TinyintUnsigned,
    /// `SMALLINT`, a signed 2-byte integer.
    Smallint,
    /// `SMALLINT UNSIGNED`, an unsigned 2-byte integer.
    SmallintUnsigned,
    /// `MEDIUMINT`, a signed 3-byte integer.
    Mediumint,
    /// `MEDIUMINT UNSIGNED`, an unsigned 3-byte integer.
    MediumintUnsigned,
    /// `INT`, a signed 4-byte integer.
    Int,
    /// `INT UNSIGNED`, an unsigned 4-byte integer.
    IntUnsigned,
    /// `BIGINT`, a signed 8-byte integer.
    Bigint,
    /// `BIGINT UNSIGNED`, an unsigned 8-byte integer.
    BigintUnsigned,
    /// `DECIMAL`, an exact fixed-point number.
    Decimal,
    /// `DECIMAL UNSIGNED`, a non-negative exact fixed-point number.
    DecimalUnsigned,
    /// `FLOAT`, a single-precision approximate number.
    Float,
    /// `FLOAT UNSIGNED`, a non-negative single-precision approximate number.
    FloatUnsigned,
    /// `DOUBLE`, a double-precision approximate number.
    Double,
    /// `DOUBLE UNSIGNED`, a non-negative double-precision approximate number.
    DoubleUnsigned,
    /// `REAL`, MySQL's SQL-mode-sensitive approximate number spelling.
    Real,
    /// `REAL UNSIGNED`, the non-negative form of MySQL's `REAL` spelling.
    RealUnsigned,
    /// `BOOLEAN`, an alias for `TINYINT(1)` in MySQL.
    Boolean,
    /// `BIT`, a bit-value type.
    Bit,
    /// `CHAR`, a fixed-length character string.
    Char,
    /// `VARCHAR`, a variable-length character string.
    Varchar,
    /// `TINYTEXT`.
    Tinytext,
    /// `TEXT`.
    #[default]
    Text,
    /// `MEDIUMTEXT`.
    Mediumtext,
    /// `LONGTEXT`.
    Longtext,
    /// `BINARY`, a fixed-length binary string.
    Binary,
    /// `VARBINARY`, a variable-length binary string.
    Varbinary,
    /// `TINYBLOB`.
    Tinyblob,
    /// `BLOB`.
    Blob,
    /// `MEDIUMBLOB`.
    Mediumblob,
    /// `LONGBLOB`.
    Longblob,
    /// MySQL's native binary JSON type.
    Json,
    /// `DATE`.
    Date,
    /// `TIME`.
    Time,
    /// `DATETIME`, which does not apply time-zone conversion.
    Datetime,
    /// `TIMESTAMP`, which MySQL converts through the session time zone.
    Timestamp,
    /// `YEAR`.
    Year,
    /// A MySQL inline `ENUM` declaration and its ordered allowed values.
    Enum(Vec<Cow<'static, str>>),
    /// A MySQL inline `SET` declaration and its allowed values.
    Set(Vec<Cow<'static, str>>),
}

impl MySQLType {
    /// Parse a macro attribute name as a non-parameterized MySQL type.
    ///
    /// `ENUM` and `SET` require inline values, so construct them with
    /// [`Self::enum_values`] or [`Self::set_values`] instead.
    #[must_use]
    pub const fn parse_attribute(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("tinyint") {
            Some(Self::Tinyint)
        } else if name.eq_ignore_ascii_case("tinyint_unsigned") {
            Some(Self::TinyintUnsigned)
        } else if name.eq_ignore_ascii_case("smallint") {
            Some(Self::Smallint)
        } else if name.eq_ignore_ascii_case("smallint_unsigned") {
            Some(Self::SmallintUnsigned)
        } else if name.eq_ignore_ascii_case("mediumint") {
            Some(Self::Mediumint)
        } else if name.eq_ignore_ascii_case("mediumint_unsigned") {
            Some(Self::MediumintUnsigned)
        } else if name.eq_ignore_ascii_case("int") || name.eq_ignore_ascii_case("integer") {
            Some(Self::Int)
        } else if name.eq_ignore_ascii_case("int_unsigned")
            || name.eq_ignore_ascii_case("integer_unsigned")
        {
            Some(Self::IntUnsigned)
        } else if name.eq_ignore_ascii_case("bigint") {
            Some(Self::Bigint)
        } else if name.eq_ignore_ascii_case("bigint_unsigned") {
            Some(Self::BigintUnsigned)
        } else if name.eq_ignore_ascii_case("decimal")
            || name.eq_ignore_ascii_case("numeric")
            || name.eq_ignore_ascii_case("dec")
            || name.eq_ignore_ascii_case("fixed")
        {
            Some(Self::Decimal)
        } else if name.eq_ignore_ascii_case("decimal_unsigned")
            || name.eq_ignore_ascii_case("numeric_unsigned")
            || name.eq_ignore_ascii_case("dec_unsigned")
            || name.eq_ignore_ascii_case("fixed_unsigned")
        {
            Some(Self::DecimalUnsigned)
        } else if name.eq_ignore_ascii_case("float") {
            Some(Self::Float)
        } else if name.eq_ignore_ascii_case("float_unsigned") {
            Some(Self::FloatUnsigned)
        } else if name.eq_ignore_ascii_case("double")
            || name.eq_ignore_ascii_case("double_precision")
        {
            Some(Self::Double)
        } else if name.eq_ignore_ascii_case("double_unsigned")
            || name.eq_ignore_ascii_case("double_precision_unsigned")
        {
            Some(Self::DoubleUnsigned)
        } else if name.eq_ignore_ascii_case("real") {
            Some(Self::Real)
        } else if name.eq_ignore_ascii_case("real_unsigned") {
            Some(Self::RealUnsigned)
        } else if name.eq_ignore_ascii_case("boolean") || name.eq_ignore_ascii_case("bool") {
            Some(Self::Boolean)
        } else if name.eq_ignore_ascii_case("bit") {
            Some(Self::Bit)
        } else if name.eq_ignore_ascii_case("char") || name.eq_ignore_ascii_case("character") {
            Some(Self::Char)
        } else if name.eq_ignore_ascii_case("varchar")
            || name.eq_ignore_ascii_case("character_varying")
        {
            Some(Self::Varchar)
        } else if name.eq_ignore_ascii_case("tinytext") {
            Some(Self::Tinytext)
        } else if name.eq_ignore_ascii_case("text") {
            Some(Self::Text)
        } else if name.eq_ignore_ascii_case("mediumtext") {
            Some(Self::Mediumtext)
        } else if name.eq_ignore_ascii_case("longtext") {
            Some(Self::Longtext)
        } else if name.eq_ignore_ascii_case("binary") {
            Some(Self::Binary)
        } else if name.eq_ignore_ascii_case("varbinary") {
            Some(Self::Varbinary)
        } else if name.eq_ignore_ascii_case("tinyblob") {
            Some(Self::Tinyblob)
        } else if name.eq_ignore_ascii_case("blob") {
            Some(Self::Blob)
        } else if name.eq_ignore_ascii_case("mediumblob") {
            Some(Self::Mediumblob)
        } else if name.eq_ignore_ascii_case("longblob") {
            Some(Self::Longblob)
        } else if name.eq_ignore_ascii_case("json") {
            Some(Self::Json)
        } else if name.eq_ignore_ascii_case("date") {
            Some(Self::Date)
        } else if name.eq_ignore_ascii_case("time") {
            Some(Self::Time)
        } else if name.eq_ignore_ascii_case("datetime") {
            Some(Self::Datetime)
        } else if name.eq_ignore_ascii_case("timestamp") {
            Some(Self::Timestamp)
        } else if name.eq_ignore_ascii_case("year") {
            Some(Self::Year)
        } else {
            None
        }
    }

    /// Build a MySQL inline `ENUM` declaration from its allowed values.
    #[must_use]
    pub fn enum_values<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'static, str>>,
    {
        Self::Enum(values.into_iter().map(Into::into).collect())
    }

    /// Build a MySQL inline `SET` declaration from its allowed values.
    #[must_use]
    pub fn set_values<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'static, str>>,
    {
        Self::Set(values.into_iter().map(Into::into).collect())
    }

    /// Return the inline values for an `ENUM` or `SET` declaration.
    #[must_use]
    pub fn inline_values(&self) -> Option<&[Cow<'static, str>]> {
        match self {
            Self::Enum(values) | Self::Set(values) => Some(values),
            _ => None,
        }
    }

    /// Return this declaration's SQL type keyword.
    ///
    /// For `ENUM` and `SET`, use [`Self::inline_values`] to render their
    /// declaration values in a DDL renderer.
    #[must_use]
    pub const fn sql(&self) -> &'static str {
        match self {
            Self::Tinyint => "TINYINT",
            Self::TinyintUnsigned => "TINYINT UNSIGNED",
            Self::Smallint => "SMALLINT",
            Self::SmallintUnsigned => "SMALLINT UNSIGNED",
            Self::Mediumint => "MEDIUMINT",
            Self::MediumintUnsigned => "MEDIUMINT UNSIGNED",
            Self::Int => "INT",
            Self::IntUnsigned => "INT UNSIGNED",
            Self::Bigint => "BIGINT",
            Self::BigintUnsigned => "BIGINT UNSIGNED",
            Self::Decimal => "DECIMAL",
            Self::DecimalUnsigned => "DECIMAL UNSIGNED",
            Self::Float => "FLOAT",
            Self::FloatUnsigned => "FLOAT UNSIGNED",
            Self::Double => "DOUBLE",
            Self::DoubleUnsigned => "DOUBLE UNSIGNED",
            Self::Real => "REAL",
            Self::RealUnsigned => "REAL UNSIGNED",
            Self::Boolean => "BOOLEAN",
            Self::Bit => "BIT",
            Self::Char => "CHAR",
            Self::Varchar => "VARCHAR",
            Self::Tinytext => "TINYTEXT",
            Self::Text => "TEXT",
            Self::Mediumtext => "MEDIUMTEXT",
            Self::Longtext => "LONGTEXT",
            Self::Binary => "BINARY",
            Self::Varbinary => "VARBINARY",
            Self::Tinyblob => "TINYBLOB",
            Self::Blob => "BLOB",
            Self::Mediumblob => "MEDIUMBLOB",
            Self::Longblob => "LONGBLOB",
            Self::Json => "JSON",
            Self::Date => "DATE",
            Self::Time => "TIME",
            Self::Datetime => "DATETIME",
            Self::Timestamp => "TIMESTAMP",
            Self::Year => "YEAR",
            Self::Enum(_) => "ENUM",
            Self::Set(_) => "SET",
        }
    }

    /// Return whether this declaration carries MySQL's `UNSIGNED` modifier.
    #[must_use]
    pub const fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Self::TinyintUnsigned
                | Self::SmallintUnsigned
                | Self::MediumintUnsigned
                | Self::IntUnsigned
                | Self::BigintUnsigned
                | Self::DecimalUnsigned
                | Self::FloatUnsigned
                | Self::DoubleUnsigned
                | Self::RealUnsigned
        )
    }

    /// Validate optional numeric, length, width, or temporal arguments.
    #[must_use]
    #[doc(hidden)]
    pub fn validate_args(&self, args: &[u16]) -> Option<&'static str> {
        match self {
            Self::Tinyint
            | Self::TinyintUnsigned
            | Self::Smallint
            | Self::SmallintUnsigned
            | Self::Mediumint
            | Self::MediumintUnsigned
            | Self::Int
            | Self::IntUnsigned
            | Self::Bigint
            | Self::BigintUnsigned
            | Self::Year
                if args.len() > 1 =>
            {
                Some("integer and YEAR types accept at most one width argument")
            }
            Self::Varchar | Self::Varbinary if args.len() != 1 => {
                Some("VARCHAR and VARBINARY require exactly one length argument")
            }
            Self::Char | Self::Binary if args.len() > 1 => {
                Some("CHAR and BINARY accept at most one length argument")
            }
            Self::Bit if args.len() > 1 => Some("BIT accepts at most one width argument"),
            Self::Decimal
            | Self::DecimalUnsigned
            | Self::Float
            | Self::FloatUnsigned
            | Self::Double
            | Self::DoubleUnsigned
            | Self::Real
            | Self::RealUnsigned
                if args.len() > 2 =>
            {
                Some("numeric types accept precision and optional scale")
            }
            Self::Double | Self::DoubleUnsigned | Self::Real | Self::RealUnsigned
                if args.len() == 1 =>
            {
                Some("DOUBLE and REAL require both precision and scale")
            }
            Self::Time | Self::Datetime | Self::Timestamp if args.len() > 1 => {
                Some("temporal types accept at most one fractional-seconds precision")
            }
            Self::Tinytext
            | Self::Text
            | Self::Mediumtext
            | Self::Longtext
            | Self::Tinyblob
            | Self::Blob
            | Self::Mediumblob
            | Self::Longblob
            | Self::Json
            | Self::Date
            | Self::Boolean
                if !args.is_empty() =>
            {
                Some("this MySQL type does not accept arguments")
            }
            Self::Bit if args.first().is_some_and(|value| !(1..=64).contains(value)) => {
                Some("BIT width must be between 1 and 64")
            }
            Self::Char | Self::Binary if args.first().is_some_and(|value| *value > 255) => {
                Some("CHAR/BINARY length must not exceed 255")
            }
            Self::Decimal | Self::DecimalUnsigned
                if args.first().is_some_and(|value| !(1..=65).contains(value)) =>
            {
                Some("DECIMAL precision must be between 1 and 65")
            }
            Self::Decimal | Self::DecimalUnsigned
                if args.get(1).is_some_and(|scale| {
                    *scale > 30 || args.first().is_some_and(|precision| scale > precision)
                }) =>
            {
                Some("DECIMAL scale must not exceed 30 or its precision")
            }
            Self::Float | Self::FloatUnsigned if args.len() == 1 && args[0] > 24 => {
                Some("FLOAT binary precision must not exceed 24; use DOUBLE for double precision")
            }
            Self::Float | Self::FloatUnsigned if args.len() == 2 && args[0] > 255 => {
                Some("FLOAT display width must not exceed 255")
            }
            Self::Double | Self::DoubleUnsigned | Self::Real | Self::RealUnsigned
                if args.first().is_some_and(|precision| *precision > 255) =>
            {
                Some("FLOAT, DOUBLE, and REAL precision must not exceed 255")
            }
            Self::Float
            | Self::FloatUnsigned
            | Self::Double
            | Self::DoubleUnsigned
            | Self::Real
            | Self::RealUnsigned
                if args.get(1).is_some_and(|scale| {
                    *scale > 30 || args.first().is_some_and(|precision| scale > precision)
                }) =>
            {
                Some("FLOAT, DOUBLE, and REAL scale must not exceed 30 or its precision")
            }
            Self::Time | Self::Datetime | Self::Timestamp
                if args.first().is_some_and(|value| *value > 6) =>
            {
                Some("fractional-seconds precision must be between 0 and 6")
            }
            _ => None,
        }
    }

    /// Return whether this is an integer declaration eligible for
    /// `AUTO_INCREMENT`.
    #[must_use]
    pub const fn supports_auto_increment(&self) -> bool {
        matches!(
            self,
            Self::Tinyint
                | Self::TinyintUnsigned
                | Self::Smallint
                | Self::SmallintUnsigned
                | Self::Mediumint
                | Self::MediumintUnsigned
                | Self::Int
                | Self::IntUnsigned
                | Self::Bigint
                | Self::BigintUnsigned
        )
    }

    /// Return whether this declaration can be the declared type of a MySQL
    /// generated column.
    ///
    /// MySQL's generated-column restrictions concern the expression and its
    /// other column attributes rather than one of these native type families.
    #[must_use]
    pub const fn supports_generated_columns(&self) -> bool {
        true
    }

    /// Return whether a column attribute is compatible with this type alone.
    ///
    /// Constraints involving multiple attributes, such as the prohibition on
    /// an `AUTO_INCREMENT` generated column, belong to a column DDL model.
    #[must_use]
    pub fn is_valid_flag(&self, flag: &str) -> bool {
        match flag {
            "primary" | "primary_key" | "unique" | "not_null" | "check" | "references"
            | "default" | "default_fn" => true,
            "autoincrement" | "auto_increment" => self.supports_auto_increment(),
            "generated" | "generated_stored" | "generated_virtual" => {
                self.supports_generated_columns()
            }
            "json" => matches!(self, Self::Json),
            "enum" => matches!(self, Self::Enum(_)),
            "set" => matches!(self, Self::Set(_)),
            _ => false,
        }
    }
}

impl core::fmt::Display for MySQLType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.sql())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_names_preserve_mysql_integer_signedness() {
        assert_eq!(
            MySQLType::parse_attribute("mediumint_unsigned"),
            Some(MySQLType::MediumintUnsigned)
        );
        assert_eq!(MySQLType::parse_attribute("integer"), Some(MySQLType::Int));
        assert_eq!(
            MySQLType::parse_attribute("bigint_unsigned"),
            Some(MySQLType::BigintUnsigned)
        );
        assert_eq!(MySQLType::parse_attribute("enum"), None);
        assert_eq!(
            MySQLType::parse_attribute("decimal_unsigned"),
            Some(MySQLType::DecimalUnsigned)
        );
        assert_eq!(
            MySQLType::parse_attribute("float_unsigned"),
            Some(MySQLType::FloatUnsigned)
        );
        assert_eq!(MySQLType::parse_attribute("real"), Some(MySQLType::Real));
        assert_eq!(
            MySQLType::parse_attribute("real_unsigned"),
            Some(MySQLType::RealUnsigned)
        );
    }

    #[test]
    fn inline_enum_and_set_values_remain_metadata() {
        let state = MySQLType::enum_values(["draft", "published"]);
        let tags = MySQLType::set_values(["rust", "sql"]);

        assert_eq!(state.sql(), "ENUM");
        assert_eq!(
            state.inline_values(),
            Some(&[Cow::Borrowed("draft"), Cow::Borrowed("published")][..])
        );
        assert_eq!(tags.sql(), "SET");
        assert_eq!(
            tags.inline_values(),
            Some(&[Cow::Borrowed("rust"), Cow::Borrowed("sql")][..])
        );
    }

    #[test]
    fn mysql_column_capabilities_are_type_specific() {
        assert!(MySQLType::BigintUnsigned.supports_auto_increment());
        assert!(!MySQLType::Decimal.supports_auto_increment());
        assert!(MySQLType::DecimalUnsigned.is_unsigned());
        assert!(MySQLType::FloatUnsigned.is_unsigned());
        assert!(MySQLType::DoubleUnsigned.is_unsigned());
        assert!(MySQLType::RealUnsigned.is_unsigned());
        assert!(!MySQLType::Boolean.supports_auto_increment());
        assert!(MySQLType::Json.supports_generated_columns());
        assert!(MySQLType::Int.is_valid_flag("auto_increment"));
        assert!(!MySQLType::Text.is_valid_flag("auto_increment"));
        assert!(MySQLType::enum_values(["one"]).is_valid_flag("enum"));
    }
}
