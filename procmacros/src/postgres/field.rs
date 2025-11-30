use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::{collections::HashSet, fmt::Display};
use syn::{Attribute, Error, Expr, ExprPath, Field, Ident, Lit, Meta, Result, Token, Type};

// =============================================================================
// Type Category - Centralized type classification for code generation
// =============================================================================

/// Categorizes Rust types for consistent handling across the macro system.
///
/// This enum provides a single source of truth for type detection, eliminating
/// fragile string matching scattered across multiple files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeCategory {
    /// `arrayvec::ArrayString<N>` - Fixed-capacity string on the stack
    ArrayString,
    /// `arrayvec::ArrayVec<u8, N>` - Fixed-capacity byte array on the stack
    ArrayVec,
    /// `std::string::String` - Heap-allocated string
    String,
    /// `Vec<u8>` - Heap-allocated byte array
    Blob,
    /// `uuid::Uuid` - UUID type (handled specially)
    Uuid,
    /// Any type with `#[json]` flag
    Json,
    /// Any type with `#[enum]` flag  
    Enum,
    /// Primitive types: i32, i64, f32, f64, bool, etc.
    Primitive,
}

impl TypeCategory {
    /// Detect the category from a type string representation.
    ///
    /// Order matters: more specific types (ArrayString) must be checked
    /// before more general types (String).
    pub(crate) fn from_type_string(type_str: &str) -> Self {
        if type_str.contains("ArrayString") {
            TypeCategory::ArrayString
        } else if type_str.contains("ArrayVec") {
            TypeCategory::ArrayVec
        } else if type_str.contains("Uuid") {
            TypeCategory::Uuid
        } else if type_str.contains("String") {
            TypeCategory::String
        } else if type_str.contains("Vec") && type_str.contains("u8") {
            TypeCategory::Blob
        } else {
            TypeCategory::Primitive
        }
    }
}

/// Enum representing supported PostgreSQL column types.
///
/// These correspond to PostgreSQL data types.
/// See: <https://www.postgresql.org/docs/current/datatype.html>
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum PostgreSQLType {
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

    /// PostgreSQL DECIMAL type - exact numeric with selectable precision (alias for NUMERIC)
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-NUMERIC-DECIMAL>
    #[cfg(feature = "rust_decimal")]
    Decimal,

    /// PostgreSQL INET type - IPv4 or IPv6 host address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "ipnet")]
    Inet,

    /// PostgreSQL CIDR type - IPv4 or IPv6 network address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "ipnet")]
    Cidr,

    /// PostgreSQL MACADDR type - MAC address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "ipnet")]
    MacAddr,

    /// PostgreSQL MACADDR8 type - EUI-64 MAC address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "ipnet")]
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
    #[cfg(feature = "bitvec")]
    Bit,

    /// PostgreSQL BIT VARYING type - variable-length bit string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-bit.html>
    #[cfg(feature = "bitvec")]
    Varbit,

    /// PostgreSQL custom ENUM type - user-defined enumerated type
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-enum.html>
    Enum(String), // The enum type name
}

impl Display for PostgreSQLType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql_type())
    }
}

