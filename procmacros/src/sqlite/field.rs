use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use std::borrow::Cow;
use std::{collections::HashSet, fmt::Display};
use syn::{
    Attribute, Error, Expr, ExprPath, Field, Ident, Lit, Meta, Result, Token, Type, ext::IdentExt,
    parse::ParseStream, punctuated::Punctuated,
};

use crate::common::make_uppercase_path;
use crate::common::{
    is_option_type, option_inner_type, references_required_message, type_is_array_string,
    type_is_array_u8, type_is_arrayvec_u8, type_is_bool, type_is_byte_slice, type_is_datetime_tz,
    type_is_float, type_is_int, type_is_json_value, type_is_naive_date, type_is_naive_datetime,
    type_is_naive_time, type_is_offset_datetime, type_is_primitive_date_time, type_is_string_like,
    type_is_time_date, type_is_time_time, type_is_uuid, type_is_vec_u8, unwrap_option,
};

// =============================================================================
// Re-export shared types from drizzle-types
// =============================================================================

/// Re-export `TypeCategory` from the shared types crate
pub use drizzle_types::sqlite::TypeCategory;

/// Re-export `SQLiteType` from the shared types crate  
pub use drizzle_types::sqlite::SQLiteType as SharedSQLiteType;

// =============================================================================
// Local SQLiteType wrapper with procmacro-specific functionality
// =============================================================================

/// Local wrapper around `SharedSQLiteType` with additional procmacro-specific methods.
///
/// This contains methods that are only needed during macro expansion,
/// such as validation with detailed error messages.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum SQLiteType {
    Integer,
    Text,
    Blob,
    Real,
    Numeric,
    #[default]
    Any,
}

impl From<SharedSQLiteType> for SQLiteType {
    fn from(shared: SharedSQLiteType) -> Self {
        match shared {
            SharedSQLiteType::Integer => Self::Integer,
            SharedSQLiteType::Text => Self::Text,
            SharedSQLiteType::Blob => Self::Blob,
            SharedSQLiteType::Real => Self::Real,
            SharedSQLiteType::Numeric => Self::Numeric,
            SharedSQLiteType::Any => Self::Any,
        }
    }
}

impl SQLiteType {
    pub(crate) const fn to_shared(&self) -> SharedSQLiteType {
        match self {
            Self::Integer => SharedSQLiteType::Integer,
            Self::Text => SharedSQLiteType::Text,
            Self::Blob => SharedSQLiteType::Blob,
            Self::Real => SharedSQLiteType::Real,
            Self::Numeric => SharedSQLiteType::Numeric,
            Self::Any => SharedSQLiteType::Any,
        }
    }

    /// Convert from attribute name to enum variant
    pub(crate) fn from_attribute_name(name: &str) -> Option<Self> {
        SharedSQLiteType::from_attribute_name(name).map(Into::into)
    }

    /// Get the SQL type string for this type
    pub(crate) const fn to_sql_type(&self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Text => "TEXT",
            Self::Blob => "BLOB",
            Self::Real => "REAL",
            Self::Numeric => "NUMERIC",
            Self::Any => "ANY",
        }
    }

    pub(crate) const fn is_strict_allowed(&self) -> bool {
        self.to_shared().is_strict_allowed()
    }

    /// Check if a flag is valid for this column type
    pub(crate) fn is_valid_flag(&self, flag: &str) -> bool {
        matches!(flag, "primary" | "primary_key" | "unique")
            || matches!(
                (self, flag),
                (Self::Integer, "autoincrement")
                    | (Self::Text | Self::Blob, "json")
                    | (Self::Text | Self::Integer, "enum")
            )
    }
}

impl Display for SQLiteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql_type())
    }
}

// =============================================================================
// TypeCategory helper functions for procmacros
// =============================================================================

/// Convert a `TypeCategory` to the local `SQLiteType`
///
/// This wraps the shared type's method and converts to our local type
pub fn type_category_to_sqlite(cat: TypeCategory) -> Option<SQLiteType> {
    drizzle_types::sqlite::TypeCategory::to_sqlite_type(&cat).map(Into::into)
}

impl SQLiteType {
    /// Validate a flag for this column type, returning an error with `SQLite` docs link if invalid.
    ///
    /// Provides helpful error messages with links to relevant `SQLite` documentation
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
                     Use: #[column(integer, primary, autoincrement)]"
                }
                "json" => {
                    "JSON serialization is only supported for TEXT or BLOB column types.\n\
                     \n\
                     JSON data can be stored as TEXT (human-readable) or BLOB (binary). \
                     The choice affects storage size and query capabilities.\n\
                     \n\
                     See: https://sqlite.org/json1.html\n\
                     Use: #[column(json)] or #[column(blob, json)]"
                }
                "enum" => {
                    "Enum serialization is supported for TEXT (string) or INTEGER (discriminant) columns.\n\
                     \n\
                     - TEXT storage: stores variant names like 'Active', 'Inactive'\n\
                     - INTEGER storage: stores discriminant values like 0, 1, 2\n\
                     \n\
                     Use: #[column(enum)] or #[column(integer, enum)]"
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
pub struct ForeignKeyReference {
    /// The referenced table identifier (e.g., "User" from `User::id`)
    pub(crate) table_ident: Ident,
    /// The referenced column identifier (e.g., "id" from `User::id`)
    pub(crate) column_ident: Ident,
    /// ON DELETE action (e.g., "CASCADE", "SET NULL")
    pub(crate) on_delete: Option<String>,
    /// ON UPDATE action (e.g., "CASCADE", "SET NULL")
    pub(crate) on_update: Option<String>,
}

/// SQLite generated column metadata parsed from `#[column(generated(...))]`.
#[derive(Debug, Clone)]
pub struct GeneratedColumn {
    /// The raw SQL expression supplied by the user, without the generated-column
    /// wrapper.
    pub(crate) expression: String,
    /// `true` for STORED, `false` for VIRTUAL.
    pub(crate) stored: bool,
}

fn parenthesized_sql_expression(expression: &str) -> String {
    let trimmed = expression.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed.to_string()
    } else {
        format!("({trimmed})")
    }
}

