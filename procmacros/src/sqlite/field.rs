use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::{collections::HashSet, fmt::Display};
use syn::{
    Attribute, Error, Expr, ExprPath, Field, Ident, Lit, Meta, Result, Token, Type,
    parse::ParseStream,
};

// =============================================================================
// Type Category - Centralized type classification for code generation
// =============================================================================

/// Categorizes Rust types for consistent handling across the macro system.
///
/// This enum provides a single source of truth for type detection, eliminating
/// fragile string matching scattered across multiple files. This is used for
/// both type inference (Rust type → SQLite type) and code generation.
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
    /// `[u8; N]` - Fixed-size byte array
    ByteArray,
    /// `uuid::Uuid` - UUID type (defaults to BLOB, can be overridden to TEXT)
    Uuid,
    /// Any type with `#[json]` flag or `serde_json::Value`
    Json,
    /// Any type with `#[enum]` flag (defaults to TEXT, can be INTEGER)
    Enum,
    /// `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32` - Integer types
    Integer,
    /// `f32`, `f64` - Floating point types
    Real,
    /// `bool` - Boolean type (stored as INTEGER 0/1)
    Bool,
    /// Chrono date/time types - stored as TEXT
    DateTime,
    /// Unknown type - requires explicit type annotation
    Unknown,
}

impl TypeCategory {
    /// Detect the category from a type string representation.
    ///
    /// Order matters: more specific types (ArrayString) must be checked
    /// before more general types (String).
    pub(crate) fn from_type_string(type_str: &str) -> Self {
        // Remove whitespace for consistent matching
        let type_str = type_str.replace(' ', "");

        // Handle Option<T> wrapper - recurse into inner type
        if type_str.starts_with("Option<") && type_str.ends_with('>') {
            let inner = &type_str[7..type_str.len() - 1];
            return Self::from_type_string(inner);
        }

        // Fixed-size byte arrays first
        if type_str.starts_with("[u8;") || type_str.contains("[u8;") {
            return TypeCategory::ByteArray;
        }

        // ArrayVec/ArrayString before generic checks
        if type_str.contains("ArrayString") {
            return TypeCategory::ArrayString;
        }
        if type_str.contains("ArrayVec") && type_str.contains("u8") {
            return TypeCategory::ArrayVec;
        }

        // UUID
        if type_str.contains("Uuid") {
            return TypeCategory::Uuid;
        }

        // JSON (serde_json::Value)
        if type_str.contains("serde_json::Value") || type_str == "Value" {
            return TypeCategory::Json;
        }

        // Chrono types - all stored as TEXT in SQLite
        if type_str.contains("NaiveDate")
            || type_str.contains("NaiveTime")
            || type_str.contains("NaiveDateTime")
            || type_str.contains("DateTime<")
        {
            return TypeCategory::DateTime;
        }

        // Time crate types
        if type_str.contains("time::Date")
            || type_str.contains("time::Time")
            || type_str.contains("PrimitiveDateTime")
            || type_str.contains("OffsetDateTime")
        {
            return TypeCategory::DateTime;
        }

        // String types
        if type_str.contains("String") {
            return TypeCategory::String;
        }

        // Vec<u8>
        if type_str.contains("Vec<u8>") {
            return TypeCategory::Blob;
        }

        // Primitives - check exact matches for simple types
        match type_str.as_str() {
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "isize" | "usize" => {
                TypeCategory::Integer
            }
            "f32" | "f64" => TypeCategory::Real,
            "bool" => TypeCategory::Bool,
            _ => TypeCategory::Unknown,
        }
    }

    /// Infer the SQLite type from this category.
    ///
    /// Returns Some(SQLiteType) for types that can be automatically inferred,
    /// or None for types that require explicit annotation (Unknown, Enum without context).
    pub(crate) fn to_sqlite_type(&self) -> Option<SQLiteType> {
        match self {
            // Integer types → INTEGER
            TypeCategory::Integer | TypeCategory::Bool => Some(SQLiteType::Integer),
            // Floating point → REAL
            TypeCategory::Real => Some(SQLiteType::Real),
            // String types → TEXT
            TypeCategory::String | TypeCategory::ArrayString | TypeCategory::DateTime => {
                Some(SQLiteType::Text)
            }
            // Binary types → BLOB
            TypeCategory::Blob | TypeCategory::ArrayVec | TypeCategory::ByteArray => {
                Some(SQLiteType::Blob)
            }
            // UUID defaults to BLOB (more efficient), but can be overridden to TEXT
            TypeCategory::Uuid => Some(SQLiteType::Blob),
            // JSON defaults to TEXT (human-readable), but can be overridden to BLOB
            TypeCategory::Json => Some(SQLiteType::Text),
            // Enum defaults to TEXT (variant names), but can be overridden to INTEGER
            TypeCategory::Enum => Some(SQLiteType::Text),
            // Unknown types require explicit annotation
            TypeCategory::Unknown => None,
        }
    }

