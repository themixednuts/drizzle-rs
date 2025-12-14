//! PostgreSQL column type definitions
//!
//! Defines the core PostgreSQL data types for schema definition.

/// Enum representing supported PostgreSQL column types.
///
/// These correspond to PostgreSQL data types.
/// See: <https://www.postgresql.org/docs/current/datatype.html>
///
/// # Examples
///
/// ```
/// use drizzle_types::postgres::PostgreSQLType;
///
/// let int_type = PostgreSQLType::Integer;
/// assert_eq!(int_type.to_sql_type(), "INTEGER");
///
/// let serial = PostgreSQLType::Serial;
/// assert!(serial.is_serial());
/// ```
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PostgreSQLType {
    /// PostgreSQL INTEGER type - 32-bit signed integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
    Integer,

    /// PostgreSQL BIGINT type - 64-bit signed integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
    Bigint,

    /// PostgreSQL SMALLINT type - 16-bit signed integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
    Smallint,

    /// PostgreSQL SERIAL type - auto-incrementing 32-bit integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
    Serial,

    /// PostgreSQL BIGSERIAL type - auto-incrementing 64-bit integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
    Bigserial,

    /// PostgreSQL TEXT type - variable-length character string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-character.html>
    #[default]
    Text,

    /// PostgreSQL VARCHAR type - variable-length character string with limit
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-character.html>
    Varchar,

    /// PostgreSQL CHAR type - fixed-length character string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-character.html>
    Char,

    /// PostgreSQL REAL type - single precision floating-point number
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
    Real,

    /// PostgreSQL DOUBLE PRECISION type - double precision floating-point number
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
    DoublePrecision,

    /// PostgreSQL NUMERIC type - exact numeric with selectable precision
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-NUMERIC-DECIMAL>
    Numeric,

    /// PostgreSQL BOOLEAN type - true/false
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-boolean.html>
    Boolean,

    /// PostgreSQL BYTEA type - binary data
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-binary.html>
    Bytea,

    /// PostgreSQL UUID type - universally unique identifier
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-uuid.html>
    #[cfg(feature = "uuid")]
    Uuid,

    /// PostgreSQL JSON type - JSON data
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-json.html>
    #[cfg(feature = "serde")]
    Json,

    /// PostgreSQL JSONB type - binary JSON data
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-json.html>
    #[cfg(feature = "serde")]
    Jsonb,

    /// PostgreSQL TIMESTAMP type - date and time
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Timestamp,

    /// PostgreSQL TIMESTAMPTZ type - date and time with time zone
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Timestamptz,

    /// PostgreSQL DATE type - calendar date
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Date,

    /// PostgreSQL TIME type - time of day
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Time,

    /// PostgreSQL TIMETZ type - time of day with time zone
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Timetz,

    /// PostgreSQL INTERVAL type - time interval
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    #[cfg(feature = "chrono")]
    Interval,

    /// PostgreSQL INET type - IPv4 or IPv6 host address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    Inet,

    /// PostgreSQL CIDR type - IPv4 or IPv6 network address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    Cidr,

    /// PostgreSQL MACADDR type - MAC address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    MacAddr,

    /// PostgreSQL MACADDR8 type - EUI-64 MAC address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    MacAddr8,

    /// PostgreSQL POINT type - geometric point
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Point,

    /// PostgreSQL LINE type - geometric line (infinite)
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Line,

    /// PostgreSQL LSEG type - geometric line segment
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Lseg,

    /// PostgreSQL BOX type - geometric box
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Box,

    /// PostgreSQL PATH type - geometric path
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Path,

    /// PostgreSQL POLYGON type - geometric polygon
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Polygon,

    /// PostgreSQL CIRCLE type - geometric circle
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Circle,

    /// PostgreSQL BIT type - fixed-length bit string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-bit.html>
    #[cfg(feature = "bit-vec")]
    Bit,

    /// PostgreSQL BIT VARYING type - variable-length bit string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-bit.html>
    #[cfg(feature = "bit-vec")]
    Varbit,

    /// PostgreSQL custom ENUM type - user-defined enumerated type
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-enum.html>
    #[cfg(feature = "serde")]
    Enum(String),
}