fn normalize_default_sql(expression: &str) -> String {
    let trimmed = expression.trim();
    match trimmed.to_ascii_uppercase().as_str() {
        "CURRENT_TIME" | "CURRENT_DATE" | "CURRENT_TIMESTAMP" => trimmed.to_string(),
        _ => parenthesized_sql_expression(trimmed),
    }
}

/// Comprehensive field information for code generation.
///
/// The many `bool` fields here each represent an independent SQL column
/// property (primary, unique, nullable, JSON, enum, UUID, etc.) and are read
/// at disparate points during code-gen. Bundling them into bitflags would
/// obscure intent and bloat every access site.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
pub struct FieldInfo<'a> {
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
    pub(crate) is_autoincrement: bool,
    pub(crate) is_json: bool,
    pub(crate) is_enum: bool,
    pub(crate) is_uuid: bool,
    /// True when the type is unknown to the macro (e.g., a user-defined enum type).
    /// The type is validated at type-check time via `DrizzleSQLiteColumn` trait bounds.
    pub(crate) is_custom_type: bool,
    pub(crate) column_type: SQLiteType,

    // Foreign key support
    pub(crate) foreign_key: Option<ForeignKeyReference>,

    /// Resolved primary-key / unique state.
    ///
    /// Set in `from_field` once the table-level `is_composite_pk` decision
    /// is known. This is the *sole* source of truth for primary-key and
    /// unique state — use `is_primary()` / `is_unique()` /
    /// `is_inline_primary()` / `is_inline_unique()` to query, never inspect
    /// the enum variants directly outside this module.
    pub(crate) constraint: crate::common::Constraint,

    /// Optional SQLite collation name from `#[column(collate = NOCASE)]`.
    /// Stored as the uppercased / literal collation name; emitted verbatim
    /// in DDL after column constraints (`... NOT NULL COLLATE NOCASE`).
    pub(crate) collate: Option<String>,

    // Attribute values
    pub(crate) default_value: Option<Expr>,
    pub(crate) default_sql: Option<String>,
    pub(crate) default_fn: Option<Expr>,
    pub(crate) generated_column: Option<GeneratedColumn>,
    pub(crate) check_constraint: Option<String>,

    // Original marker expressions for IDE hover documentation
    // These preserve the original tokens so rust-analyzer can resolve them
    pub(crate) marker_exprs: Vec<syn::ExprPath>,

    // Type representations for models
    pub(crate) select_type: Option<TokenStream>,
    #[allow(dead_code)]
    pub(crate) update_type: Option<TokenStream>,
}

/// Parse attribute items, handling reserved keywords like 'enum'
fn parse_item(input: ParseStream) -> Result<Expr> {
    if input.peek(Ident) {
        let fork = input.fork();
        let ident = Ident::parse_any(&fork)?;
        if ident.to_string().eq_ignore_ascii_case("generated") && fork.peek(syn::token::Paren) {
            let generated_ident = Ident::parse_any(input)?;
            let content;
            let paren_token = syn::parenthesized!(content in input);
            let kind_ident = Ident::parse_any(&content)?;
            content.parse::<Token![,]>()?;
            let expr_lit: syn::LitStr = content.parse()?;
            if !content.is_empty() {
                return Err(Error::new(
                    content.span(),
                    "#[column(generated(...))] accepts exactly two arguments",
                ));
            }

            let mut args = Punctuated::new();
            args.push_value(Expr::Path(syn::ExprPath {
                attrs: Vec::new(),
                qself: None,
                path: syn::Path::from(kind_ident),
            }));
            args.push_punct(Default::default());
            args.push_value(Expr::Lit(syn::ExprLit {
                attrs: Vec::new(),
                lit: Lit::Str(expr_lit),
            }));

            return Ok(Expr::Call(syn::ExprCall {
                attrs: Vec::new(),
                func: Box::new(Expr::Path(syn::ExprPath {
                    attrs: Vec::new(),
                    qself: None,
                    path: syn::Path::from(generated_ident),
                })),
                paren_token,
                args,
            }));
        }
    }

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
    default_sql: Option<String>,
    default_fn: Option<Expr>,
    generated_column: Option<GeneratedColumn>,
    check_constraint: Option<String>,
    references: Option<Expr>,
    on_delete: Option<String>,
    on_update: Option<String>,
    name: Option<Expr>,
    /// SQLite collation name from `collate = "NOCASE"` (or other built-in /
    /// custom registered collation). Stored as the literal name; emitted
    /// verbatim in DDL as `COLLATE <name>`.
    collate: Option<String>,
    flags: HashSet<String>,
    /// Original marker expressions for IDE hover documentation
    /// These preserve the original tokens so rust-analyzer can resolve them
    marker_exprs: Vec<syn::ExprPath>,
    /// Explicit `SQLite` type override (e.g., from #[column(text)] or #[column(blob)])
    explicit_type: Option<SQLiteType>,
}