    /// Check if this category requires the FromSQLiteValue trait for conversion
    #[allow(dead_code)]
    pub(crate) fn uses_from_sqlite_value(&self) -> bool {
        matches!(self, TypeCategory::ArrayString | TypeCategory::ArrayVec)
    }

    /// Check if this category should use a generic `impl Into<...>` parameter
    #[allow(dead_code)]
    pub(crate) fn uses_into_param(&self) -> bool {
        matches!(
            self,
            TypeCategory::String | TypeCategory::Blob | TypeCategory::Uuid
        )
    }
}

/// Enum representing supported SQLite column types.
///
/// These correspond to the [SQLite storage classes](https://sqlite.org/datatype3.html#storage_classes_and_datatypes).
/// Each type maps to specific Rust types and has different capabilities for constraints and features.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum SQLiteType {
    /// SQLite INTEGER type - stores signed integers up to 8 bytes.
    ///
    /// See: <https://sqlite.org/datatype3.html#integer_datatype>
    ///
    /// Supports: primary keys, autoincrement, enums (discriminant storage)
    Integer,

    /// SQLite TEXT type - stores text in UTF-8, UTF-16BE, or UTF-16LE encoding.
    ///
    /// See: <https://sqlite.org/datatype3.html#text_datatype>
    ///
    /// Supports: enums (variant name storage), JSON serialization
    Text,

    /// SQLite BLOB type - stores binary data exactly as input.
    ///
    /// See: <https://sqlite.org/datatype3.html#blob_datatype>
    ///
    /// Supports: JSON serialization, UUID storage
    Blob,

    /// SQLite REAL type - stores floating point values as 8-byte IEEE floating point numbers.
    ///
    /// See: <https://sqlite.org/datatype3.html#real_datatype>
    Real,

    /// SQLite NUMERIC type - stores values as INTEGER, REAL, or TEXT depending on the value.
    ///
    /// See: <https://sqlite.org/datatype3.html#numeric_datatype>
    Numeric,

    /// SQLite ANY type - no type affinity, can store any type of data.
    ///
    /// See: <https://sqlite.org/datatype3.html#type_affinity>
    #[default]
    Any,
}

impl Display for SQLiteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql_type())
    }
}

impl SQLiteType {
    /// Convert from attribute name to enum variant
    pub(crate) fn from_attribute_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "integer" => Some(Self::Integer),
            "text" => Some(Self::Text),
            "blob" => Some(Self::Blob),
            "real" => Some(Self::Real),
            "number" | "numeric" => Some(Self::Numeric),
            "boolean" => Some(Self::Integer), // Store booleans as integers (0/1)
            "any" => Some(Self::Any),
            _ => None,
        }
    }

    /// Get the SQL type string for this type
    pub(crate) fn to_sql_type(&self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Text => "TEXT",
            Self::Blob => "BLOB",
            Self::Real => "REAL",
            Self::Numeric => "NUMERIC",
            Self::Any => "ANY",
        }
    }

    /// Check if a flag is valid for this column type
    pub(crate) fn is_valid_flag(&self, flag: &str) -> bool {
        matches!(
            (self, flag),
            (Self::Integer, "autoincrement")
                | (Self::Text | Self::Blob, "json")
                | (Self::Text | Self::Integer, "enum")
                | (_, "primary" | "primary_key" | "unique")
        )
    }

    /// Validate a flag for this column type, returning an error with SQLite docs link if invalid.
    ///
    /// Provides helpful error messages with links to relevant SQLite documentation
    /// when incompatible flag/type combinations are used.
    pub(crate) fn validate_flag(&self, flag: &str, attr: &Attribute) -> Result<()> {
        if !self.is_valid_flag(flag) {
            let error_msg = match flag {
                "autoincrement" => {
                    "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY columns.\n\
                     \n\
                     SQLite AUTOINCREMENT ensures that new rows get unique rowids, but it only \
                     works on INTEGER PRIMARY KEY columns in regular (non-WITHOUT ROWID) tables.\n\
                     \n\
                     See: https://sqlite.org/autoinc.html\n\
                     Use: #[integer(primary, autoincrement)]"
                }
                "json" => {
                    "JSON serialization is only supported for TEXT or BLOB column types.\n\
                     \n\
                     JSON data can be stored as TEXT (human-readable) or BLOB (binary). \
                     The choice affects storage size and query capabilities.\n\
                     \n\
                     See: https://sqlite.org/json1.html\n\
                     Use: #[text(json)] or #[blob(json)]"
                }
                "enum" => {
                    "Enum serialization is supported for TEXT (string) or INTEGER (discriminant) columns.\n\
                     \n\
                     - TEXT storage: stores variant names like 'Active', 'Inactive'\n\
                     - INTEGER storage: stores discriminant values like 0, 1, 2\n\
                     \n\
                     Use: #[text(enum)] or #[integer(enum)]"
                }
                "not_null" => {
                    "Use Option<T> in your struct field to represent nullable columns instead of 'not_null' attribute.\n\
                     \n\
                     Drizzle RS uses Rust's type system for nullability:\n\
                     - Field type `T` = NOT NULL column\n\
                     - Field type `Option<T>` = NULL allowed column\n\
                     \n\
                     See: https://sqlite.org/lang_createtable.html#notnullconst\n\
                     Example: pub email: Option<String> for nullable TEXT"
                }
                _ => return Ok(()),
            };

            return Err(Error::new_spanned(attr, error_msg));
        }

        Ok(())
    }
}