impl PostgreSQLType {
    /// Convert from attribute name to enum variant
    ///
    /// For native enums, use [`Self::from_enum_attribute`] instead.
    #[must_use]
    pub fn from_attribute_name(name: &str) -> Option<Self> {
        match name {
            // Integer types and aliases
            "integer" | "int" | "int4" => Some(Self::Integer),
            "bigint" | "int8" => Some(Self::Bigint),
            "smallint" | "int2" => Some(Self::Smallint),
            "serial" | "serial4" => Some(Self::Serial),
            "bigserial" | "serial8" => Some(Self::Bigserial),

            // Text types and aliases
            "text" => Some(Self::Text),
            "varchar" | "character_varying" => Some(Self::Varchar),
            "char" | "character" => Some(Self::Char),

            // Float types and aliases
            "real" | "float4" => Some(Self::Real),
            "double_precision" | "float8" | "double" => Some(Self::DoublePrecision),
            "numeric" | "decimal" => Some(Self::Numeric),

            // Other basic types
            "boolean" | "bool" => Some(Self::Boolean),
            "bytea" => Some(Self::Bytea),

            // UUID
            #[cfg(feature = "uuid")]
            "uuid" => Some(Self::Uuid),

            // JSON types
            #[cfg(feature = "serde")]
            "json" => Some(Self::Json),
            #[cfg(feature = "serde")]
            "jsonb" => Some(Self::Jsonb),

            // Date/time types and aliases
            "timestamp" | "timestamp_without_time_zone" => Some(Self::Timestamp),
            "timestamptz" | "timestamp_with_time_zone" => Some(Self::Timestamptz),
            "date" => Some(Self::Date),
            "time" | "time_without_time_zone" => Some(Self::Time),
            "timetz" | "time_with_time_zone" => Some(Self::Timetz),
            #[cfg(feature = "chrono")]
            "interval" => Some(Self::Interval),

            // Network address types
            #[cfg(feature = "cidr")]
            "inet" => Some(Self::Inet),
            #[cfg(feature = "cidr")]
            "cidr" => Some(Self::Cidr),
            #[cfg(feature = "cidr")]
            "macaddr" => Some(Self::MacAddr),
            #[cfg(feature = "cidr")]
            "macaddr8" => Some(Self::MacAddr8),

            // Geometric types
            #[cfg(feature = "geo-types")]
            "point" => Some(Self::Point),
            #[cfg(feature = "geo-types")]
            "line" => Some(Self::Line),
            #[cfg(feature = "geo-types")]
            "lseg" => Some(Self::Lseg),
            #[cfg(feature = "geo-types")]
            "box" => Some(Self::Box),
            #[cfg(feature = "geo-types")]
            "path" => Some(Self::Path),
            #[cfg(feature = "geo-types")]
            "polygon" => Some(Self::Polygon),
            #[cfg(feature = "geo-types")]
            "circle" => Some(Self::Circle),

            // Bit string types
            #[cfg(feature = "bit-vec")]
            "bit" => Some(Self::Bit),
            #[cfg(feature = "bit-vec")]
            "varbit" | "bit_varying" => Some(Self::Varbit),

            "enum" => None, // enum() requires a parameter, handled separately
            _ => None,
        }
    }

    /// Create a native PostgreSQL enum type from enum attribute
    ///
    /// Used for `#[enum(MyEnum)]` syntax.
    #[cfg(feature = "serde")]
    #[must_use]
    pub fn from_enum_attribute(enum_name: &str) -> Self {
        Self::Enum(String::from(enum_name))
    }