#[derive(Default)]
struct AttributeData {
    column_type: SQLiteType,
    /// Whether the type was explicitly specified (vs inferred from Rust type)
    has_explicit_type: bool,
    flags: HashSet<String>,
    default_value: Option<Expr>,
    default_sql: Option<String>,
    default_fn: Option<Expr>,
    generated_column: Option<GeneratedColumn>,
    check_constraint: Option<String>,
    references_path: Option<ExprPath>,
    on_delete: Option<String>,
    on_update: Option<String>,
    attr_name: Option<String>,
    /// SQLite collation name. See [`ParsedArgs::collate`].
    collate: Option<String>,
    /// Original marker expressions for IDE hover documentation
    marker_exprs: Vec<syn::ExprPath>,
}

impl<'a> FieldInfo<'a> {
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
                    "invalid referential action `{action_str}`; expected one of: CASCADE, SET_NULL, SET_DEFAULT, RESTRICT, NO_ACTION"
                ),
            )),
        }
    }

    /// Parse attribute arguments, extracting flags and named parameters.
    ///
    /// Supports:
    /// - `SQLite` type overrides: `text`, `integer`, `blob`, `real`, `any`
    /// - Constraint flags: `primary`, `unique`, `autoincrement`, `json`, `enum`
    /// - Named parameters: `default = value`, `default_fn = func`, `references = Table::col`
    fn parse_args(input: ParseStream) -> Result<ParsedArgs> {
        if input.is_empty() {
            return Ok(ParsedArgs::default());
        }

        let items = input.parse_terminated(parse_item, Token![,])?;
        let mut args = ParsedArgs::default();

        for expr in items {
            match expr {
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
                                args.marker_exprs.push(make_uppercase_path(ident, "JSON"));
                            }
                            "JSONB" => {
                                // JSONB = BLOB storage with JSON serialization
                                args.explicit_type = Some(SQLiteType::Blob);
                                args.flags.insert("json".to_string());
                                args.marker_exprs.push(make_uppercase_path(ident, "JSONB"));
                            }
                            "DEFAULT" => {
                                args.default_fn = Some(syn::parse_quote!(Default::default));
                            }
                            "ENUM" => {
                                args.flags.insert("enum".to_string());
                                args.marker_exprs.push(make_uppercase_path(ident, "ENUM"));
                            }
                            "PRIMARY" | "PRIMARY_KEY" => {
                                args.flags.insert("primary".to_string());
                                args.marker_exprs
                                    .push(make_uppercase_path(ident, "PRIMARY"));
                            }
                            "AUTOINCREMENT" => {
                                args.flags.insert("autoincrement".to_string());
                                args.marker_exprs
                                    .push(make_uppercase_path(ident, "AUTOINCREMENT"));
                            }
                            "UNIQUE" => {
                                args.flags.insert("unique".to_string());
                                args.marker_exprs.push(make_uppercase_path(ident, "UNIQUE"));
                            }
                            _ => {
                                // Check if this is a SQLite type override (case-insensitive for types)
                                if let Some(sqlite_type) =
                                    SQLiteType::from_attribute_name(&ident_str)
                                {
                                    args.explicit_type = Some(sqlite_type);
                                } else {
                                    args.flags.insert(ident_str);
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
                                    .push(make_uppercase_path(param, "DEFAULT"));
                            }
                            "DEFAULT_SQL" => {
                                if let Expr::Lit(syn::ExprLit {
                                    lit: Lit::Str(lit_str),
                                    ..
                                }) = &*assign.right
                                {
                                    args.default_sql =
                                        Some(normalize_default_sql(&lit_str.value()));
                                    args.marker_exprs
                                        .push(make_uppercase_path(param, "DEFAULT_SQL"));
                                } else {
                                    return Err(Error::new_spanned(
                                        &assign.right,
                                        "default_sql requires a string literal, e.g. default_sql = \"CURRENT_TIMESTAMP\"",
                                    ));
                                }
                            }
                            "DEFAULT_FN" => {
                                args.default_fn = Some(*assign.right);
                                args.marker_exprs
                                    .push(make_uppercase_path(param, "DEFAULT_FN"));
                            }
                            "REFERENCES" => {
                                args.references = Some(*assign.right.clone());
                                args.marker_exprs
                                    .push(make_uppercase_path(param, "REFERENCES"));
                            }
                            "ON_DELETE" => {
                                if let Expr::Path(action_path) = &*assign.right
                                    && let Some(action_ident) = action_path.path.get_ident()
                                {
                                    let action_upper =
                                        action_ident.to_string().to_ascii_uppercase();
                                    args.on_delete =
                                        Self::validate_referential_action(action_ident).ok();
                                    args.marker_exprs
                                        .push(make_uppercase_path(param, "ON_DELETE"));
                                    // Add marker for the action value (CASCADE, SET_NULL, etc.)
                                    args.marker_exprs
                                        .push(make_uppercase_path(action_ident, &action_upper));
                                }
                            }
                            "ON_UPDATE" => {
                                if let Expr::Path(action_path) = &*assign.right
                                    && let Some(action_ident) = action_path.path.get_ident()
                                {
                                    let action_upper =
                                        action_ident.to_string().to_ascii_uppercase();
                                    args.on_update =
                                        Self::validate_referential_action(action_ident).ok();
                                    args.marker_exprs
                                        .push(make_uppercase_path(param, "ON_UPDATE"));
                                    // Add marker for the action value (CASCADE, SET_NULL, etc.)
                                    args.marker_exprs
                                        .push(make_uppercase_path(action_ident, &action_upper));
                                }
                            }
                            "NAME" => {
                                args.name = Some(*assign.right.clone());
                                args.marker_exprs.push(make_uppercase_path(param, "NAME"));
                            }
                            "COLLATE" => {
                                // Accept either a string literal (`collate = "NOCASE"`) or
                                // a bare ident (`collate = NOCASE`). The ident form keeps
                                // attribute syntax consistent with the rest of `#[column(...)]`
                                // and gets an IDE marker for hover.
                                match &*assign.right {
                                    Expr::Lit(syn::ExprLit {
                                        lit: Lit::Str(lit_str),
                                        ..
                                    }) => {
                                        args.collate = Some(lit_str.value());
                                    }
                                    Expr::Path(path) => {
                                        if let Some(ident) = path.path.get_ident() {
                                            let upper = ident.to_string().to_ascii_uppercase();
                                            args.collate = Some(upper.clone());
                                            args.marker_exprs
                                                .push(make_uppercase_path(ident, &upper));
                                        }
                                    }
                                    _ => {}
                                }
                                args.marker_exprs
                                    .push(make_uppercase_path(param, "COLLATE"));
                            }
                            "CHECK" => {
                                if let Expr::Lit(syn::ExprLit {
                                    lit: Lit::Str(lit_str),
                                    ..
                                }) = &*assign.right
                                {
                                    args.check_constraint = Some(lit_str.value());
                                    args.marker_exprs.push(make_uppercase_path(param, "CHECK"));
                                } else {
                                    return Err(Error::new_spanned(
                                        &assign.right,
                                        "CHECK requires a string literal, e.g. CHECK = \"score >= 0\"",
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Expr::Call(call) => {
                    if let Expr::Path(path) = &*call.func
                        && let Some(ident) = path.path.get_ident()
                        && ident.to_string().eq_ignore_ascii_case("generated")
                    {
                        let mut args_iter = call.args.iter();
                        let Some(kind_expr) = args_iter.next() else {
                            return Err(Error::new_spanned(
                                call,
                                "#[column(generated(...))] requires arguments: generated(stored, \"expression\") or generated(virtual, \"expression\")",
                            ));
                        };
                        let stored = match kind_expr {
                            Expr::Path(kind_path) => {
                                let Some(kind_ident) = kind_path.path.get_ident() else {
                                    return Err(Error::new_spanned(
                                        kind_expr,
                                        "expected `stored` or `virtual` for #[column(generated(...))]",
                                    ));
                                };
                                match kind_ident.to_string().to_ascii_lowercase().as_str() {
                                    "stored" => true,
                                    "virtual" => false,
                                    _ => {
                                        return Err(Error::new_spanned(
                                            kind_ident,
                                            "expected `stored` or `virtual` for #[column(generated(...))]",
                                        ));
                                    }
                                }
                            }
                            _ => {
                                return Err(Error::new_spanned(
                                    kind_expr,
                                    "expected `stored` or `virtual` for #[column(generated(...))]",
                                ));
                            }
                        };

                        let Some(expr_arg) = args_iter.next() else {
                            return Err(Error::new_spanned(
                                call,
                                "expected a string literal for the SQL expression, e.g. generated(stored, \"col_a + col_b\")",
                            ));
                        };
                        let Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(expr_lit),
                            ..
                        }) = expr_arg
                        else {
                            return Err(Error::new_spanned(
                                expr_arg,
                                "expected a string literal for the SQL expression, e.g. generated(stored, \"col_a + col_b\")",
                            ));
                        };
                        if let Some(extra) = args_iter.next() {
                            return Err(Error::new_spanned(
                                extra,
                                "#[column(generated(...))] accepts exactly two arguments",
                            ));
                        }

                        args.generated_column = Some(GeneratedColumn {
                            expression: expr_lit.value(),
                            stored,
                        });
                        args.marker_exprs
                            .push(make_uppercase_path(ident, "GENERATED"));
                    }
                }
                _ => {}
            }
        }

        Ok(args)
    }

    /// Parse field information from a struct field
    pub(crate) fn from_field(field: &'a Field, is_part_of_composite_pk: bool) -> Result<Self> {
        let Some(field_name) = &field.ident else {
            return Err(Error::new_spanned(
                field,
                "all struct fields must have names; tuple structs are not supported.\n\
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
    /// For the new syntax, if no explicit type is provided, the `SQLite` type is
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
                let args = attr.parse_args_with(Self::parse_args)?;
                // Validate flags against the explicit column type
                args.flags
                    .iter()
                    .try_for_each(|flag| column_type.validate_flag(flag, attr))?;

                data.flags.extend(args.flags);
                data.default_value = data.default_value.or(args.default_value);
                data.default_sql = data.default_sql.or(args.default_sql);
                data.default_fn = data.default_fn.or(args.default_fn);
                data.generated_column = data.generated_column.or(args.generated_column);
                data.check_constraint = data.check_constraint.or(args.check_constraint);
                data.marker_exprs.extend(args.marker_exprs);
                data.on_delete = data.on_delete.or(args.on_delete);
                data.on_update = data.on_update.or(args.on_update);
                data.collate = data.collate.or(args.collate);

                if let Some(Expr::Path(path)) = args.references {
                    data.references_path = Some(path);
                }

                if let Some(Expr::Lit(expr_lit)) = args.name
                    && let Lit::Str(lit_str) = expr_lit.lit
                {
                    data.attr_name = Some(lit_str.value());
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
                let args = attr.parse_args_with(Self::parse_args)?;
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
                data.default_sql = data.default_sql.or(args.default_sql);
                data.default_fn = data.default_fn.or(args.default_fn);
                data.generated_column = data.generated_column.or(args.generated_column);
                data.check_constraint = data.check_constraint.or(args.check_constraint);
                data.marker_exprs.extend(args.marker_exprs);
                data.on_delete = data.on_delete.or(args.on_delete);
                data.on_update = data.on_update.or(args.on_update);
                data.collate = data.collate.or(args.collate);

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

        // Validate: on_delete and on_update require references
        if (data.on_delete.is_some() || data.on_update.is_some()) && data.references_path.is_none()
        {
            let msg =
                references_required_message(data.on_delete.is_some(), data.on_update.is_some());
            // Use the first marker as span source for the error
            if let Some(marker) = data.marker_exprs.first() {
                return Err(Error::new_spanned(marker, msg));
            }
            return Err(Error::new(proc_macro2::Span::call_site(), msg));
        }

        Ok(data)
    }

    /// Build `FieldInfo` from parsed components
    fn build(
        field_name: &'a Ident,
        field_type: &'a Type,
        attrs: AttributeData,
        is_part_of_composite_pk: bool,
    ) -> Result<Self> {
        let column_name = attrs
            .attr_name
            .clone()
            .unwrap_or_else(|| field_name.to_string().to_snake_case());
        let is_nullable = is_option_type(field_type);
        let base_type = option_inner_type(field_type).unwrap_or(field_type);

        // Constraint flags lifted straight from user attributes / Rust type.
        let is_primary = attrs.flags.contains("primary_key") || attrs.flags.contains("primary");
        let is_autoincrement = attrs.flags.contains("autoincrement");
        let is_unique = attrs.flags.contains("unique");
        let is_json = attrs.flags.contains("json");
        let is_enum = attrs.flags.contains("enum");
        let is_uuid = type_is_uuid(base_type);
        let has_default = attrs.default_value.is_some()
            || attrs.default_sql.is_some()
            || attrs.default_fn.is_some();

        // Determine the SQLite type:
        // 1. Use explicit type from attribute if provided
        // 2. Otherwise, infer from Rust type (hard error if unsupported)
        let type_category = if is_json {
            TypeCategory::Json
        } else if is_enum {
            TypeCategory::Enum
        } else {
            type_category_from_type(base_type)
        };

        // Track whether this is a custom type (unknown to the macro, validated via trait bounds).
        // This remains true even when a user writes an explicit storage marker like
        // #[column(blob)] so the read/write path still uses DrizzleSQLiteColumn.
        let mut is_custom_type = matches!(type_category, TypeCategory::Unknown);

        let column_type = if attrs.has_explicit_type {
            // Use the explicit type from the attribute
            attrs.column_type.clone()
        } else if let Some(sqlite_type) = type_category_to_sqlite(type_category) {
            // Infer from Rust type
            sqlite_type
        } else {
            // Unknown type — deferred to DrizzleSQLiteColumn trait at type-check time.
            // Use Any as placeholder; real SQL_TYPE comes from the trait const.
            is_custom_type = true;
            SQLiteType::Any
        };

        Self::validate_constraints(
            &column_type,
            ConstraintFlags {
                is_primary,
                is_autoincrement,
                is_uuid,
            },
            attrs.default_value.as_ref(),
            attrs.default_sql.as_deref(),
            attrs.default_fn.as_ref(),
            attrs.generated_column.as_ref(),
            field_name,
        )?;

        let sql_definition = build_sql_definition(
            &column_name,
            &column_type,
            SqlDefinitionFlags {
                is_primary_single: is_primary && !is_part_of_composite_pk,
                is_not_null: !is_nullable,
                is_unique,
                is_autoincrement,
            },
            attrs.default_value.as_ref(),
            attrs.default_sql.as_deref(),
            attrs.generated_column.as_ref(),
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
            has_default,
            is_autoincrement,
            is_json,
            is_enum,
            is_uuid,
            is_custom_type,
            column_type,
            foreign_key,
            constraint: crate::common::Constraint::from_flags(
                is_primary,
                is_unique,
                is_part_of_composite_pk,
            ),
            collate: attrs.collate,
            default_value: attrs.default_value,
            default_sql: attrs.default_sql,
            default_fn: attrs.default_fn,
            generated_column: attrs.generated_column,
            check_constraint: attrs.check_constraint,
            marker_exprs: attrs.marker_exprs,
            select_type: Some(select_type(base_type, is_nullable, has_default)),
            update_type: Some(update_type(base_type)),
        })
    }

    /// Validate field constraints and configuration
    fn validate_constraints(
        column_type: &SQLiteType,
        props: ConstraintFlags,
        default_value: Option<&Expr>,
        default_sql: Option<&str>,
        default_fn: Option<&Expr>,
        generated_column: Option<&GeneratedColumn>,
        field_name: &Ident,
    ) -> Result<()> {
        let validations = [
            (
                props.is_autoincrement && !matches!(column_type, SQLiteType::Integer),
                "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY.\n\
              See: https://sqlite.org/autoinc.html\n\
              Hint: Change column type to '#[column(integer, primary, autoincrement)]'",
            ),
            (
                props.is_autoincrement && !props.is_primary,
                "AUTOINCREMENT requires PRIMARY KEY constraint.\n\
              See: https://sqlite.org/autoinc.html\n\
              Hint: Add 'primary' flag: '#[column(primary, autoincrement)]'",
            ),
            (
                default_value.is_some() && default_fn.is_some(),
                "Cannot specify both 'default' (compile-time literal) and 'default_fn' (runtime function).\n\
              Choose one: either 'default = literal' or 'default_fn = function'\n\
              Examples:\n  #[column(default = \"hello\")] for compile-time defaults\n  #[column(default_fn = String::new)] for runtime defaults",
            ),
            (
                default_value.is_some() && default_sql.is_some(),
                "Cannot specify both 'default' (literal default) and 'default_sql' (raw SQL default).\n\
              Choose one: either 'default = literal' or 'default_sql = \"CURRENT_TIMESTAMP\"'",
            ),
            (
                default_sql.is_some() && generated_column.is_some(),
                "Cannot specify both 'default_sql' and 'generated' on the same column.\n\
              SQLite generated columns cannot also declare a DEFAULT expression.",
            ),
            (
                props.is_uuid && !matches!(column_type, SQLiteType::Blob | SQLiteType::Text),
                "UUID fields must use either BLOB or TEXT column type.\n\
              BLOB storage: Efficient 16-byte binary format (recommended)\n\
              TEXT storage: Human-readable string format\n\
              See: https://sqlite.org/datatype3.html#storage_classes_and_datatypes\n\
              Examples:\n  #[column(blob, primary, default_fn = uuid::Uuid::new_v4)] pub id: uuid::Uuid\n  #[column(text, default_fn = uuid::Uuid::new_v4)] pub uuid_text: uuid::Uuid",
            ),
        ];

        validations
            .iter()
            .find(|(condition, _)| *condition)
            .map_or(Ok(()), |(_, msg)| Err(Error::new_spanned(field_name, msg)))
    }
}

/// Flags consumed by [`FieldInfo::validate_constraints`]. These are the
/// subset of column flags that participate in validation rules (e.g.
/// AUTOINCREMENT requires INTEGER PRIMARY KEY; UUID requires BLOB/TEXT).
#[derive(Debug, Clone, Copy)]
struct ConstraintFlags {
    is_primary: bool,
    is_autoincrement: bool,
    is_uuid: bool,
}

/// Flags controlling SQL column definition output. Each flag corresponds to
/// an independent piece of the emitted column definition (PRIMARY KEY,
/// NOT NULL, UNIQUE, AUTOINCREMENT) and is toggled independently.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
struct SqlDefinitionFlags {
    is_primary_single: bool,
    is_not_null: bool,
    is_unique: bool,
    is_autoincrement: bool,
}

/// Build SQL column definition string
fn build_sql_definition(
    column_name: &str,
    column_type: &SQLiteType,
    flags: SqlDefinitionFlags,
    default_value: Option<&Expr>,
    default_sql: Option<&str>,
    generated_column: Option<&GeneratedColumn>,
) -> String {
    let mut sql = format!("\"{}\" {}", column_name, column_type.to_sql_type());

    // Handle primary key with potential autoincrement
    if flags.is_primary_single {
        sql.push_str(" PRIMARY KEY");
        if flags.is_autoincrement {
            sql.push_str(" AUTOINCREMENT");
        }
    }

    // Add NOT NULL constraint
    if flags.is_not_null {
        sql.push_str(" NOT NULL");
    }

    // Add UNIQUE constraint
    if flags.is_unique {
        sql.push_str(" UNIQUE");
    }

    if let Some(generated) = generated_column {
        let generated_type = if generated.stored {
            "STORED"
        } else {
            "VIRTUAL"
        };
        let generated_expr = parenthesized_sql_expression(&generated.expression);
        sql.push_str(&format!(
            " GENERATED ALWAYS AS {generated_expr} {generated_type}"
        ));
    }

    if generated_column.is_none()
        && let Some(Expr::Lit(expr_lit)) = default_value
    {
        let default_val = match &expr_lit.lit {
            Lit::Int(i) => format!(" DEFAULT {i}"),
            Lit::Float(f) => format!(" DEFAULT {f}"),
            Lit::Bool(b) => format!(" DEFAULT {}", i64::from(b.value())),
            Lit::Str(s) => {
                let escaped = s.value().replace('\'', "''");
                format!(" DEFAULT '{escaped}'")
            }
            _ => String::new(),
        };
        sql.push_str(&default_val);
    }
    if generated_column.is_none()
        && let Some(default_sql) = default_sql
    {
        sql.push_str(&format!(" DEFAULT {default_sql}"));
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
    let sqlite_update_value = crate::paths::sqlite::sqlite_update_value();
    let sqlite_value = crate::paths::sqlite::sqlite_value();
    quote!(#sqlite_update_value<'a, #sqlite_value<'a>, #base_type>)
}

impl FieldInfo<'_> {
    /// Get the model field type for this field in the `SelectModel`
    pub(crate) fn get_select_type(&self) -> TokenStream {
        self.select_type
            .clone()
            .unwrap_or_else(|| select_type(self.base_type, self.is_nullable, self.has_default))
    }

    /// Get the model field type for this field in the `UpdateModel`
    #[allow(dead_code)]
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
        if self.is_enum || self.is_custom_type {
            return TypeCategory::Enum;
        }
        if self.is_uuid {
            return TypeCategory::Uuid;
        }

        type_category_from_type(self.base_type)
    }

    /// Get the inner type for `SQLiteInsertValue` wrapper.
    ///
    /// For types that use `impl Into<...>` parameters, this returns the
    /// appropriate target type (e.g., String for text, Vec<u8> for blobs).
    pub(crate) fn insert_value_inner_type(&self) -> TokenStream {
        let base_type = self.base_type;

        match self.type_category() {
            TypeCategory::Uuid => {
                // UUID uses String for TEXT columns, Uuid for BLOB columns
                if self.column_type == SQLiteType::Text {
                    quote!(::std::string::String)
                } else {
                    quote!(::uuid::Uuid)
                }
            }
            TypeCategory::String => quote!(::std::string::String),
            TypeCategory::Blob => quote!(::std::vec::Vec<u8>),
            // ArrayString, ArrayVec, and primitives use the actual type
            _ => quote!(#base_type),
        }
    }

    /// Generate the full `SQLiteInsertValue`<...> type for this field.
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

    /// SQL type string expression for generated schema metadata.
    ///
    /// Built-in columns use a literal. Custom columns use the associated const
    /// from `DrizzleSQLiteColumn`, making the trait the source of truth.
    pub(crate) fn sql_type_expr(&self) -> TokenStream {
        if self.is_custom_type {
            let base_type = self.base_type;
            let drizzle_sqlite_column = crate::paths::sqlite::drizzle_sqlite_column();
            quote!(<#base_type as #drizzle_sqlite_column>::SQL_TYPE)
        } else {
            let sql_type = self.column_type.to_sql_type();
            quote!(#sql_type)
        }
    }

    /// Drizzle SQL type marker used by expression generation.
    pub(crate) fn sql_type_marker(&self) -> TokenStream {
        if self.is_custom_type {
            let base_type = self.base_type;
            let drizzle_sqlite_column = crate::paths::sqlite::drizzle_sqlite_column();
            quote!(<#base_type as #drizzle_sqlite_column>::SQLType)
        } else {
            crate::common::sqlite_column_type_to_sql_type(&self.column_type)
        }
    }

    /// Column SQL definition expression for `SQLSchema::SQL`.
    pub(crate) fn sql_definition_expr(&self) -> TokenStream {
        if !self.is_custom_type {
            let sql_definition = &self.sql_definition;
            return quote!(#sql_definition);
        }

        let const_format = crate::common::paths::const_format();
        let sql_type = self.sql_type_expr();
        let prefix = format!("\"{}\" ", self.column_name);
        let placeholder = format!(
            "\"{}\" {}",
            self.column_name,
            self.column_type.to_sql_type()
        );
        let suffix = self
            .sql_definition
            .strip_prefix(&placeholder)
            .unwrap_or_default();

        quote!(#const_format::concatcp!(#prefix, #sql_type, #suffix))
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
    pub(crate) fn to_column_meta(&self, table_name: &str) -> drizzle_types::sqlite::ddl::Column {
        let mut col = drizzle_types::sqlite::ddl::Column::new(
            table_name.to_string(),
            self.column_name.clone(),
            self.column_type.to_sql_type().to_lowercase(),
        );

        // Note: primary_key is handled via PrimaryKey entity, not a column field
        if !self.is_nullable {
            col = col.not_null();
        }
        if self.is_autoincrement {
            col = col.autoincrement();
        }
        if let Some(default_sql) = &self.default_sql {
            col = col.default_value(default_sql.clone());
        } else if let Some(default) = self.default_to_json_value() {
            // Convert serde_json::Value to String for DDL storage
            let default_str = match &default {
                serde_json::Value::String(s) => s.clone(),
                other => serde_json::to_string(other).unwrap_or_default(),
            };
            col = col.default_value(default_str);
        }
        if let Some(generated) = &self.generated_column {
            col.generated = Some(drizzle_types::sqlite::ddl::Generated {
                expression: Cow::Owned(parenthesized_sql_expression(&generated.expression)),
                gen_type: if generated.stored {
                    drizzle_types::sqlite::ddl::GeneratedType::Stored
                } else {
                    drizzle_types::sqlite::ddl::GeneratedType::Virtual
                },
            });
        }
        if let Some(collate) = &self.collate {
            col.collate = Some(Cow::Owned(collate.clone()));
        }

        col
    }

    /// Convert this field to a drizzle-schema `ForeignKey` if it has a reference.
    pub(crate) fn to_foreign_key_meta(
        &self,
        table_name: &str,
    ) -> Option<drizzle_types::sqlite::ddl::ForeignKey> {
        let fk_ref = self.foreign_key.as_ref()?;

        let table_to = fk_ref.table_ident.to_string().to_snake_case();
        let target_column = fk_ref.column_ident.to_string().to_snake_case();
        let fk_name = format!(
            "fk_{}_{}_{}_{}_fk",
            table_name, self.column_name, table_to, target_column
        );

        // Convert Vec<String> to Cow<'static, [Cow<'static, str>]>
        let columns: Vec<Cow<'static, str>> = vec![Cow::Owned(self.column_name.clone())];
        let columns_to: Vec<Cow<'static, str>> = vec![Cow::Owned(target_column)];

        let fk = drizzle_types::sqlite::ddl::ForeignKey {
            table: Cow::Owned(table_name.to_string()),
            name: Cow::Owned(fk_name),
            name_explicit: false,
            columns: Cow::Owned(columns),
            table_to: Cow::Owned(table_to),
            columns_to: Cow::Owned(columns_to),
            on_update: fk_ref.on_update.clone().map(Cow::Owned),
            on_delete: fk_ref.on_delete.clone().map(Cow::Owned),
        };

        Some(fk)
    }
}

// =============================================================================
// Table Metadata Generation - Uses drizzle-schema types
// =============================================================================

/// Generate the complete table metadata JSON for use in drizzle-kit compatible migrations.
///
/// Uses the actual drizzle-schema types for type-safe construction and serde serialization.
pub fn generate_table_meta_json(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
) -> String {
    use drizzle_types::sqlite::ddl::{PrimaryKey, SqliteEntity, Table};

    // Collect all entities
    let mut entities: Vec<SqliteEntity> = Vec::new();

    // Add Table entity
    entities.push(SqliteEntity::Table(Table::new(table_name.to_string())));

    // Add columns
    for field in field_infos {
        entities.push(SqliteEntity::Column(field.to_column_meta(table_name)));
    }

    // Add foreign keys
    for field in field_infos {
        if let Some(fk) = field.to_foreign_key_meta(table_name) {
            entities.push(SqliteEntity::ForeignKey(fk));
        }
    }

    // Add composite primary key if applicable
    if is_composite_pk {
        let pk_columns: Vec<String> = field_infos
            .iter()
            .filter(|f| f.is_primary())
            .map(|f| f.column_name.clone())
            .collect();

        if pk_columns.len() > 1 {
            let pk_name = format!("{table_name}_pk");
            let pk = PrimaryKey::from_strings(table_name.to_string(), pk_name, pk_columns);
            entities.push(SqliteEntity::PrimaryKey(pk));
        }
    }

    serde_json::to_string(&entities).unwrap_or_else(|_| "[]".to_string())
}

fn type_category_from_type(ty: &Type) -> TypeCategory {
    let ty = unwrap_option(ty);

    if type_is_array_u8(ty) {
        return TypeCategory::ByteArray;
    }
    if type_is_array_string(ty) {
        return TypeCategory::ArrayString;
    }
    if type_is_arrayvec_u8(ty) {
        return TypeCategory::ArrayVec;
    }
    if type_is_uuid(ty) {
        return TypeCategory::Uuid;
    }
    if type_is_json_value(ty) {
        return TypeCategory::Json;
    }
    if type_is_naive_date(ty)
        || type_is_naive_time(ty)
        || type_is_naive_datetime(ty)
        || type_is_datetime_tz(ty)
        || type_is_time_date(ty)
        || type_is_time_time(ty)
        || type_is_primitive_date_time(ty)
        || type_is_offset_datetime(ty)
    {
        return TypeCategory::DateTime;
    }
    if type_is_string_like(ty) {
        return TypeCategory::String;
    }
    if type_is_vec_u8(ty) || type_is_byte_slice(ty) {
        return TypeCategory::Blob;
    }
    if type_is_bool(ty) {
        return TypeCategory::Bool;
    }
    if type_is_int(ty, "i8")
        || type_is_int(ty, "i16")
        || type_is_int(ty, "i32")
        || type_is_int(ty, "i64")
        || type_is_int(ty, "u8")
        || type_is_int(ty, "u16")
        || type_is_int(ty, "u32")
        || type_is_int(ty, "isize")
        || type_is_int(ty, "usize")
    {
        return TypeCategory::Integer;
    }
    if type_is_float(ty, "f32") || type_is_float(ty, "f64") {
        return TypeCategory::Real;
    }

    TypeCategory::Unknown
}

/// Detect if an `ExprPath` is a foreign key reference (`Table::column` syntax)
/// Returns `ForeignKeyReference` with `on_delete/on_update` if the path matches the pattern
pub fn detect_foreign_key_reference_from_path(
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

/// Inherent accessors that delegate to the `Constraint` enum so callers
/// don't have to know the enum's variants. `is_primary()` covers both
/// standalone and composite primary keys; `is_unique()` is the "user
/// wrote `UNIQUE`" predicate — which, by `Constraint::from_flags`, is
/// only true when the column is *not* also a primary key (primary key
/// implies uniqueness, and an explicit `unique` flag on a PK column is
/// degenerate). `is_inline_primary()` / `is_inline_unique()` answer the
/// finer "should this constraint be inlined in the column definition?"
/// question used by DDL emitters.
impl FieldInfo<'_> {
    #[inline]
    pub(crate) fn is_primary(&self) -> bool {
        self.constraint.is_primary()
    }

    #[inline]
    pub(crate) fn is_unique(&self) -> bool {
        self.constraint.is_inline_unique()
    }
}

// Trait impls for shared constraint generation

impl crate::common::constraints::ForeignKeyRef for ForeignKeyReference {
    fn ref_table(&self) -> &Ident {
        &self.table_ident
    }
    fn ref_column(&self) -> &Ident {
        &self.column_ident
    }
}

impl crate::common::constraints::ConstraintFieldInfo for FieldInfo<'_> {
    type ForeignKey = ForeignKeyReference;

    fn ident(&self) -> &Ident {
        self.ident
    }
    fn column_name(&self) -> &str {
        &self.column_name
    }
    fn is_primary(&self) -> bool {
        Self::is_primary(self)
    }
    fn is_unique(&self) -> bool {
        Self::is_unique(self)
    }
    fn foreign_key(&self) -> Option<&ForeignKeyReference> {
        self.foreign_key.as_ref()
    }
}