/// Foreign key reference information
#[derive(Debug, Clone)]
pub(crate) struct ForeignKeyReference {
    /// The referenced table identifier (e.g., "User" from User::id)
    pub(crate) table_ident: Ident,
    /// The referenced column identifier (e.g., "id" from User::id)
    pub(crate) column_ident: Ident,
    /// ON DELETE action (e.g., "CASCADE", "SET NULL")
    pub(crate) on_delete: Option<String>,
    /// ON UPDATE action (e.g., "CASCADE", "SET NULL")
    pub(crate) on_update: Option<String>,
}

/// Comprehensive field information for code generation
#[derive(Clone)]
pub(crate) struct FieldInfo<'a> {
    // Basic field identifiers and types
    pub(crate) ident: &'a Ident,
    /// The original field type (e.g., Option<String> or i32)
    pub(crate) field_type: &'a Type,
    /// The base type with Option<> unwrapped (e.g., String from Option<String>)
    pub(crate) base_type: &'a Type,

    // Database mapping
    pub(crate) column_name: String,
    pub(crate) sql_definition: String,

    // Field properties
    pub(crate) is_nullable: bool,
    pub(crate) has_default: bool,
    pub(crate) is_primary: bool,
    pub(crate) is_autoincrement: bool,
    pub(crate) is_unique: bool,
    pub(crate) is_json: bool,
    pub(crate) is_enum: bool,
    pub(crate) is_uuid: bool,
    pub(crate) column_type: SQLiteType,

    // Foreign key support
    pub(crate) foreign_key: Option<ForeignKeyReference>,

    // Attribute values
    pub(crate) default_value: Option<Expr>,
    pub(crate) default_fn: Option<Expr>,

    // Original marker expressions for IDE hover documentation
    // These preserve the original tokens so rust-analyzer can resolve them
    pub(crate) marker_exprs: Vec<syn::ExprPath>,

    // Type representations for models
    pub(crate) select_type: Option<TokenStream>,
    pub(crate) update_type: Option<TokenStream>,
}

/// Parse attribute items, handling reserved keywords like 'enum'
fn parse_item(input: ParseStream) -> Result<Expr> {
    let lookahead = input.lookahead1();

    if lookahead.peek(Token![enum]) {
        input.parse::<Token![enum]>()?;
        let ident = syn::Ident::new("enum", proc_macro2::Span::call_site());
        Ok(syn::Expr::Path(syn::ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: syn::Path::from(ident),
        }))
    } else {
        input.parse::<Expr>()
    }
}

#[derive(Default)]
struct ParsedArgs {
    default_value: Option<Expr>,
    default_fn: Option<Expr>,
    references: Option<Expr>,
    on_delete: Option<String>,
    on_update: Option<String>,
    name: Option<Expr>,
    flags: HashSet<String>,
    /// Original marker expressions for IDE hover documentation
    /// These preserve the original tokens so rust-analyzer can resolve them
    marker_exprs: Vec<syn::ExprPath>,
    /// Explicit SQLite type override (e.g., from #[column(text)] or #[column(blob)])
    explicit_type: Option<SQLiteType>,
}

#[derive(Default)]
struct AttributeData {
    column_type: SQLiteType,
    /// Whether the type was explicitly specified (vs inferred from Rust type)
    has_explicit_type: bool,
    flags: HashSet<String>,
    default_value: Option<Expr>,
    default_fn: Option<Expr>,
    references_path: Option<ExprPath>,
    on_delete: Option<String>,
    on_update: Option<String>,
    attr_name: Option<String>,
    /// Original marker expressions for IDE hover documentation
    marker_exprs: Vec<syn::ExprPath>,
}

struct FieldProperties {
    is_primary: bool,
    is_autoincrement: bool,
    is_unique: bool,
    is_json: bool,
    is_enum: bool,
    is_uuid: bool,
    has_default: bool,
}

impl FieldProperties {
    fn from_flags_and_types(flags: &HashSet<String>, _field_type: &Type, base_type: &Type) -> Self {
        Self {
            is_primary: flags.contains("primary_key") || flags.contains("primary"),
            is_autoincrement: flags.contains("autoincrement"),
            is_unique: flags.contains("unique"),
            is_json: flags.contains("json"),
            is_enum: flags.contains("enum"),
            is_uuid: base_type.to_token_stream().to_string().contains("Uuid"),
            has_default: false, // Will be set in build() based on actual values
        }
    }
}