    /// Get the SQL type string for this type
    #[must_use]
    pub fn to_sql_type(&self) -> &str {
        match self {
            Self::Integer => "INTEGER",
            Self::Bigint => "BIGINT",
            Self::Smallint => "SMALLINT",
            Self::Serial => "SERIAL",
            Self::Bigserial => "BIGSERIAL",
            Self::Text => "TEXT",
            Self::Varchar => "VARCHAR",
            Self::Char => "CHAR",
            Self::Real => "REAL",
            Self::DoublePrecision => "DOUBLE PRECISION",
            Self::Numeric => "NUMERIC",
            Self::Boolean => "BOOLEAN",
            Self::Bytea => "BYTEA",
            #[cfg(feature = "uuid")]
            Self::Uuid => "UUID",
            #[cfg(feature = "serde")]
            Self::Json => "JSON",
            #[cfg(feature = "serde")]
            Self::Jsonb => "JSONB",
            Self::Timestamp => "TIMESTAMP",
            Self::Timestamptz => "TIMESTAMPTZ",
            Self::Date => "DATE",
            Self::Time => "TIME",
            Self::Timetz => "TIMETZ",
            #[cfg(feature = "chrono")]
            Self::Interval => "INTERVAL",
            #[cfg(feature = "cidr")]
            Self::Inet => "INET",
            #[cfg(feature = "cidr")]
            Self::Cidr => "CIDR",
            #[cfg(feature = "cidr")]
            Self::MacAddr => "MACADDR",
            #[cfg(feature = "cidr")]
            Self::MacAddr8 => "MACADDR8",
            #[cfg(feature = "geo-types")]
            Self::Point => "POINT",
            #[cfg(feature = "geo-types")]
            Self::Line => "LINE",
            #[cfg(feature = "geo-types")]
            Self::Lseg => "LSEG",
            #[cfg(feature = "geo-types")]
            Self::Box => "BOX",
            #[cfg(feature = "geo-types")]
            Self::Path => "PATH",
            #[cfg(feature = "geo-types")]
            Self::Polygon => "POLYGON",
            #[cfg(feature = "geo-types")]
            Self::Circle => "CIRCLE",
            #[cfg(feature = "bit-vec")]
            Self::Bit => "BIT",
            #[cfg(feature = "bit-vec")]
            Self::Varbit => "VARBIT",
            #[cfg(feature = "serde")]
            Self::Enum(name) => name.as_str(), // Custom enum type name
        }
    }

    /// Check if this type is an auto-incrementing type
    #[must_use]
    pub const fn is_serial(&self) -> bool {
        matches!(self, Self::Serial | Self::Bigserial)
    }

    /// Check if this type supports primary keys
    #[must_use]
    pub const fn supports_primary_key(&self) -> bool {
        #[cfg(feature = "serde")]
        {
            !matches!(self, Self::Json | Self::Jsonb)
        }
        #[cfg(not(feature = "serde"))]
        {
            true
        }
    }

    /// Check if a flag is valid for this column type
    #[must_use]
    pub fn is_valid_flag(&self, flag: &str) -> bool {
        match (self, flag) {
            (Self::Serial | Self::Bigserial, "identity") => true,
            (Self::Text | Self::Bytea, "json") => true,
            #[cfg(feature = "serde")]
            (Self::Json | Self::Jsonb, "json") => true,
            (Self::Text | Self::Integer | Self::Smallint | Self::Bigint, "enum") => true,
            #[cfg(feature = "serde")]
            (Self::Enum(_), "enum") => true,
            (_, "primary" | "primary_key" | "unique" | "not_null" | "check") => true,
            _ => false,
        }
    }
}

impl core::fmt::Display for PostgreSQLType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.to_sql_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_attribute_name() {
        assert_eq!(
            PostgreSQLType::from_attribute_name("integer"),
            Some(PostgreSQLType::Integer)
        );
        assert_eq!(
            PostgreSQLType::from_attribute_name("int"),
            Some(PostgreSQLType::Integer)
        );
        assert_eq!(
            PostgreSQLType::from_attribute_name("text"),
            Some(PostgreSQLType::Text)
        );
        assert_eq!(
            PostgreSQLType::from_attribute_name("varchar"),
            Some(PostgreSQLType::Varchar)
        );
        assert_eq!(
            PostgreSQLType::from_attribute_name("serial"),
            Some(PostgreSQLType::Serial)
        );
        assert_eq!(PostgreSQLType::from_attribute_name("unknown"), None);
    }

    #[test]
    fn test_to_sql_type() {
        assert_eq!(PostgreSQLType::Integer.to_sql_type(), "INTEGER");
        assert_eq!(PostgreSQLType::Bigint.to_sql_type(), "BIGINT");
        assert_eq!(PostgreSQLType::Text.to_sql_type(), "TEXT");
        assert_eq!(
            PostgreSQLType::DoublePrecision.to_sql_type(),
            "DOUBLE PRECISION"
        );
        assert_eq!(PostgreSQLType::Boolean.to_sql_type(), "BOOLEAN");
    }

    #[test]
    fn test_is_serial() {
        assert!(PostgreSQLType::Serial.is_serial());
        assert!(PostgreSQLType::Bigserial.is_serial());
        assert!(!PostgreSQLType::Integer.is_serial());
        assert!(!PostgreSQLType::Text.is_serial());
    }
}
