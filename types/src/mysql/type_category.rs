//! MySQL Rust and SQL type classification.

use super::MySQLType;

/// Categorizes Rust types for MySQL schema inference.
///
/// MySQL has separate signed and unsigned integer declarations, so this
/// category preserves Rust integer signedness instead of collapsing all
/// integral values into one marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TypeCategory {
    ArrayString,
    ArrayVec,
    String,
    Blob,
    ByteArray,
    CharArray,
    Uuid,
    Json,
    Enum,
    Set,
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F32,
    F64,
    Bool,
    NaiveDate,
    NaiveTime,
    NaiveDateTime,
    DateTimeTz,
    TimeDate,
    TimeTime,
    TimePrimitiveDateTime,
    TimeOffsetDateTime,
    Unknown,
}

impl TypeCategory {
    /// Classify a stringified Rust type.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn classify(type_str: &str) -> Self {
        let type_str = type_str.replace(' ', "");

        if type_str.starts_with("Option<") && type_str.ends_with('>') {
            return Self::classify(&type_str[7..type_str.len() - 1]);
        }

        if type_str.starts_with("[u8;")
            || (type_str.contains("[u8;") && !type_str.contains("SmallVec"))
        {
            return Self::ByteArray;
        }
        if type_str.starts_with("[char;") || type_str.contains("[char;") {
            return Self::CharArray;
        }

        if type_str.contains("ArrayString") || type_str.contains("CompactString") {
            return Self::ArrayString;
        }
        if (type_str.contains("ArrayVec") && type_str.contains("u8"))
            || type_str.contains("bytes::Bytes")
            || type_str.contains("bytes::BytesMut")
            || type_str == "Bytes"
            || type_str == "BytesMut"
            || (type_str.contains("SmallVec") && type_str.contains("u8"))
        {
            return Self::ArrayVec;
        }

        if type_str.contains("Uuid") {
            return Self::Uuid;
        }
        if type_str.contains("serde_json::Value") || type_str == "Value" {
            return Self::Json;
        }

        if type_str.contains("NaiveDate") && !type_str.contains("NaiveDateTime") {
            return Self::NaiveDate;
        }
        if type_str.contains("NaiveTime") {
            return Self::NaiveTime;
        }
        if type_str.contains("NaiveDateTime") {
            return Self::NaiveDateTime;
        }
        if type_str.contains("DateTime<") {
            return Self::DateTimeTz;
        }

        if type_str.contains("time::Date") || type_str == "Date" {
            return Self::TimeDate;
        }
        if type_str.contains("time::Time") {
            return Self::TimeTime;
        }
        if type_str.contains("PrimitiveDateTime") {
            return Self::TimePrimitiveDateTime;
        }
        if type_str.contains("OffsetDateTime") {
            return Self::TimeOffsetDateTime;
        }

        if type_str.contains("String") {
            return Self::String;
        }
        if type_str.contains("Vec<u8>") {
            return Self::Blob;
        }

        match type_str.as_str() {
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "isize" => Self::Isize,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "usize" => Self::Usize,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            _ => Self::Unknown,
        }
    }

    /// Return the MySQL declaration inferred for this Rust category.
    #[must_use]
    pub fn sql_type(self) -> Option<MySQLType> {
        match self {
            Self::I8 => Some(MySQLType::Tinyint),
            Self::I16 => Some(MySQLType::Smallint),
            Self::I32 => Some(MySQLType::Int),
            Self::I64 | Self::Isize => Some(MySQLType::Bigint),
            Self::U8 => Some(MySQLType::TinyintUnsigned),
            Self::U16 => Some(MySQLType::SmallintUnsigned),
            Self::U32 => Some(MySQLType::IntUnsigned),
            Self::U64 | Self::Usize => Some(MySQLType::BigintUnsigned),
            Self::F32 => Some(MySQLType::Float),
            Self::F64 => Some(MySQLType::Double),
            Self::Bool => Some(MySQLType::Boolean),
            Self::String => Some(MySQLType::Text),
            Self::ArrayString => Some(MySQLType::Varchar),
            Self::CharArray => Some(MySQLType::Char),
            Self::Blob | Self::ByteArray | Self::ArrayVec => Some(MySQLType::Blob),
            #[cfg(feature = "uuid")]
            Self::Uuid => Some(MySQLType::Binary),
            #[cfg(not(feature = "uuid"))]
            Self::Uuid => None,
            #[cfg(feature = "serde")]
            Self::Json => Some(MySQLType::Json),
            #[cfg(not(feature = "serde"))]
            Self::Json => None,
            Self::NaiveDate | Self::TimeDate => Some(MySQLType::Date),
            Self::NaiveTime | Self::TimeTime => Some(MySQLType::Time),
            Self::NaiveDateTime | Self::TimePrimitiveDateTime => Some(MySQLType::Datetime),
            Self::DateTimeTz | Self::TimeOffsetDateTime => Some(MySQLType::Timestamp),
            Self::Enum | Self::Set | Self::Unknown => None,
        }
    }
}