impl<'a> FieldInfo<'a> {
    /// Create an ExprPath with an UPPERCASE ident but preserving the original span.
    ///
    /// This allows users to write `#[column(primary)]` (lowercase) but the generated
    /// code references `PRIMARY` (uppercase, resolves to prelude). The preserved span
    /// enables IDE hover documentation by linking back to the user's source.
    fn make_uppercase_path(original_ident: &syn::Ident, uppercase_name: &str) -> syn::ExprPath {
        let new_ident = syn::Ident::new(uppercase_name, original_ident.span());
        syn::ExprPath {
            attrs: vec![],
            qself: None,
            path: new_ident.into(),
        }
    }

    /// Validate a referential action (ON DELETE/ON UPDATE)
    fn validate_referential_action(action: &syn::Ident) -> Result<String> {
        let action_str = action.to_string().to_ascii_uppercase();
        match action_str.as_str() {
            "CASCADE" => Ok("CASCADE".to_string()),
            "SET_NULL" => Ok("SET NULL".to_string()),
            "SET_DEFAULT" => Ok("SET DEFAULT".to_string()),
            "RESTRICT" => Ok("RESTRICT".to_string()),
            "NO_ACTION" => Ok("NO ACTION".to_string()),
            _ => Err(Error::new_spanned(
                action,
                format!(
                    "Invalid referential action '{}'. Supported: CASCADE, SET_NULL, SET_DEFAULT, RESTRICT, NO_ACTION",
                    action_str
                ),
            )),
        }
    }