impl PostgreSQLType {
    /// Convert from attribute name to enum variant
    /// For native enums, use `from_enum_attribute` instead
    pub(crate) fn from_attribute_name(name: &str) -> Option<Self> {
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

            // Decimal alias (rust_decimal feature)
            #[cfg(feature = "rust_decimal")]
            "decimal" => Some(Self::Decimal),

            // Network address types
            #[cfg(feature = "ipnet")]
            "inet" => Some(Self::Inet),
            #[cfg(feature = "ipnet")]
            "cidr" => Some(Self::Cidr),
            #[cfg(feature = "ipnet")]
            "macaddr" => Some(Self::MacAddr),
            #[cfg(feature = "ipnet")]
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
            #[cfg(feature = "bitvec")]
            "bit" => Some(Self::Bit),
            #[cfg(feature = "bitvec")]
            "varbit" | "bit_varying" => Some(Self::Varbit),

            "enum" => None, // enum() requires a parameter, handled separately
            _ => None,
        }
    }

    /// Create a native PostgreSQL enum type from enum attribute
    /// Used for #[enum(MyEnum)] syntax
    pub(crate) fn from_enum_attribute(enum_name: &str) -> Self {
        Self::Enum(enum_name.to_string())
    }

    /// Get the SQL type string for this type
    pub(crate) fn to_sql_type(&self) -> &str {
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
            #[cfg(feature = "rust_decimal")]
            Self::Decimal => "DECIMAL",
            #[cfg(feature = "ipnet")]
            Self::Inet => "INET",
            #[cfg(feature = "ipnet")]
            Self::Cidr => "CIDR",
            #[cfg(feature = "ipnet")]
            Self::MacAddr => "MACADDR",
            #[cfg(feature = "ipnet")]
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
            #[cfg(feature = "bitvec")]
            Self::Bit => "BIT",
            #[cfg(feature = "bitvec")]
            Self::Varbit => "VARBIT",
            Self::Enum(name) => name.as_str(), // Custom enum type name
        }
    }

    /// Check if a flag is valid for this column type
    pub(crate) fn is_valid_flag(&self, flag: &str) -> bool {
        match (self, flag) {
            (Self::Serial | Self::Bigserial, "generated_identity") => true,
            (Self::Text | Self::Bytea, "json") => true,
            #[cfg(feature = "serde")]
            (Self::Json | Self::Jsonb, "json") => true,
            (Self::Text | Self::Integer | Self::Smallint | Self::Bigint, "enum") => true,
            (Self::Enum(_), "enum") => true, // Native PostgreSQL enums support enum flag
            (_, "primary" | "primary_key" | "unique" | "not_null" | "check") => true,
            _ => false,
        }
    }

    /// Validate a flag for this column type, returning an error with PostgreSQL docs link if invalid.
    pub(crate) fn validate_flag(&self, flag: &str, span: proc_macro2::Span) -> Result<()> {
        if !self.is_valid_flag(flag) {
            let message = match (self, flag) {
                (non_serial, "generated_identity")
                    if !matches!(non_serial, Self::Serial | Self::Bigserial) =>
                {
                    "generated_identity can only be used with SERIAL or BIGSERIAL columns. \
                        See: https://www.postgresql.org/docs/current/ddl-identity-columns.html"
                        .to_string()
                }
                (non_text_or_binary, "json") => {
                    #[cfg(feature = "serde")]
                    let supports_json = matches!(
                        non_text_or_binary,
                        Self::Text | Self::Bytea | Self::Json | Self::Jsonb
                    );
                    #[cfg(not(feature = "serde"))]
                    let supports_json = matches!(non_text_or_binary, Self::Text | Self::Bytea);

                    if !supports_json {
                        "json can only be used with TEXT, BYTEA, JSON, or JSONB columns. \
                            See: https://www.postgresql.org/docs/current/datatype-json.html"
                            .to_string()
                    } else {
                        return Ok(());
                    }
                }
                (non_enum_compatible, "enum")
                    if !matches!(
                        non_enum_compatible,
                        Self::Text | Self::Integer | Self::Smallint | Self::Bigint | Self::Enum(_)
                    ) =>
                {
                    "enum can only be used with TEXT, INTEGER, SMALLINT, BIGINT, or native ENUM columns. \
                        For custom enum types, see: https://www.postgresql.org/docs/current/datatype-enum.html"
                        .to_string()
                }
                _ => format!("'{flag}' is not valid for {} columns", self.to_sql_type()),
            };

            return Err(Error::new(span, message));
        }
        Ok(())
    }

    /// Check if this type is an auto-incrementing type
    pub(crate) fn is_serial(&self) -> bool {
        matches!(self, Self::Serial | Self::Bigserial)
    }

    /// Check if this type supports primary keys
    pub(crate) fn supports_primary_key(&self) -> bool {
        #[cfg(feature = "serde")]
        {
            !matches!(self, Self::Json | Self::Jsonb)
        }
        #[cfg(not(feature = "serde"))]
        {
            true
        }
    }
}