/// MySQL SQL type classification for parsing and introspection.
///
/// This retains the signedness of integer declarations. Length, precision,
/// scale, and inline `ENUM`/`SET` values remain part of the declaration
/// metadata rather than this category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum MySQLTypeCategory {
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
    Decimal,
    Float,
    Double,
    Boolean,
    Bit,
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
    Custom,
}

impl MySQLTypeCategory {
    fn starts_with_type_name(sql_type: &str, type_name: &str) -> bool {
        if sql_type.len() < type_name.len()
            || !sql_type[..type_name.len()].eq_ignore_ascii_case(type_name)
        {
            return false;
        }

        sql_type[type_name.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    }

    fn is_unsigned(sql_type: &str) -> bool {
        sql_type
            .split(|ch: char| !ch.is_ascii_alphabetic())
            .any(|word| {
                word.eq_ignore_ascii_case("unsigned") || word.eq_ignore_ascii_case("zerofill")
            })
    }

    fn integer_category(
        sql_type: &str,
        type_name: &str,
        signed: Self,
        unsigned: Self,
    ) -> Option<Self> {
        Self::starts_with_type_name(sql_type, type_name).then_some(if Self::is_unsigned(sql_type) {
            unsigned
        } else {
            signed
        })
    }

    /// Classify a MySQL type declaration.
    ///
    /// `ZEROFILL` is treated as unsigned because MySQL implicitly adds the
    /// `UNSIGNED` attribute to a `ZEROFILL` numeric declaration.
    #[must_use]
    pub fn classify(sql_type: &str) -> Self {
        let normalized = sql_type.trim();

        Self::integer_category(normalized, "tinyint", Self::TinyInt, Self::TinyIntUnsigned)
            .or_else(|| {
                Self::integer_category(
                    normalized,
                    "smallint",
                    Self::SmallInt,
                    Self::SmallIntUnsigned,
                )
            })
            .or_else(|| {
                Self::integer_category(
                    normalized,
                    "mediumint",
                    Self::MediumInt,
                    Self::MediumIntUnsigned,
                )
            })
            .or_else(|| {
                Self::integer_category(normalized, "bigint", Self::BigInt, Self::BigIntUnsigned)
            })
            .or_else(|| Self::integer_category(normalized, "integer", Self::Int, Self::IntUnsigned))
            .or_else(|| Self::integer_category(normalized, "int", Self::Int, Self::IntUnsigned))
            .unwrap_or_else(|| Self::non_integer_category(normalized))
    }

    fn non_integer_category(sql_type: &str) -> Self {
        if Self::starts_with_type_name(sql_type, "decimal")
            || Self::starts_with_type_name(sql_type, "numeric")
            || Self::starts_with_type_name(sql_type, "dec")
            || Self::starts_with_type_name(sql_type, "fixed")
        {
            Self::Decimal
        } else if Self::starts_with_type_name(sql_type, "float") {
            Self::Float
        } else if Self::starts_with_type_name(sql_type, "double")
            || Self::starts_with_type_name(sql_type, "real")
        {
            Self::Double
        } else if Self::starts_with_type_name(sql_type, "boolean")
            || Self::starts_with_type_name(sql_type, "bool")
        {
            Self::Boolean
        } else if Self::starts_with_type_name(sql_type, "bit") {
            Self::Bit
        } else if Self::starts_with_type_name(sql_type, "varchar")
            || Self::starts_with_type_name(sql_type, "character varying")
        {
            Self::Varchar
        } else if Self::starts_with_type_name(sql_type, "char")
            || Self::starts_with_type_name(sql_type, "character")
        {
            Self::Char
        } else if Self::starts_with_type_name(sql_type, "tinytext") {
            Self::TinyText
        } else if Self::starts_with_type_name(sql_type, "mediumtext") {
            Self::MediumText
        } else if Self::starts_with_type_name(sql_type, "longtext") {
            Self::LongText
        } else if Self::starts_with_type_name(sql_type, "text") {
            Self::Text
        } else if Self::starts_with_type_name(sql_type, "varbinary") {
            Self::Varbinary
        } else if Self::starts_with_type_name(sql_type, "binary") {
            Self::Binary
        } else if Self::starts_with_type_name(sql_type, "tinyblob") {
            Self::TinyBlob
        } else if Self::starts_with_type_name(sql_type, "mediumblob") {
            Self::MediumBlob
        } else if Self::starts_with_type_name(sql_type, "longblob") {
            Self::LongBlob
        } else if Self::starts_with_type_name(sql_type, "blob") {
            Self::Blob
        } else if Self::starts_with_type_name(sql_type, "json") {
            Self::Json
        } else if Self::starts_with_type_name(sql_type, "datetime") {
            Self::DateTime
        } else if Self::starts_with_type_name(sql_type, "timestamp") {
            Self::Timestamp
        } else if Self::starts_with_type_name(sql_type, "date") {
            Self::Date
        } else if Self::starts_with_type_name(sql_type, "time") {
            Self::Time
        } else if Self::starts_with_type_name(sql_type, "year") {
            Self::Year
        } else if Self::starts_with_type_name(sql_type, "enum") {
            Self::Enum
        } else if Self::starts_with_type_name(sql_type, "set") {
            Self::Set
        } else {
            Self::Custom
        }
    }

    /// Return the conventional Drizzle column-builder name for this category.
    #[must_use]
    pub const fn drizzle_import(self) -> &'static str {
        match self {
            Self::TinyInt | Self::TinyIntUnsigned => "tinyint",
            Self::SmallInt | Self::SmallIntUnsigned => "smallint",
            Self::MediumInt | Self::MediumIntUnsigned => "mediumint",
            Self::Int | Self::IntUnsigned => "int",
            Self::BigInt | Self::BigIntUnsigned => "bigint",
            Self::Decimal => "decimal",
            Self::Float => "float",
            Self::Double => "double",
            Self::Boolean => "boolean",
            Self::Bit => "bit",
            Self::Char => "char",
            Self::Varchar => "varchar",
            Self::TinyText => "tinytext",
            Self::Text => "text",
            Self::MediumText => "mediumtext",
            Self::LongText => "longtext",
            Self::Binary => "binary",
            Self::Varbinary => "varbinary",
            Self::TinyBlob => "tinyblob",
            Self::Blob => "blob",
            Self::MediumBlob => "mediumblob",
            Self::LongBlob => "longblob",
            Self::Json => "json",
            Self::Date => "date",
            Self::Time => "time",
            Self::DateTime => "datetime",
            Self::Timestamp => "timestamp",
            Self::Year => "year",
            Self::Enum => "mysqlEnum",
            Self::Set => "set",
            Self::Custom => "customType",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_type_categories_keep_signedness() {
        assert_eq!(TypeCategory::classify("i8"), TypeCategory::I8);
        assert_eq!(TypeCategory::classify("u64"), TypeCategory::U64);
        assert_eq!(
            TypeCategory::classify("Option<u32>").sql_type(),
            Some(MySQLType::IntUnsigned)
        );
        assert_eq!(
            TypeCategory::classify("chrono::NaiveDateTime").sql_type(),
            Some(MySQLType::Datetime)
        );
    }

    #[test]
    fn sql_type_categories_parse_mysql_extensions() {
        assert_eq!(
            MySQLTypeCategory::classify("MEDIUMINT(8) UNSIGNED"),
            MySQLTypeCategory::MediumIntUnsigned
        );
        assert_eq!(
            MySQLTypeCategory::classify("INT ZEROFILL"),
            MySQLTypeCategory::IntUnsigned
        );
        assert_eq!(
            MySQLTypeCategory::classify("DECIMAL(20, 8)"),
            MySQLTypeCategory::Decimal
        );
        assert_eq!(
            MySQLTypeCategory::classify("ENUM('draft', 'published')"),
            MySQLTypeCategory::Enum
        );
        assert_eq!(
            MySQLTypeCategory::classify("DATETIME(6)"),
            MySQLTypeCategory::DateTime
        );
    }

    #[test]
    fn type_names_do_not_match_longer_identifiers() {
        assert_eq!(
            MySQLTypeCategory::classify("introspection_type"),
            MySQLTypeCategory::Custom
        );
    }
}