    /// Parse attribute arguments, extracting flags and named parameters.
    ///
    /// Supports:
    /// - SQLite type overrides: `text`, `integer`, `blob`, `real`, `any`
    /// - Constraint flags: `primary`, `unique`, `autoincrement`, `json`, `enum`
    /// - Named parameters: `default = value`, `default_fn = func`, `references = Table::col`
    fn parse_args(input: ParseStream) -> Result<ParsedArgs> {
        if input.is_empty() {
            return Ok(ParsedArgs::default());
        }

        let items = input.parse_terminated(parse_item, Token![,])?;
        let mut args = ParsedArgs::default();

        items.into_iter().for_each(|expr| match expr {
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    let ident_str = ident.to_string();
                    // Match case-insensitively - create UPPERCASE ident with original span for IDE hover
                    // This allows users to write lowercase but resolves to UPPERCASE prelude exports
                    let upper = ident_str.to_ascii_uppercase();
                    match upper.as_str() {
                        "JSON" => {
                            // JSON = TEXT storage with JSON serialization
                            args.explicit_type = Some(SQLiteType::Text);
                            args.flags.insert("json".to_string());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(ident, "JSON"));
                        }
                        "JSONB" => {
                            // JSONB = BLOB storage with JSON serialization
                            args.explicit_type = Some(SQLiteType::Blob);
                            args.flags.insert("json".to_string());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(ident, "JSONB"));
                        }
                        "DEFAULT" => {
                            args.default_fn = Some(syn::parse_quote!(Default::default));
                        }
                        "ENUM" => {
                            args.flags.insert("enum".to_string());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(ident, "ENUM"));
                        }
                        "PRIMARY" | "PRIMARY_KEY" => {
                            args.flags.insert("primary".to_string());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(ident, "PRIMARY"));
                        }
                        "AUTOINCREMENT" => {
                            args.flags.insert("autoincrement".to_string());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(ident, "AUTOINCREMENT"));
                        }
                        "UNIQUE" => {
                            args.flags.insert("unique".to_string());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(ident, "UNIQUE"));
                        }
                        _ => {
                            // Check if this is a SQLite type override (case-insensitive for types)
                            if let Some(sqlite_type) = SQLiteType::from_attribute_name(&ident_str) {
                                args.explicit_type = Some(sqlite_type);
                            } else {
                                args.flags.insert(ident_str.clone());
                            }
                        }
                    }
                }
            }
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left
                    && let Some(param) = path.path.get_ident()
                {
                    let param_str = param.to_string();
                    // Match case-insensitively - create UPPERCASE ident with original span for IDE hover
                    let upper = param_str.to_ascii_uppercase();
                    match upper.as_str() {
                        "DEFAULT" => {
                            args.default_value = Some(*assign.right);
                            args.marker_exprs
                                .push(Self::make_uppercase_path(param, "DEFAULT"));
                        }
                        "DEFAULT_FN" => {
                            args.default_fn = Some(*assign.right);
                            args.marker_exprs
                                .push(Self::make_uppercase_path(param, "DEFAULT_FN"));
                        }
                        "REFERENCES" => {
                            args.references = Some(*assign.right.clone());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(param, "REFERENCES"));
                        }
                        "ON_DELETE" => {
                            if let Expr::Path(action_path) = &*assign.right {
                                if let Some(action_ident) = action_path.path.get_ident() {
                                    let action_upper =
                                        action_ident.to_string().to_ascii_uppercase();
                                    args.on_delete =
                                        Self::validate_referential_action(action_ident).ok();
                                    args.marker_exprs
                                        .push(Self::make_uppercase_path(param, "ON_DELETE"));
                                    // Add marker for the action value (CASCADE, SET_NULL, etc.)
                                    args.marker_exprs.push(Self::make_uppercase_path(
                                        action_ident,
                                        &action_upper,
                                    ));
                                }
                            }
                        }
                        "ON_UPDATE" => {
                            if let Expr::Path(action_path) = &*assign.right {
                                if let Some(action_ident) = action_path.path.get_ident() {
                                    let action_upper =
                                        action_ident.to_string().to_ascii_uppercase();
                                    args.on_update =
                                        Self::validate_referential_action(action_ident).ok();
                                    args.marker_exprs
                                        .push(Self::make_uppercase_path(param, "ON_UPDATE"));
                                    // Add marker for the action value (CASCADE, SET_NULL, etc.)
                                    args.marker_exprs.push(Self::make_uppercase_path(
                                        action_ident,
                                        &action_upper,
                                    ));
                                }
                            }
                        }
                        "NAME" => {
                            args.name = Some(*assign.right.clone());
                            args.marker_exprs
                                .push(Self::make_uppercase_path(param, "NAME"));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        });

        Ok(args)
    }

    /// Parse field information from a struct field
    pub(crate) fn from_field(field: &'a Field, is_part_of_composite_pk: bool) -> Result<Self> {
        let Some(field_name) = &field.ident else {
            return Err(Error::new_spanned(
                field,
                "All struct fields must have names. Tuple structs are not supported.\n\
                 Example: pub field_name: String",
            ));
        };

        let attrs = Self::parse_attributes(&field.attrs)?;
        Self::build(field_name, &field.ty, attrs, is_part_of_composite_pk)
    }

    /// Parse field attributes to extract column information.
    ///
    /// Supports two syntaxes:
    /// 1. Legacy: `#[text]`, `#[integer(primary)]`, etc. - type from attribute name
    /// 2. New: `#[column(primary)]`, `#[column(text, primary)]` - type inferred or explicit
    ///
    /// For the new syntax, if no explicit type is provided, the SQLite type is
    /// inferred from the Rust type using `TypeCategory::to_sqlite_type()`.
    fn parse_attributes(attrs: &[Attribute]) -> Result<AttributeData> {
        let mut data = AttributeData::default();

        for attr in attrs {
            let Some(ident) = attr.path().get_ident() else {
                continue;
            };
            let attr_name = ident.to_string();

            // Check for legacy type attribute (#[text], #[integer], etc.)
            if let Some(column_type) = SQLiteType::from_attribute_name(&attr_name) {
                data.column_type = column_type.clone();
                data.has_explicit_type = true;

                // Handle empty attributes like #[text]
                if matches!(&attr.meta, Meta::Path(_)) {
                    continue;
                }

                // Parse arguments for legacy syntax
                if let Ok(args) = attr.parse_args_with(Self::parse_args) {
                    // Validate flags against the explicit column type
                    args.flags
                        .iter()
                        .try_for_each(|flag| column_type.validate_flag(flag, attr))?;

                    data.flags.extend(args.flags);
                    data.default_value = data.default_value.or(args.default_value);
                    data.default_fn = data.default_fn.or(args.default_fn);
                    data.marker_exprs.extend(args.marker_exprs);
                    data.on_delete = data.on_delete.or(args.on_delete);
                    data.on_update = data.on_update.or(args.on_update);

                    if let Some(Expr::Path(path)) = args.references {
                        data.references_path = Some(path);
                    }

                    if let Some(Expr::Lit(expr_lit)) = args.name
                        && let Lit::Str(lit_str) = expr_lit.lit
                    {
                        data.attr_name = Some(lit_str.value());
                    }
                }
                continue;
            }

            // Check for new #[column(...)] syntax
            if attr_name == "column" {
                // Handle empty #[column] (just type inference, no constraints)
                if matches!(&attr.meta, Meta::Path(_)) {
                    continue;
                }

                // Parse arguments for new syntax
                if let Ok(args) = attr.parse_args_with(Self::parse_args) {
                    // If explicit type was provided in args, use it
                    if let Some(explicit_type) = args.explicit_type {
                        data.column_type = explicit_type.clone();
                        data.has_explicit_type = true;

                        // Validate flags against the explicit type
                        args.flags
                            .iter()
                            .try_for_each(|flag| explicit_type.validate_flag(flag, attr))?;
                    }
                    // Otherwise, type will be inferred in build()

                    data.flags.extend(args.flags);
                    data.default_value = data.default_value.or(args.default_value);
                    data.default_fn = data.default_fn.or(args.default_fn);
                    data.marker_exprs.extend(args.marker_exprs);
                    data.on_delete = data.on_delete.or(args.on_delete);
                    data.on_update = data.on_update.or(args.on_update);

                    if let Some(Expr::Path(path)) = args.references {
                        data.references_path = Some(path);
                    }

                    if let Some(Expr::Lit(expr_lit)) = args.name
                        && let Lit::Str(lit_str) = expr_lit.lit
                    {
                        data.attr_name = Some(lit_str.value());
                    }
                }
            }
        }

        // Validate: on_delete and on_update require references
        if (data.on_delete.is_some() || data.on_update.is_some()) && data.references_path.is_none()
        {
            let msg = if data.on_delete.is_some() && data.on_update.is_some() {
                "on_delete and on_update require a references attribute.\n\
                 Example: #[column(references = Table::column, on_delete = CASCADE, on_update = CASCADE)]"
            } else if data.on_delete.is_some() {
                "on_delete requires a references attribute.\n\
                 Example: #[column(references = Table::column, on_delete = CASCADE)]"
            } else {
                "on_update requires a references attribute.\n\
                 Example: #[column(references = Table::column, on_update = CASCADE)]"
            };
            // Use the first marker as span source for the error
            if let Some(marker) = data.marker_exprs.first() {
                return Err(Error::new_spanned(marker, msg));
            } else {
                return Err(Error::new(proc_macro2::Span::call_site(), msg));
            }
        }

        Ok(data)
    }

    /// Build FieldInfo from parsed components
    fn build(
        field_name: &'a Ident,
        field_type: &'a Type,
        attrs: AttributeData,
        is_part_of_composite_pk: bool,
    ) -> Result<Self> {
        let column_name = attrs
            .attr_name
            .clone()
            .unwrap_or_else(|| field_name.to_string());
        let is_nullable = is_option_type(field_type);
        let base_type = if is_nullable {
            extract_option_inner(field_type).unwrap_or(field_type)
        } else {
            field_type
        };

        let mut properties =
            FieldProperties::from_flags_and_types(&attrs.flags, field_type, base_type);
        properties.has_default = attrs.default_value.is_some() || attrs.default_fn.is_some();

        // Determine the SQLite type:
        // 1. Use explicit type from attribute if provided
        // 2. Otherwise, infer from Rust type
        let type_str = base_type.to_token_stream().to_string();
        let type_category = if properties.is_json {
            TypeCategory::Json
        } else if properties.is_enum {
            TypeCategory::Enum
        } else {
            TypeCategory::from_type_string(&type_str)
        };

        let column_type = if attrs.has_explicit_type {
            // Use the explicit type from the attribute
            attrs.column_type.clone()
        } else {
            // Infer from Rust type
            type_category.to_sqlite_type().unwrap_or_else(|| {
                // If we can't infer, default to ANY (flexible SQLite type)
                // This allows unknown types to work but may cause runtime issues
                SQLiteType::Any
            })
        };

        Self::validate_constraints(
            &column_type,
            &properties,
            &attrs.default_value,
            &attrs.default_fn,
            field_name,
        )?;

        let sql_definition = build_sql_definition(
            &column_name,
            &column_type,
            properties.is_primary && !is_part_of_composite_pk,
            !is_nullable,
            properties.is_unique,
            properties.is_autoincrement,
            &attrs.default_value,
        );

        // Detect foreign key reference from the attributes (references = Table::column)
        let foreign_key = if let Some(ref path) = attrs.references_path {
            detect_foreign_key_reference_from_path(
                path,
                attrs.on_delete.clone(),
                attrs.on_update.clone(),
            )
        } else {
            None
        };

        Ok(FieldInfo {
            ident: field_name,
            field_type,
            base_type,
            column_name,
            sql_definition,
            is_nullable,
            has_default: properties.has_default,
            is_primary: properties.is_primary,
            is_autoincrement: properties.is_autoincrement,
            is_unique: properties.is_unique,
            is_json: properties.is_json,
            is_enum: properties.is_enum,
            is_uuid: properties.is_uuid,
            column_type,
            foreign_key,
            default_value: attrs.default_value,
            default_fn: attrs.default_fn,
            marker_exprs: attrs.marker_exprs,
            select_type: Some(select_type(base_type, is_nullable, properties.has_default)),
            update_type: Some(update_type(base_type)),
        })
    }

    /// Validate field constraints and configuration
    fn validate_constraints(
        column_type: &SQLiteType,
        props: &FieldProperties,
        default_value: &Option<Expr>,
        default_fn: &Option<Expr>,
        field_name: &Ident,
    ) -> Result<()> {
        let validations = [
            (
                props.is_autoincrement && !matches!(column_type, SQLiteType::Integer),
                "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY.\n\
              See: https://sqlite.org/autoinc.html\n\
              Hint: Change column type to '#[integer(primary, autoincrement)]'",
            ),
            (
                props.is_autoincrement && !props.is_primary,
                "AUTOINCREMENT requires PRIMARY KEY constraint.\n\
              See: https://sqlite.org/autoinc.html\n\
              Hint: Add 'primary' flag: '#[integer(primary, autoincrement)]'",
            ),
            (
                default_value.is_some() && default_fn.is_some(),
                "Cannot specify both 'default' (compile-time literal) and 'default_fn' (runtime function).\n\
              Choose one: either 'default = literal' or 'default_fn = function'\n\
              Examples:\n  #[text(default = \"hello\")] for compile-time defaults\n  #[text(default_fn = String::new)] for runtime defaults",
            ),
            (
                props.is_uuid && !matches!(column_type, SQLiteType::Blob | SQLiteType::Text),
                "UUID fields must use either BLOB or TEXT column type.\n\
              BLOB storage: Efficient 16-byte binary format (recommended)\n\
              TEXT storage: Human-readable string format\n\
              See: https://sqlite.org/datatype3.html#storage_classes_and_datatypes\n\
              Examples:\n  #[blob(primary, default_fn = uuid::Uuid::new_v4)] pub id: uuid::Uuid\n  #[text(default_fn = uuid::Uuid::new_v4)] pub uuid_text: uuid::Uuid",
            ),
        ];

        validations
            .iter()
            .find(|(condition, _)| *condition)
            .map_or(Ok(()), |(_, msg)| Err(Error::new_spanned(field_name, msg)))
    }
}