/// PostgreSQL column constraint flags
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PostgreSQLFlag {
    Primary,
    Unique,
    NotNull,
    GeneratedIdentity,
    /// Used with TEXT/INTEGER columns to store enum as text/discriminant
    Enum,
    /// Used with native PostgreSQL ENUM types - references the enum type name
    NativeEnum(String),
    Json,
    Check(String),
}

impl PostgreSQLFlag {
    /// Parse a flag from its string name and optional value
    pub(crate) fn from_name_and_value(name: &str, value: Option<&Expr>) -> Result<Self> {
        match name {
            "primary" | "primary_key" => Ok(Self::Primary),
            "unique" => Ok(Self::Unique),
            "not_null" => Ok(Self::NotNull),
            "generated_identity" => Ok(Self::GeneratedIdentity),
            "enum" => Ok(Self::Enum),
            "json" => Ok(Self::Json),
            "check" => {
                if let Some(expr) = value {
                    if let Expr::Lit(syn::ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }) = expr
                    {
                        Ok(Self::Check(lit_str.value()))
                    } else {
                        Err(Error::new_spanned(
                            expr,
                            "check constraint must be a string literal",
                        ))
                    }
                } else {
                    Err(Error::new_spanned(
                        name,
                        "check constraint requires a value",
                    ))
                }
            }
            _ => Err(Error::new_spanned(
                name,
                format!("Unknown PostgreSQL flag: {}", name),
            )),
        }
    }

    /// Convert flag to SQL string
    pub(crate) fn to_sql(&self) -> String {
        match self {
            Self::Primary => "PRIMARY KEY".to_string(),
            Self::Unique => "UNIQUE".to_string(),
            Self::NotNull => "NOT NULL".to_string(),
            Self::GeneratedIdentity => "GENERATED ALWAYS AS IDENTITY".to_string(),
            Self::Enum => String::new(), // Handled separately in type conversion
            Self::NativeEnum(_) => String::new(), // Type already specifies the enum name
            Self::Json => String::new(), // Handled separately in type conversion
            Self::Check(constraint) => format!("CHECK ({})", constraint),
        }
    }
}

/// Default value specification for PostgreSQL columns
#[derive(Debug, Clone)]
pub(crate) enum PostgreSQLDefault {
    /// Literal value (e.g., 'default_value')
    Literal(String),
    /// Function call (e.g., NOW())
    Function(String),
    /// Expression using Rust code (evaluated at compile time)
    Expression(TokenStream),
}