/// Build SQL column definition string
fn build_sql_definition(
    column_name: &str,
    column_type: &SQLiteType,
    is_primary_single: bool,
    is_not_null: bool,
    is_unique: bool,
    is_autoincrement: bool,
    default_value: &Option<Expr>,
) -> String {
    let mut sql = format!("{} {}", column_name, column_type.to_sql_type());

    // Handle primary key with potential autoincrement
    if is_primary_single {
        sql.push_str(" PRIMARY KEY");
        if is_autoincrement {
            sql.push_str(" AUTOINCREMENT");
        }
    }

    // Add NOT NULL constraint
    if is_not_null {
        sql.push_str(" NOT NULL");
    }

    // Add UNIQUE constraint
    if is_unique {
        sql.push_str(" UNIQUE");
    }

    if let Some(Expr::Lit(expr_lit)) = default_value {
        let default_val = match &expr_lit.lit {
            Lit::Int(i) => format!(" DEFAULT {i}"),
            Lit::Float(f) => format!(" DEFAULT {f}"),
            Lit::Bool(b) => format!(" DEFAULT {}", b.value() as i64),
            Lit::Str(s) => format!(" DEFAULT \"{}\"", s.value()),
            _ => String::new(),
        };
        sql.push_str(&default_val);
    }

    sql
}

/// Generate the appropriate type for select models
fn select_type(base_type: &Type, is_nullable: bool, has_default: bool) -> TokenStream {
    if !is_nullable || has_default {
        quote!(#base_type)
    } else {
        quote!(::std::option::Option<#base_type>)
    }
}

/// Generate the appropriate type for update models
fn update_type(base_type: &Type) -> TokenStream {
    quote!(::std::option::Option<#base_type>)
}

impl<'a> FieldInfo<'a> {
    /// Get the model field type for this field in the SelectModel
    pub(crate) fn get_select_type(&self) -> TokenStream {
        self.select_type
            .clone()
            .unwrap_or_else(|| select_type(self.base_type, self.is_nullable, self.has_default))
    }

    /// Get the model field type for this field in the UpdateModel
    pub(crate) fn get_update_type(&self) -> TokenStream {
        self.update_type
            .clone()
            .unwrap_or_else(|| update_type(self.base_type))
    }

    // =========================================================================
    // Type Category Methods - Centralized type classification
    // =========================================================================

    /// Get the category of this field's type for code generation decisions.
    ///
    /// This provides a single source of truth for type handling, eliminating
    /// scattered string matching throughout the codebase.
    pub(crate) fn type_category(&self) -> TypeCategory {
        // Special flags take precedence
        if self.is_json {
            return TypeCategory::Json;
        }
        if self.is_enum {
            return TypeCategory::Enum;
        }
        if self.is_uuid {
            return TypeCategory::Uuid;
        }

        // Detect from the base type string
        let type_str = self.base_type.to_token_stream().to_string();
        TypeCategory::from_type_string(&type_str)
    }

    /// Get the inner type for SQLiteInsertValue wrapper.
    ///
    /// For types that use `impl Into<...>` parameters, this returns the
    /// appropriate target type (e.g., String for text, Vec<u8> for blobs).
    pub(crate) fn insert_value_inner_type(&self) -> TokenStream {
        let base_type = self.base_type;

        match self.type_category() {
            TypeCategory::Uuid => {
                // UUID uses String for TEXT columns, Uuid for BLOB columns
                match self.column_type {
                    SQLiteType::Text => quote!(::std::string::String),
                    _ => quote!(::uuid::Uuid),
                }
            }
            TypeCategory::String => quote!(::std::string::String),
            TypeCategory::Blob => quote!(::std::vec::Vec<u8>),
            // ArrayString, ArrayVec, and primitives use the actual type
            _ => quote!(#base_type),
        }
    }

    /// Generate the full SQLiteInsertValue<...> type for this field.
    #[allow(dead_code)]
    pub(crate) fn sqlite_insert_value_type(&self) -> TokenStream {
        let inner = self.insert_value_inner_type();
        quote!(SQLiteInsertValue<'a, SQLiteValue<'a>, #inner>)
    }

    /// Generate an `impl Into<SQLiteInsertValue<...>>` parameter type for constructors.
    #[allow(dead_code)]
    pub(crate) fn insert_param_type(&self) -> TokenStream {
        let insert_value_type = self.sqlite_insert_value_type();
        quote!(impl Into<#insert_value_type>)
    }

    // =========================================================================
    // Schema Metadata Methods - For drizzle-kit compatible migrations
    // =========================================================================

    /// Convert default value expression to a JSON-compatible value
    fn default_to_json_value(&self) -> Option<serde_json::Value> {
        let Expr::Lit(expr_lit) = self.default_value.as_ref()? else {
            return None;
        };

        Some(match &expr_lit.lit {
            Lit::Int(i) => serde_json::Value::Number(
                i.base10_digits()
                    .parse::<i64>()
                    .ok()
                    .map(serde_json::Number::from)?,
            ),
            Lit::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(
                f.base10_digits().parse::<f64>().ok()?,
            )?),
            Lit::Bool(b) => serde_json::Value::Bool(b.value()),
            Lit::Str(s) => serde_json::Value::String(s.value()),
            _ => return None,
        })
    }

    /// Convert this field to a drizzle-schema Column type.
    ///
    /// Uses the actual schema types for type-safe construction,
    /// ensuring consistency with drizzle-kit format.
    pub(crate) fn to_column_meta(&self) -> drizzle_migrations::sqlite::Column {
        let mut col = drizzle_migrations::sqlite::Column::new(
            &self.column_name,
            self.column_type.to_sql_type().to_lowercase(),
        );

        if self.is_primary {
            col = col.primary_key();
        }
        if !self.is_nullable {
            col = col.not_null();
        }
        if self.is_autoincrement {
            col = col.autoincrement();
        }
        if let Some(default) = self.default_to_json_value() {
            col = col.default_value(default);
        }

        col
    }

    /// Convert this field to a drizzle-schema ForeignKey if it has a reference.
    pub(crate) fn to_foreign_key_meta(
        &self,
        table_name: &str,
    ) -> Option<drizzle_migrations::sqlite::ForeignKey> {
        let fk_ref = self.foreign_key.as_ref()?;

        let table_to = fk_ref.table_ident.to_string();
        let column_to = fk_ref.column_ident.to_string();
        let fk_name = format!(
            "{}_{}_{}_{}_fk",
            table_name, self.column_name, table_to, column_to
        );

        Some(drizzle_migrations::sqlite::ForeignKey {
            name: fk_name,
            table_from: table_name.to_string(),
            columns_from: vec![self.column_name.clone()],
            table_to,
            columns_to: vec![column_to],
            on_update: fk_ref.on_update.clone(),
            on_delete: fk_ref.on_delete.clone(),
        })
    }
}

// =============================================================================
// Table Metadata Generation - Uses drizzle-schema types
// =============================================================================

/// Generate the complete table metadata JSON for use in drizzle-kit compatible migrations.
///
/// Uses the actual drizzle-schema types for type-safe construction and serde serialization.
pub(crate) fn generate_table_meta_json(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
) -> String {
    let mut table = drizzle_migrations::sqlite::Table::new(table_name);

    // Add columns
    for field in field_infos {
        table.add_column(field.to_column_meta());
    }

    // Add foreign keys
    for field in field_infos {
        if let Some(fk) = field.to_foreign_key_meta(table_name) {
            table.add_foreign_key(fk);
        }
    }

    // Add composite primary key if applicable
    if is_composite_pk {
        let pk_columns: Vec<String> = field_infos
            .iter()
            .filter(|f| f.is_primary)
            .map(|f| f.column_name.clone())
            .collect();

        if pk_columns.len() > 1 {
            let pk_name = format!("{}_pk", table_name);
            let pk = drizzle_migrations::sqlite::CompositePK {
                name: Some(pk_name.clone()),
                columns: pk_columns,
            };
            table.composite_primary_keys.insert(pk_name, pk);
        }
    }

    serde_json::to_string(&table).unwrap_or_else(|_| "{}".to_string())
}

/// Check if a type is an Option<T>
pub(crate) fn is_option_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path)
        if type_path.path.segments.last()
            .is_some_and(|seg| seg.ident == "Option"))
}

/// Extract the inner type from Option<T>
pub(crate) fn extract_option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;

    if segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
    {
        Some(inner_type)
    } else {
        None
    }
}

/// Detect if an ExprPath is a foreign key reference (Table::column syntax)
/// Returns ForeignKeyReference with on_delete/on_update if the path matches the pattern
pub(crate) fn detect_foreign_key_reference_from_path(
    path: &ExprPath,
    on_delete: Option<String>,
    on_update: Option<String>,
) -> Option<ForeignKeyReference> {
    // Check if this is a path with exactly 2 segments (Table::column)
    if path.path.segments.len() == 2 {
        let table_ident = path.path.segments.first()?.ident.clone();
        let column_ident = path.path.segments.last()?.ident.clone();

        // Basic validation: ensure both segments exist and are valid identifiers
        if !table_ident.to_string().is_empty() && !column_ident.to_string().is_empty() {
            return Some(ForeignKeyReference {
                table_ident,
                column_ident,
                on_delete,
                on_update,
            });
        }
    } else {
        // Path doesn't match expected pattern
    }
    None
}