/// References specification for PostgreSQL foreign keys
#[derive(Debug, Clone)]
pub(crate) struct PostgreSQLReference {
    pub table: Ident,
    pub column: Ident,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Information about a PostgreSQL table field
#[derive(Clone)]
pub(crate) struct FieldInfo {
    pub ident: Ident,
    pub vis: syn::Visibility,
    pub ty: Type,
    pub column_type: PostgreSQLType,
    pub flags: HashSet<PostgreSQLFlag>,
    pub is_primary: bool,
    pub is_unique: bool,
    pub is_nullable: bool,
    pub is_enum: bool,
    pub is_pgenum: bool,
    pub is_json: bool,
    pub is_serial: bool,
    pub default: Option<PostgreSQLDefault>,
    pub default_fn: Option<TokenStream>,
    pub check_constraint: Option<String>,
    pub foreign_key: Option<PostgreSQLReference>,
    pub has_default: bool,
}

impl FieldInfo {
    /// Parse field information from a struct field
    pub(crate) fn from_field(field: &Field, is_composite_pk: bool) -> Result<Self> {
        let name = field.ident.as_ref().unwrap().clone();
        let vis = field.vis.clone();
        let ty = field.ty.clone();

        // Check if field is nullable (wrapped in Option<T>)
        let is_nullable = Self::is_option_type(&ty);

        // Parse column attributes
        let mut column_type = PostgreSQLType::default();
        let mut flags = HashSet::new();
        let mut default = None;
        let mut default_fn = None;
        let mut check_constraint = None;
        let mut foreign_key = None;

        // Find the column type attribute
        for attr in &field.attrs {
            if let Some(column_info) = Self::parse_column_attribute(attr)? {
                column_type = column_info.column_type;
                flags = column_info.flags;
                default = column_info.default;
                default_fn = column_info.default_fn;
                check_constraint = column_info.check_constraint;
                foreign_key = column_info.foreign_key;
                break;
            }
        }

        // Validate flags against column type
        for flag in &flags {
            match flag {
                PostgreSQLFlag::Primary => column_type.validate_flag("primary", name.span())?,
                PostgreSQLFlag::Unique => column_type.validate_flag("unique", name.span())?,
                PostgreSQLFlag::GeneratedIdentity => {
                    column_type.validate_flag("generated_identity", name.span())?
                }
                PostgreSQLFlag::Enum => column_type.validate_flag("enum", name.span())?,
                PostgreSQLFlag::Json => column_type.validate_flag("json", name.span())?,
                _ => {} // Other flags don't need type validation
            }
        }

        let is_primary = flags.contains(&PostgreSQLFlag::Primary);
        let is_unique = flags.contains(&PostgreSQLFlag::Unique);
        let is_enum = flags.contains(&PostgreSQLFlag::Enum);
        let is_pgenum = flags
            .iter()
            .any(|f| matches!(f, PostgreSQLFlag::NativeEnum(_)));
        let is_json = flags.contains(&PostgreSQLFlag::Json);
        let is_serial = column_type.is_serial();
        let has_default = default.is_some() || default_fn.is_some() || is_serial;

        Ok(FieldInfo {
            ident: name,
            vis,
            ty,
            column_type,
            flags,
            is_primary,
            is_unique,
            is_nullable,
            is_enum,
            is_pgenum,
            is_json,
            is_serial,
            default,
            default_fn,
            check_constraint,
            foreign_key,
            has_default,
        })
    }

    /// Check if a type is Option<T>
    fn is_option_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty
            && let Some(segment) = type_path.path.segments.last()
        {
            return segment.ident == "Option";
        }
        false
    }

    /// Extract the inner type from Option<T>, returning T
    /// If the type is not Option<T>, returns the original type
    fn extract_option_inner(ty: &Type) -> &Type {
        if let Type::Path(type_path) = ty
            && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return inner;
        }
        ty
    }

    /// Get the base type (inner type for Option<T>, or the type itself for non-Option)
    pub(crate) fn base_type(&self) -> &Type {
        Self::extract_option_inner(&self.ty)
    }

    /// Parse column attribute information
    fn parse_column_attribute(attr: &Attribute) -> Result<Option<ColumnInfo>> {
        // Only process attributes that match PostgreSQL column types
        let attr_name = if let Some(ident) = attr.path().get_ident() {
            ident.to_string()
        } else if attr.path().segments.len() == 1 {
            // Handle keyword identifiers like `enum`
            attr.path().segments.first().unwrap().ident.to_string()
        } else {
            return Ok(None);
        };

        // Handle native enum attribute #[enum(MyEnum)] or #[r#enum(MyEnum)]
        if attr_name == "enum" || attr_name == "r#enum" {
            return Self::parse_native_enum_attribute(attr);
        }

        let Some(column_type) = PostgreSQLType::from_attribute_name(&attr_name) else {
            return Ok(None);
        };

        let mut flags = HashSet::new();
        let mut default = None;
        let mut default_fn = None;
        let mut check_constraint = None;
        let mut foreign_key = None;

        // Parse attribute arguments
        if attr.meta.require_list().is_ok() {
            attr.parse_nested_meta(|meta| {
                let path = meta.path.get_ident().unwrap().to_string();

                match path.as_str() {
                    "primary" | "primary_key" => {
                        flags.insert(PostgreSQLFlag::Primary);
                    }
                    "unique" => {
                        flags.insert(PostgreSQLFlag::Unique);
                    }
                    "not_null" => {
                        flags.insert(PostgreSQLFlag::NotNull);
                    }
                    "generated_identity" => {
                        flags.insert(PostgreSQLFlag::GeneratedIdentity);
                    }
                    "enum" => {
                        flags.insert(PostgreSQLFlag::Enum);
                    }
                    "json" => {
                        flags.insert(PostgreSQLFlag::Json);
                    }
                    "default" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let lit: Lit = meta.input.parse()?;
                            if let Lit::Str(s) = lit {
                                default = Some(PostgreSQLDefault::Literal(s.value()));
                            }
                        }
                    }
                    "default_fn" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let expr: Expr = meta.input.parse()?;
                            default_fn = Some(quote! { #expr });
                        }
                    }
                    "check" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let lit: Lit = meta.input.parse()?;
                            if let Lit::Str(s) = lit {
                                check_constraint = Some(s.value());
                                flags.insert(PostgreSQLFlag::Check(s.value()));
                            }
                        }
                    }
                    "references" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let path: ExprPath = meta.input.parse()?;
                            foreign_key = Some(Self::parse_reference(&path)?);
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &meta.path,
                            format!("Unknown attribute: {}", path),
                        ));
                    }
                }
                Ok(())
            })?;
        }

        Ok(Some(ColumnInfo {
            column_type,
            flags,
            default,
            default_fn,
            check_constraint,
            foreign_key,
        }))
    }

    /// Parse native PostgreSQL enum attribute #[enum(MyEnum)]
    fn parse_native_enum_attribute(attr: &Attribute) -> Result<Option<ColumnInfo>> {
        let mut enum_name = None;

        // Parse the enum type name from #[enum(MyEnum)]
        if let Meta::List(meta_list) = &attr.meta {
            let tokens = &meta_list.tokens;
            let enum_ident: syn::Ident = syn::parse2(tokens.clone())?;
            enum_name = Some(enum_ident.to_string());
        } else {
            return Err(Error::new_spanned(
                attr,
                "enum attribute requires a type name: #[enum(MyEnum)]",
            ));
        }

        let Some(enum_name) = enum_name else {
            return Err(Error::new_spanned(
                attr,
                "enum attribute requires a type name: #[enum(MyEnum)]",
            ));
        };

        let column_type = PostgreSQLType::from_enum_attribute(&enum_name);
        let mut flags = HashSet::new();
        flags.insert(PostgreSQLFlag::NativeEnum(enum_name));

        Ok(Some(ColumnInfo {
            column_type,
            flags,
            default: None,
            default_fn: None,
            check_constraint: None,
            foreign_key: None,
        }))
    }

    /// Parse foreign key reference from path expression
    fn parse_reference(path: &ExprPath) -> Result<PostgreSQLReference> {
        // Convert ExprPath to string format like "Users::id"

        if !path.path.segments.len().eq(&2) {
            return Err(Error::new_spanned(
                path,
                "References must be in the format Table::column",
            ));
        }

        let table = path
            .path
            .segments
            .first()
            .ok_or(Error::new_spanned(
                path,
                "References must be in the format Table::column",
            ))?
            .ident
            .clone();
        let column = path
            .path
            .segments
            .last()
            .ok_or(Error::new_spanned(
                path,
                "References must be in the format Table::column",
            ))?
            .ident
            .clone();

        Ok(PostgreSQLReference {
            table,
            column,
            on_delete: None, // TODO: Add support for ON DELETE/UPDATE actions
            on_update: None,
        })
    }
}

impl FieldInfo {
    /// Get the category of this field's type for code generation decisions.
    ///
    /// This provides a single source of truth for type handling, eliminating
    /// scattered string matching throughout the codebase.
    pub(crate) fn type_category(&self) -> TypeCategory {
        // Special flags take precedence
        if self.is_json {
            return TypeCategory::Json;
        }
        if self.is_enum || self.is_pgenum {
            return TypeCategory::Enum;
        }

        // Detect from the base type string
        let type_str = self.ty.to_token_stream().to_string();
        TypeCategory::from_type_string(&type_str)
    }
}

/// Intermediate structure for parsing column information
struct ColumnInfo {
    column_type: PostgreSQLType,
    flags: HashSet<PostgreSQLFlag>,
    default: Option<PostgreSQLDefault>,
    default_fn: Option<TokenStream>,
    check_constraint: Option<String>,
    foreign_key: Option<PostgreSQLReference>,
}
