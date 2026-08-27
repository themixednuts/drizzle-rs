//! Attribute interpretation with macro-equivalent semantics.
//!
//! Every function in this module mirrors the matching parser in
//! `drizzle-macros` (`procmacros/src/sqlite/field.rs`, `table/attributes.rs`,
//! `index.rs`, `view.rs`, and the postgres twins). Where the macros accept
//! multiple spellings (case-insensitive markers, `primary`/`PRIMARY_KEY`,
//! `collate = NOCASE` vs `collate = "NOCASE"`, ...), the same set is accepted
//! here; where the macros reject input with a compile error, an entry is
//! pushed to [`Diags::errors`]; where they silently ignore input, so does the
//! parser.
//!
//! Keeping the interpretation in one module (with unit tests per attribute at
//! the bottom) makes future macro/parser drift testable.

use drizzle_types::Dialect;
use drizzle_types::mysql::{MySQLType, TypeCategory as MySQLRustTypeCategory};
use drizzle_types::sqlite::SQLiteType;
use proc_macro2::Span;
use quote::ToTokens;
use syn::ext::IdentExt;
use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, Ident, Lit, Meta, Token, Type};

use super::types::{
    ColumnSpec, CompositeFkSpec, IndexSpec, ParsedDefault, ParsedGenerated, ParsedIdentity,
    ParsedReference, SerialKind, TableCheckSpec, TableSpec, TableUniqueSpec,
};

/// Warning / error sinks threaded through interpretation.
pub(crate) struct Diags {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl Diags {
    pub(crate) fn new() -> Self {
        Self {
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }
}

// =============================================================================
// Source-text helpers
// =============================================================================

/// Slice the original source for a span. Falls back to token printing when
/// the span range is out of bounds (should not happen for `syn::parse_file`
/// output, but never panic on user input).
pub(crate) fn source_slice(source: &str, span: Span, fallback: &dyn Fn() -> String) -> String {
    let range = span.byte_range();
    source
        .get(range)
        .map_or_else(fallback, std::string::ToString::to_string)
}

/// Source text of any `ToTokens` value.
pub(crate) fn spanned_source<T: ToTokens + Spanned>(source: &str, value: &T) -> String {
    source_slice(source, value.span(), &|| {
        value.to_token_stream().to_string()
    })
}

/// Last path segment ident of an attribute path, as a string.
pub(crate) fn attr_last_segment(attr: &Attribute) -> Option<String> {
    attr.path().segments.last().map(|s| s.ident.to_string())
}

/// Doc comment from `#[doc = "..."]` attributes, matching
/// `postgres::field::doc_comment_from_attrs` (single leading space stripped
/// per line, lines joined with `\n`).
pub(crate) fn doc_comment(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            let Meta::NameValue(meta) = &attr.meta else {
                return None;
            };
            let Expr::Lit(expr_lit) = &meta.value else {
                return None;
            };
            let Lit::Str(lit) = &expr_lit.lit else {
                return None;
            };
            let value = lit.value();
            Some(value.strip_prefix(' ').unwrap_or(&value).to_string())
        })
        .collect();
    let comment = lines.join("\n");
    if comment.is_empty() {
        None
    } else {
        Some(comment)
    }
}

/// Whether the item has any `#[cfg(...)]` attribute.
pub(crate) fn has_cfg(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("cfg"))
}

/// Validate a referential action ident the way both field macros do
/// (`CASCADE`, `SET_NULL` → `SET NULL`, ...). Returns `None` for invalid
/// actions.
pub(crate) fn normalize_referential_action(action: &str) -> Option<String> {
    match action.to_ascii_uppercase().as_str() {
        "CASCADE" => Some("CASCADE".to_string()),
        "SET_NULL" => Some("SET NULL".to_string()),
        "SET_DEFAULT" => Some("SET DEFAULT".to_string()),
        "RESTRICT" => Some("RESTRICT".to_string()),
        "NO_ACTION" => Some("NO ACTION".to_string()),
        _ => None,
    }
}

// =============================================================================
// Naming helpers (macro-equivalent name derivation)
// =============================================================================

/// SQLite index name derivation: the naive uppercase-fold the `SQLiteIndex`
/// macro uses (`IdxUsersEmail` → `idx_users_email`). Note this is *not* heck
/// snake-casing (consecutive capitals each get an underscore).
pub(crate) fn sqlite_index_name(struct_ident: &str) -> String {
    struct_ident
        .chars()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if i > 0 && c.is_uppercase() {
                acc.push('_');
            }
            acc.extend(c.to_lowercase());
            acc
        })
}

/// Postgres index name derivation (`generate_index_name` in
/// `procmacros/src/postgres/index.rs`): heck snake-case plus an `_idx`
/// suffix unless it already ends with `_idx` / `_index`.
pub(crate) fn postgres_index_name(struct_ident: &str) -> String {
    let snake = heck::AsSnakeCase(struct_ident).to_string();
    if snake.ends_with("_idx") || snake.ends_with("_index") {
        snake
    } else {
        format!("{snake}_idx")
    }
}

/// MySQL index name derivation (`mysql::index::mysql_index_attr_macro`):
/// heck snake-case, with no SQLite-style suffix.
pub(crate) fn mysql_index_name(struct_ident: &str) -> String {
    heck::AsSnakeCase(struct_ident).to_string()
}

// =============================================================================
// Rust-type classification (mirrors procmacros/src/common/type_utils.rs)
// =============================================================================

fn last_ident(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        p.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

fn last_ident_is(ty: &Type, name: &str) -> bool {
    last_ident(ty).is_some_and(|id| id == name)
}

fn path_contains(ty: &Type, name: &str) -> bool {
    if let Type::Path(p) = ty {
        p.path.segments.iter().any(|s| s.ident == name)
    } else {
        false
    }
}

/// `Option<T>` detection by last path segment (accepts
/// `std::option::Option<T>` and `core::option::Option<T>`).
pub(crate) fn is_option_type(ty: &Type) -> bool {
    last_ident_is(ty, "Option")
}

/// Inner type of `Option<T>`.
pub(crate) fn option_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(p) = ty else { return None };
    let segment = p.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| {
        if let syn::GenericArgument::Type(inner) = arg {
            Some(inner)
        } else {
            None
        }
    })
}

pub(crate) fn unwrap_option(ty: &Type) -> &Type {
    option_inner_type(ty).unwrap_or(ty)
}

fn vec_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(p) = ty else { return None };
    let segment = p.path.segments.last()?;
    if segment.ident != "Vec" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| {
        if let syn::GenericArgument::Type(inner) = arg {
            Some(inner)
        } else {
            None
        }
    })
}

fn is_array_of(ty: &Type, elem: &str) -> bool {
    matches!(ty, Type::Array(a) if last_ident_is(&a.elem, elem))
}

fn is_vec_u8(ty: &Type) -> bool {
    vec_inner_type(ty).is_some_and(|inner| last_ident_is(inner, "u8"))
}

fn is_byte_slice(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => {
            matches!(r.elem.as_ref(), Type::Slice(s) if last_ident_is(&s.elem, "u8"))
        }
        Type::Slice(s) => last_ident_is(&s.elem, "u8"),
        _ => false,
    }
}

fn is_string_like(ty: &Type) -> bool {
    if last_ident_is(ty, "String") {
        return true;
    }
    if let Type::Reference(r) = ty
        && let Type::Path(p) = r.elem.as_ref()
    {
        return p.path.segments.last().is_some_and(|s| s.ident == "str");
    }
    false
}

fn is_array_string(ty: &Type) -> bool {
    // The macro gates `CompactString` behind the `compact-str` feature; the
    // parser cannot know the user's feature set, so it accepts the superset.
    last_ident_is(ty, "ArrayString") || last_ident_is(ty, "CompactString")
}

fn is_arrayvec_u8_like(ty: &Type) -> bool {
    // `bytes::Bytes`/`BytesMut` and `SmallVec<[u8; N]>` are feature-gated in
    // the macro; accepted unconditionally here (see `is_array_string`).
    if last_ident_is(ty, "Bytes") || last_ident_is(ty, "BytesMut") {
        return true;
    }
    let Type::Path(p) = ty else { return false };
    let Some(segment) = p.path.segments.last() else {
        return false;
    };
    if segment.ident == "SmallVec" {
        let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
            return false;
        };
        return args.args.iter().any(|arg| {
            matches!(arg, syn::GenericArgument::Type(Type::Array(a)) if last_ident_is(&a.elem, "u8"))
        });
    }
    if segment.ident != "ArrayVec" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    args.args.iter().any(|arg| {
        matches!(arg, syn::GenericArgument::Type(Type::Path(inner))
            if inner.path.segments.last().is_some_and(|s| s.ident == "u8"))
    })
}

fn is_json_value(ty: &Type) -> bool {
    last_ident_is(ty, "Value") && path_contains(ty, "serde_json")
}

fn is_int(ty: &Type, name: &str) -> bool {
    last_ident_is(ty, name)
}

/// SQLite storage-class inference, mirroring
/// `sqlite::field::type_category_from_type` +
/// `TypeCategory::to_sqlite_type`. Returns `None` for unknown types (the
/// macros defer those to a trait const the parser cannot evaluate).
pub(crate) fn infer_sqlite_type(ty: &Type) -> Option<SQLiteType> {
    let ty = unwrap_option(ty);

    if is_array_of(ty, "u8") {
        return Some(SQLiteType::Blob);
    }
    if is_array_string(ty) {
        return Some(SQLiteType::Text);
    }
    if is_arrayvec_u8_like(ty) {
        return Some(SQLiteType::Blob);
    }
    if last_ident_is(ty, "Uuid") {
        return Some(SQLiteType::Blob);
    }
    if is_json_value(ty) {
        return Some(SQLiteType::Text);
    }
    if [
        "NaiveDate",
        "NaiveTime",
        "NaiveDateTime",
        "DateTime",
        "Date",
        "Time",
        "PrimitiveDateTime",
        "OffsetDateTime",
    ]
    .iter()
    .any(|name| last_ident_is(ty, name))
    {
        return Some(SQLiteType::Text);
    }
    if is_string_like(ty) {
        return Some(SQLiteType::Text);
    }
    if is_vec_u8(ty) || is_byte_slice(ty) {
        return Some(SQLiteType::Blob);
    }
    if last_ident_is(ty, "bool") {
        return Some(SQLiteType::Integer);
    }
    if [
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "isize", "usize",
    ]
    .iter()
    .any(|name| is_int(ty, name))
    {
        return Some(SQLiteType::Integer);
    }
    if is_int(ty, "f32") || is_int(ty, "f64") {
        return Some(SQLiteType::Real);
    }

    None
}

/// Scalar `PostgreSQL` type inference, mirroring
/// `postgres::field::TypeCategory::{from_type, to_postgres_type}`. Returns
/// the uppercase SQL type string, or `None` for unknown/custom types.
pub(crate) fn infer_postgres_scalar_type(ty: &Type) -> Option<&'static str> {
    let ty = unwrap_option(ty);

    if is_array_of(ty, "u8") {
        return Some("BYTEA");
    }
    if is_array_of(ty, "char") {
        return Some("CHAR");
    }
    if is_array_string(ty) {
        return Some("VARCHAR");
    }
    if is_arrayvec_u8_like(ty) {
        return Some("BYTEA");
    }
    if last_ident_is(ty, "Uuid") {
        return Some("UUID");
    }
    if is_json_value(ty) {
        return Some("JSONB");
    }
    for (name, sql) in [
        ("NaiveDate", "DATE"),
        ("NaiveTime", "TIME"),
        ("NaiveDateTime", "TIMESTAMP"),
        ("DateTime", "TIMESTAMPTZ"),
        ("Date", "DATE"),
        ("Time", "TIME"),
        ("PrimitiveDateTime", "TIMESTAMP"),
        ("OffsetDateTime", "TIMESTAMPTZ"),
        ("Point", "POINT"),
        ("Rect", "BOX"),
        ("LineString", "PATH"),
        ("IpAddr", "INET"),
        ("IpInet", "INET"),
        ("IpCidr", "CIDR"),
        ("MacAddress", "MACADDR"),
        ("BitVec", "VARBIT"),
    ] {
        if last_ident_is(ty, name) {
            return Some(sql);
        }
    }
    if is_string_like(ty) {
        return Some("TEXT");
    }
    if is_vec_u8(ty) {
        return Some("BYTEA");
    }
    for (name, sql) in [
        ("i16", "SMALLINT"),
        ("i32", "INTEGER"),
        ("i64", "BIGINT"),
        ("f32", "REAL"),
        ("f64", "DOUBLE PRECISION"),
        ("bool", "BOOLEAN"),
    ] {
        if is_int(ty, name) {
            return Some(sql);
        }
    }

    None
}

/// Array element categories the Postgres macro supports for `Vec<T>`
/// (`postgres::field::is_supported_array_category`). Note the time-crate
/// types are intentionally absent — the macro doesn't support them in arrays.
fn postgres_array_element_type(ty: &Type) -> Option<&'static str> {
    if is_string_like(ty) {
        return Some("TEXT");
    }
    for (name, sql) in [
        ("i16", "SMALLINT"),
        ("i32", "INTEGER"),
        ("i64", "BIGINT"),
        ("f32", "REAL"),
        ("f64", "DOUBLE PRECISION"),
        ("bool", "BOOLEAN"),
        ("Uuid", "UUID"),
        ("NaiveDate", "DATE"),
        ("NaiveTime", "TIME"),
        ("NaiveDateTime", "TIMESTAMP"),
        ("DateTime", "TIMESTAMPTZ"),
    ] {
        if last_ident_is(ty, name) {
            return Some(sql);
        }
    }
    None
}

/// `Vec<T>` array detection mirroring `postgres::field::postgres_array_info`
/// (rejects `Vec<u8>` and nested `Vec<Vec<T>>`).
pub(crate) fn infer_postgres_array(ty: &Type) -> Option<&'static str> {
    let ty = unwrap_option(ty);
    if is_vec_u8(ty) {
        return None;
    }
    let inner = vec_inner_type(ty)?;
    if vec_inner_type(inner).is_some() {
        return None;
    }
    postgres_array_element_type(inner)
}

/// The Rust type printed the way the Postgres macro prints enum base types
/// (`to_token_stream().to_string().replace(' ', "")`).
pub(crate) fn type_token_string(ty: &Type) -> String {
    ty.to_token_stream().to_string().replace(' ', "")
}

// =============================================================================
// SQLite column attributes (mirrors sqlite::field::{parse_attributes, parse_args})
// =============================================================================

/// Per-attribute scratch, merged first-wins across attributes like the
/// macro's `AttributeData` merge.
#[derive(Default)]
struct SqliteArgs {
    explicit_type: Option<SQLiteType>,
    primary: bool,
    autoincrement: bool,
    unique: bool,
    json: bool,
    enum_marker: bool,
    default: Option<ParsedDefault>,
    default_raw: Option<(String, String)>,
    default_sql: Option<String>,
    default_fn: bool,
    generated: Option<ParsedGenerated>,
    check: Option<String>,
    references: Option<ParsedReference>,
    on_delete: Option<String>,
    on_delete_raw: Option<String>,
    on_update: Option<String>,
    on_update_raw: Option<String>,
    name: Option<String>,
    collate: Option<String>,
    relation: Option<String>,
    named_values: Vec<(String, String)>,
}

/// Parse one attribute item, handling the `enum` keyword and `generated(...)`
/// like the macro's `parse_item`.
fn parse_sqlite_item(input: ParseStream) -> syn::Result<Expr> {
    if input.peek(Ident::peek_any) {
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
                return Err(syn::Error::new(
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
            args.push_punct(<Token![,]>::default());
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

    if input.peek(Token![enum]) {
        input.parse::<Token![enum]>()?;
        let ident = Ident::new("enum", Span::call_site());
        return Ok(Expr::Path(syn::ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: syn::Path::from(ident),
        }));
    }
    input.parse::<Expr>()
}

fn reference_from_expr(expr: &Expr) -> Option<ParsedReference> {
    let Expr::Path(path) = expr else { return None };
    if path.path.segments.len() != 2 {
        return None;
    }
    Some(ParsedReference {
        table: path.path.segments.first()?.ident.to_string(),
        column: path.path.segments.last()?.ident.to_string(),
    })
}

fn default_from_expr(expr: &Expr, source: &str) -> ParsedDefault {
    if let Expr::Lit(expr_lit) = expr {
        match &expr_lit.lit {
            Lit::Int(i) => return ParsedDefault::Int(i.to_string()),
            Lit::Float(f) => return ParsedDefault::Float(f.to_string()),
            Lit::Bool(b) => return ParsedDefault::Bool(b.value()),
            Lit::Str(s) => return ParsedDefault::Str(s.value()),
            _ => {}
        }
    }
    ParsedDefault::Unsupported(spanned_source(source, expr))
}

#[allow(clippy::too_many_lines)]
fn parse_sqlite_args(
    tokens: proc_macro2::TokenStream,
    source: &str,
    field_desc: &str,
    diags: &mut Diags,
) -> SqliteArgs {
    let mut args = SqliteArgs::default();

    let item_parser = |input: ParseStream| {
        Punctuated::<Expr, Token![,]>::parse_terminated_with(input, parse_sqlite_item)
    };
    let parsed = item_parser.parse2(tokens.clone());
    let items = match parsed {
        Ok(items) => items,
        Err(err) => {
            diags.errors.push(format!(
                "{field_desc}: failed to parse column attribute: {err}"
            ));
            return args;
        }
    };

    for expr in &items {
        match expr {
            Expr::Path(path) => {
                let Some(ident) = path.path.get_ident() else {
                    continue;
                };
                let ident_str = ident.to_string();
                match ident_str.to_ascii_uppercase().as_str() {
                    "JSON" => {
                        args.explicit_type = Some(SQLiteType::Text);
                        args.json = true;
                    }
                    "JSONB" => {
                        args.explicit_type = Some(SQLiteType::Blob);
                        args.json = true;
                    }
                    // Bare DEFAULT means `default_fn = Default::default`.
                    "DEFAULT" => args.default_fn = true,
                    "ENUM" => args.enum_marker = true,
                    "PRIMARY" | "PRIMARY_KEY" => args.primary = true,
                    "AUTOINCREMENT" => args.autoincrement = true,
                    "UNIQUE" => args.unique = true,
                    _ => {
                        if let Some(ty) = SQLiteType::from_attribute_name(&ident_str) {
                            args.explicit_type = Some(ty);
                        }
                        // Unknown bare flags are collected silently by the
                        // macro; mirrored here (no diagnostic).
                    }
                }
            }
            Expr::Assign(assign) => {
                let Expr::Path(path) = &*assign.left else {
                    continue;
                };
                let Some(param) = path.path.get_ident() else {
                    continue;
                };
                let key = param.to_string();
                let raw_value = spanned_source(source, &*assign.right);
                args.named_values.push((key.clone(), raw_value.clone()));
                match key.to_ascii_uppercase().as_str() {
                    "DEFAULT" => {
                        let default = default_from_expr(&assign.right, source);
                        if matches!(default, ParsedDefault::Unsupported(_)) {
                            diags.warnings.push(format!(
                                "{field_desc}: `default = {raw_value}` is not a literal the \
                                 table macros emit into DDL; the default is ignored"
                            ));
                        }
                        args.default_raw = Some((key, raw_value));
                        args.default = Some(default);
                    }
                    "DEFAULT_SQL" => {
                        if let Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &*assign.right
                        {
                            args.default_sql = Some(lit_str.value());
                        } else {
                            diags.errors.push(format!(
                                "{field_desc}: default_sql requires a string literal"
                            ));
                        }
                    }
                    "DEFAULT_FN" => args.default_fn = true,
                    "REFERENCES" => {
                        args.references = reference_from_expr(&assign.right);
                        if args.references.is_none() {
                            diags.warnings.push(format!(
                                "{field_desc}: `references = {raw_value}` is not a \
                                 `Table::column` path; the foreign key is ignored"
                            ));
                        }
                    }
                    "RELATION" => {
                        if let Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &*assign.right
                        {
                            args.relation = Some(lit_str.value());
                        } else {
                            diags
                                .errors
                                .push(format!("{field_desc}: relation requires a string literal"));
                        }
                    }
                    "ON_DELETE" | "ON_UPDATE" => {
                        let is_delete = key.eq_ignore_ascii_case("on_delete");
                        if let Expr::Path(action_path) = &*assign.right
                            && let Some(action_ident) = action_path.path.get_ident()
                        {
                            let raw = action_ident.to_string();
                            // The SQLite macro swallows invalid actions
                            // (`validate_referential_action(..).ok()`);
                            // surface a warning instead of silence.
                            let normalized = normalize_referential_action(&raw);
                            if normalized.is_none() {
                                diags.warnings.push(format!(
                                    "{field_desc}: invalid referential action `{raw}`; \
                                     expected CASCADE, SET_NULL, SET_DEFAULT, RESTRICT, or \
                                     NO_ACTION (action ignored, matching the macro)"
                                ));
                            }
                            if is_delete {
                                args.on_delete = normalized;
                                args.on_delete_raw = Some(raw);
                            } else {
                                args.on_update = normalized;
                                args.on_update_raw = Some(raw);
                            }
                        } else {
                            diags.warnings.push(format!(
                                "{field_desc}: `{key} = {raw_value}` is not a bare action \
                                 ident; the macro ignores this form"
                            ));
                        }
                    }
                    "NAME" => {
                        if let Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &*assign.right
                        {
                            args.name = Some(lit_str.value());
                        }
                    }
                    "COLLATE" => match &*assign.right {
                        Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) => args.collate = Some(lit_str.value()),
                        Expr::Path(p) => {
                            if let Some(ident) = p.path.get_ident() {
                                // Bare idents are uppercased by the macro.
                                args.collate = Some(ident.to_string().to_ascii_uppercase());
                            }
                        }
                        _ => {}
                    },
                    "CHECK" => {
                        if let Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &*assign.right
                        {
                            args.check = Some(lit_str.value());
                        } else {
                            diags
                                .errors
                                .push(format!("{field_desc}: CHECK requires a string literal"));
                        }
                    }
                    // Unknown `key = value` pairs are ignored by the macro.
                    _ => {}
                }
            }
            Expr::Call(call) => {
                let Expr::Path(path) = &*call.func else {
                    continue;
                };
                let Some(ident) = path.path.get_ident() else {
                    continue;
                };
                if !ident.to_string().eq_ignore_ascii_case("generated") {
                    continue;
                }
                let mut call_args = call.args.iter();
                let (Some(kind_expr), Some(expr_arg)) = (call_args.next(), call_args.next()) else {
                    diags.errors.push(format!(
                        "{field_desc}: #[column(generated(...))] requires \
                         generated(stored|virtual, \"expression\")"
                    ));
                    continue;
                };
                let stored = match kind_expr {
                    Expr::Path(kind_path) => match kind_path
                        .path
                        .get_ident()
                        .map(|id| id.to_string().to_ascii_lowercase())
                        .as_deref()
                    {
                        Some("stored") => true,
                        Some("virtual") => false,
                        _ => {
                            diags.errors.push(format!(
                                "{field_desc}: expected `stored` or `virtual` for generated(...)"
                            ));
                            continue;
                        }
                    },
                    _ => {
                        diags.errors.push(format!(
                            "{field_desc}: expected `stored` or `virtual` for generated(...)"
                        ));
                        continue;
                    }
                };
                let Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(expr_lit),
                    ..
                }) = expr_arg
                else {
                    diags.errors.push(format!(
                        "{field_desc}: expected a string literal for the generated(...) expression"
                    ));
                    continue;
                };
                args.generated = Some(ParsedGenerated {
                    expression: expr_lit.value(),
                    stored,
                });
            }
            _ => {}
        }
    }

    args
}

/// Interpret the attributes of a field on an `SQLite` table struct.
pub(crate) fn sqlite_column_spec(
    field: &syn::Field,
    source: &str,
    table_desc: &str,
    diags: &mut Diags,
) -> ColumnSpec {
    let mut spec = ColumnSpec {
        nullable: is_option_type(&field.ty),
        comment: doc_comment(&field.attrs),
        ..ColumnSpec::default()
    };
    let field_name = field
        .ident
        .as_ref()
        .map_or_else(|| "<unnamed>".to_string(), std::string::ToString::to_string);
    let field_desc = format!("{table_desc}.{field_name}");

    let mut explicit_type: Option<SQLiteType> = None;
    let mut seen_column_data = false;

    for attr in &field.attrs {
        let Some(ident) = attr
            .path()
            .get_ident()
            .map(std::string::ToString::to_string)
        else {
            continue;
        };

        // Legacy per-type attribute (`#[text]`, `#[integer(primary)]`, ...).
        let legacy_type = SQLiteType::from_attribute_name(&ident);
        let is_column = ident == "column";
        if legacy_type.is_none() && !is_column {
            continue;
        }

        if let Some(ty) = legacy_type {
            explicit_type = explicit_type.or(Some(ty));
        }

        let args = match &attr.meta {
            Meta::Path(_) => continue,
            Meta::List(list) => parse_sqlite_args(list.tokens.clone(), source, &field_desc, diags),
            Meta::NameValue(_) => continue,
        };

        // Merge with the macro's first-attribute-wins semantics.
        if !seen_column_data {
            seen_column_data = true;
        }
        explicit_type = explicit_type.or(args.explicit_type);
        spec.primary |= args.primary;
        spec.autoincrement |= args.autoincrement;
        spec.unique |= args.unique;
        spec.json |= args.json;
        spec.enum_marker |= args.enum_marker;
        if spec.default.is_none() {
            spec.default = args.default;
        }
        spec.default_sql = spec.default_sql.take().or(args.default_sql);
        spec.has_default_fn |= args.default_fn;
        if spec.generated.is_none() {
            spec.generated = args.generated;
        }
        spec.check = spec.check.take().or(args.check);
        if spec.references.is_none() {
            spec.references = args.references;
        }
        spec.on_delete = spec.on_delete.take().or(args.on_delete);
        spec.on_delete_raw = spec.on_delete_raw.take().or(args.on_delete_raw);
        spec.on_update = spec.on_update.take().or(args.on_update);
        spec.on_update_raw = spec.on_update_raw.take().or(args.on_update_raw);
        spec.explicit_name = spec.explicit_name.take().or(args.name);
        spec.collate = spec.collate.take().or(args.collate);
        spec.relation = spec.relation.take().or(args.relation);
        spec.named_values.extend(args.named_values);
    }

    // on_delete / on_update / relation require `references` (macro compile
    // errors).
    if (spec.on_delete_raw.is_some() || spec.on_update_raw.is_some()) && spec.references.is_none() {
        diags.errors.push(format!(
            "{field_desc}: on_delete/on_update require a `references = Table::column` attribute"
        ));
    }
    if spec.relation.is_some() && spec.references.is_none() {
        diags.errors.push(format!(
            "{field_desc}: relation requires a `references = Table::column` attribute"
        ));
    }

    // Resolve the SQLite storage type exactly like the macro: explicit type
    // wins; json/enum markers imply TEXT; unknown types fall back to ANY with
    // the real type deferred to a trait const the parser cannot evaluate.
    let base_type = unwrap_option(&field.ty);
    let resolved = if let Some(ty) = explicit_type {
        ty
    } else if spec.json || spec.enum_marker || is_json_value(base_type) {
        SQLiteType::Text
    } else if let Some(ty) = infer_sqlite_type(base_type) {
        ty
    } else {
        spec.is_custom_type = true;
        diags.warnings.push(format!(
            "{field_desc}: unknown Rust type `{}` — the macro defers the SQL type to the \
             DrizzleSQLiteColumn trait; the parser records ANY",
            type_token_string(base_type)
        ));
        SQLiteType::Any
    };
    spec.sqlite_type = Some(resolved.to_sql_type().to_string());

    spec
}

// =============================================================================
// MySQL column attributes (mirrors mysql::field::parse_column_attrs)
// =============================================================================

/// Parsed MySQL-only column state before it is folded into [`ColumnSpec`].
/// Keeping the native type as [`MySQLType`] until the end prevents losing
/// signedness and distinguishes inline `ENUM`/`SET` from an arbitrary SQL
/// type string.
#[derive(Default)]
struct MySqlArgs {
    explicit_type: Option<MySQLType>,
    type_args: Vec<u16>,
    primary: bool,
    unique: bool,
    not_null: bool,
    autoincrement: bool,
    enum_marker: bool,
    set_values: Option<Vec<String>>,
    default: Option<ParsedDefault>,
    default_sql: Option<String>,
    default_fn: bool,
    generated: Option<ParsedGenerated>,
    check: Option<String>,
    references: Option<ParsedReference>,
    on_delete: Option<String>,
    on_update: Option<String>,
    on_delete_raw: Option<String>,
    on_update_raw: Option<String>,
    name: Option<String>,
    relation: Option<String>,
    charset: Option<String>,
    collate: Option<String>,
    mysql_on_update: Option<String>,
    comment: Option<String>,
    named_values: Vec<(String, String)>,
}

fn mysql_string_value(
    expr: &Expr,
    field_desc: &str,
    attr_name: &str,
    diags: &mut Diags,
) -> Option<String> {
    let Some(value) = lit_str_value(expr) else {
        diags.errors.push(format!(
            "{field_desc}: {attr_name} requires a string literal"
        ));
        return None;
    };
    Some(value)
}

fn set_mysql_explicit_type(
    args: &mut MySqlArgs,
    ty: MySQLType,
    field_desc: &str,
    diags: &mut Diags,
) {
    if args.explicit_type.replace(ty).is_some() {
        diags.errors.push(format!(
            "{field_desc}: a MySQL column may specify only one SQL type"
        ));
    }
}

fn mysql_referential_action(value: &str) -> Option<String> {
    match value.to_ascii_uppercase().replace('_', " ").as_str() {
        "CASCADE" => Some("CASCADE".to_string()),
        "SET NULL" => Some("SET NULL".to_string()),
        "RESTRICT" => Some("RESTRICT".to_string()),
        "NO ACTION" => Some("NO ACTION".to_string()),
        // MySQL/InnoDB rejects SET DEFAULT. Do not reuse the SQLite/Postgres
        // normalizer here: accepting it would make the parser less strict
        // than the MySQL macro.
        _ => None,
    }
}

fn mysql_parse_type_args(list: &syn::MetaList, field_desc: &str, diags: &mut Diags) -> Vec<u16> {
    let parsed = Punctuated::<syn::LitInt, Token![,]>::parse_terminated.parse2(list.tokens.clone());
    match parsed {
        Ok(values) => values
            .into_iter()
            .filter_map(|value| match value.base10_parse::<u16>() {
                Ok(value) => Some(value),
                Err(_) => {
                    diags.errors.push(format!(
                        "{field_desc}: MySQL type arguments must fit in an unsigned 16-bit integer"
                    ));
                    None
                }
            })
            .collect(),
        Err(_) => {
            diags.errors.push(format!(
                "{field_desc}: MySQL type arguments must be integer literals"
            ));
            Vec::new()
        }
    }
}

fn mysql_parse_generated(
    list: &syn::MetaList,
    field_desc: &str,
    diags: &mut Diags,
) -> Option<ParsedGenerated> {
    let args = match Punctuated::<Expr, Token![,]>::parse_terminated.parse2(list.tokens.clone()) {
        Ok(args) => args,
        Err(err) => {
            diags.errors.push(format!(
                "{field_desc}: generated(...) could not be parsed: {err}"
            ));
            return None;
        }
    };
    if args.len() != 2 {
        diags.errors.push(format!(
            "{field_desc}: generated(...) accepts exactly `stored|virtual, \"expression\"`"
        ));
        return None;
    }
    let Some(kind) = args.first().and_then(|expr| match expr {
        Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
        _ => None,
    }) else {
        diags.errors.push(format!(
            "{field_desc}: generated(...) expects `stored` or `virtual` as its first argument"
        ));
        return None;
    };
    let stored = match kind.to_ascii_lowercase().as_str() {
        "stored" => true,
        "virtual" => false,
        _ => {
            diags.errors.push(format!(
                "{field_desc}: generated(...) expects `stored` or `virtual`"
            ));
            return None;
        }
    };
    let Some(expression) = args.get(1).and_then(lit_str_value) else {
        diags.errors.push(format!(
            "{field_desc}: generated(...) expression must be a string literal"
        ));
        return None;
    };
    Some(ParsedGenerated { expression, stored })
}

fn mysql_parse_set_values(
    list: &syn::MetaList,
    field_desc: &str,
    diags: &mut Diags,
) -> Option<Vec<String>> {
    let parsed =
        match Punctuated::<syn::LitStr, Token![,]>::parse_terminated.parse2(list.tokens.clone()) {
            Ok(values) => values,
            Err(_) => {
                diags.errors.push(format!(
                    "{field_desc}: SET expects string values: SET(\"a\", \"b\")"
                ));
                return None;
            }
        };
    if parsed.is_empty() {
        diags.errors.push(format!(
            "{field_desc}: MySQL SET requires at least one value"
        ));
        return None;
    }
    if parsed.len() > 64 {
        diags.errors.push(format!(
            "{field_desc}: MySQL SET supports at most 64 values"
        ));
        return None;
    }
    let mut values = Vec::with_capacity(parsed.len());
    for value in parsed {
        let value = value.value();
        if value.is_empty() || value.contains(['\0', '\'', '\\']) {
            diags.errors.push(format!(
                "{field_desc}: MySQL inline SET labels must be non-empty and cannot contain NUL, quotes, or backslashes"
            ));
            return None;
        }
        values.push(value);
    }
    Some(values)
}

fn mysql_array_capacity(ty: &Type) -> Option<u16> {
    let Type::Array(array) = ty else { return None };
    let Expr::Lit(lit) = &array.len else {
        return None;
    };
    let Lit::Int(value) = &lit.lit else {
        return None;
    };
    value.base10_parse::<u16>().ok()
}

fn mysql_const_capacity(ty: &Type) -> Option<u16> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let expression = arguments.args.iter().rev().find_map(|argument| {
        if let syn::GenericArgument::Const(expression) = argument {
            Some(expression)
        } else {
            None
        }
    })?;
    let Expr::Lit(lit) = expression else {
        return None;
    };
    let Lit::Int(value) = &lit.lit else {
        return None;
    };
    value.base10_parse::<u16>().ok()
}

fn mysql_inferred_type(ty: &Type) -> Option<(MySQLType, Vec<u16>)> {
    let rendered = type_token_string(ty);
    let category = MySQLRustTypeCategory::from_type_string(&rendered);
    match category {
        MySQLRustTypeCategory::Uuid => Some((MySQLType::Binary, vec![16])),
        MySQLRustTypeCategory::ArrayString if last_ident_is(ty, "CompactString") => {
            Some((MySQLType::Text, Vec::new()))
        }
        MySQLRustTypeCategory::ArrayString => {
            mysql_const_capacity(ty).map(|capacity| (MySQLType::Varchar, vec![capacity]))
        }
        MySQLRustTypeCategory::ByteArray => {
            mysql_array_capacity(ty).map(|capacity| (MySQLType::Binary, vec![capacity]))
        }
        MySQLRustTypeCategory::CharArray => {
            mysql_array_capacity(ty).map(|capacity| (MySQLType::Char, vec![capacity]))
        }
        MySQLRustTypeCategory::ArrayVec if last_ident_is(ty, "ArrayVec") => {
            mysql_const_capacity(ty).map(|capacity| (MySQLType::Varbinary, vec![capacity]))
        }
        MySQLRustTypeCategory::ArrayVec => Some((MySQLType::Blob, Vec::new())),
        _ => category.to_mysql_type().map(|ty| (ty, Vec::new())),
    }
}

fn mysql_render_type(ty: &MySQLType, args: &[u16]) -> String {
    match ty {
        MySQLType::Enum(_) => "ENUM".to_string(),
        MySQLType::Set(values) => format!(
            "SET({})",
            values
                .iter()
                .map(|value| format!("'{value}'"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ if args.is_empty() => ty.to_sql_type().to_string(),
        _ => format!(
            "{}({})",
            ty.to_sql_type(),
            args.iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn mysql_supports_character_options(ty: &MySQLType) -> bool {
    matches!(
        ty,
        MySQLType::Char
            | MySQLType::Varchar
            | MySQLType::Tinytext
            | MySQLType::Text
            | MySQLType::Mediumtext
            | MySQLType::Longtext
            | MySQLType::Enum(_)
            | MySQLType::Set(_)
    )
}

fn mysql_validate_type_args(ty: &MySQLType, args: &[u16]) -> Option<&'static str> {
    match ty {
        MySQLType::Varchar | MySQLType::Varbinary if args.len() != 1 => {
            Some("VARCHAR and VARBINARY require exactly one length argument")
        }
        MySQLType::Char | MySQLType::Binary if args.len() > 1 => {
            Some("CHAR and BINARY accept at most one length argument")
        }
        MySQLType::Bit if args.len() > 1 => Some("BIT accepts at most one width argument"),
        MySQLType::Decimal if args.len() > 2 => {
            Some("DECIMAL accepts precision and optional scale")
        }
        MySQLType::Time | MySQLType::Datetime | MySQLType::Timestamp if args.len() > 1 => {
            Some("temporal types accept at most one fractional-seconds precision")
        }
        _ if !args.is_empty()
            && !matches!(
                ty,
                MySQLType::Varchar
                    | MySQLType::Varbinary
                    | MySQLType::Char
                    | MySQLType::Binary
                    | MySQLType::Bit
                    | MySQLType::Decimal
                    | MySQLType::Time
                    | MySQLType::Datetime
                    | MySQLType::Timestamp
            ) =>
        {
            Some("this MySQL type does not accept length, precision, or scale arguments")
        }
        MySQLType::Bit if args.first().is_some_and(|value| !(1..=64).contains(value)) => {
            Some("BIT width must be between 1 and 64")
        }
        MySQLType::Char | MySQLType::Binary if args.first().is_some_and(|value| *value > 255) => {
            Some("CHAR/BINARY length must not exceed 255")
        }
        MySQLType::Decimal if args.first().is_some_and(|value| !(1..=65).contains(value)) => {
            Some("DECIMAL precision must be between 1 and 65")
        }
        MySQLType::Decimal
            if args.get(1).is_some_and(|scale| {
                *scale > 30 || args.first().is_some_and(|precision| scale > precision)
            }) =>
        {
            Some("DECIMAL scale must not exceed 30 or its precision")
        }
        MySQLType::Time | MySQLType::Datetime | MySQLType::Timestamp
            if args.first().is_some_and(|value| *value > 6) =>
        {
            Some("fractional-seconds precision must be between 0 and 6")
        }
        _ => None,
    }
}

fn mysql_validate_explicit_signedness(
    rust_type: &Type,
    sql_type: &MySQLType,
    field_desc: &str,
    diags: &mut Diags,
) {
    let rust_category = MySQLRustTypeCategory::from_type_string(&type_token_string(rust_type));
    let rust_unsigned = matches!(
        rust_category,
        MySQLRustTypeCategory::U8
            | MySQLRustTypeCategory::U16
            | MySQLRustTypeCategory::U32
            | MySQLRustTypeCategory::U64
            | MySQLRustTypeCategory::Usize
    );
    let rust_signed = matches!(
        rust_category,
        MySQLRustTypeCategory::I8
            | MySQLRustTypeCategory::I16
            | MySQLRustTypeCategory::I32
            | MySQLRustTypeCategory::I64
            | MySQLRustTypeCategory::Isize
    );
    let sql_signed = matches!(
        sql_type,
        MySQLType::Tinyint
            | MySQLType::Smallint
            | MySQLType::Mediumint
            | MySQLType::Int
            | MySQLType::Bigint
    );

    if sql_type.is_unsigned() && rust_signed {
        diags.errors.push(format!(
            "{field_desc}: {} is unsigned but the Rust field is signed; use the corresponding u8/u16/u32/u64 type",
            sql_type.to_sql_type()
        ));
    }
    if sql_signed && rust_unsigned {
        diags.errors.push(format!(
            "{field_desc}: {} is signed but the Rust field is unsigned; use the corresponding signed Rust type or an *_UNSIGNED SQL type",
            sql_type.to_sql_type()
        ));
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn mysql_column_spec(
    field: &syn::Field,
    source: &str,
    table_desc: &str,
    diags: &mut Diags,
) -> ColumnSpec {
    let mut spec = ColumnSpec {
        nullable: is_option_type(&field.ty),
        ..ColumnSpec::default()
    };
    let field_name = field
        .ident
        .as_ref()
        .map_or_else(|| "<unnamed>".to_string(), std::string::ToString::to_string);
    let field_desc = format!("{table_desc}.{field_name}");
    let mut args = MySqlArgs::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("column") {
            continue;
        }
        let Meta::List(list) = &attr.meta else {
            diags
                .errors
                .push(format!("{field_desc}: #[column] requires arguments"));
            continue;
        };
        let metas =
            match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone()) {
                Ok(metas) => metas,
                Err(err) => {
                    diags.errors.push(format!(
                        "{field_desc}: failed to parse MySQL column attribute: {err}"
                    ));
                    continue;
                }
            };

        for meta in metas {
            let Some(ident) = meta.path().get_ident() else {
                diags.errors.push(format!(
                    "{field_desc}: expected a MySQL column attribute name"
                ));
                continue;
            };
            let name = ident.to_string();
            let upper = name.to_ascii_uppercase();
            match meta {
                Meta::Path(_path) => match upper.as_str() {
                    "PRIMARY" | "PRIMARY_KEY" => args.primary = true,
                    "UNIQUE" => args.unique = true,
                    "NOT_NULL" => args.not_null = true,
                    "AUTO_INCREMENT" => args.autoincrement = true,
                    "AUTOINCREMENT" => diags.errors.push(format!(
                        "{field_desc}: AUTOINCREMENT is SQLite-only; MySQL uses AUTO_INCREMENT"
                    )),
                    "ENUM" => {
                        if args.explicit_type.is_some() {
                            diags.errors.push(format!(
                                "{field_desc}: MySQL inline ENUM uses #[column(ENUM)] without a second SQL type override"
                            ));
                        }
                        args.enum_marker = true;
                    }
                    "JSON" => {
                        set_mysql_explicit_type(&mut args, MySQLType::Json, &field_desc, diags)
                    }
                    "JSONB" => diags.errors.push(format!(
                        "{field_desc}: JSONB is PostgreSQL/SQLite-only; MySQL uses JSON"
                    )),
                    "SERIAL" | "BIGSERIAL" | "SMALLSERIAL" | "IDENTITY" | "PGENUM" => {
                        diags.errors.push(format!(
                            "{field_desc}: {upper} is PostgreSQL-only; use a MySQL integer with AUTO_INCREMENT or inline ENUM"
                        ));
                    }
                    "DEFERRABLE" | "INITIALLY_DEFERRED" => diags.errors.push(format!(
                        "{field_desc}: MySQL foreign keys are not deferrable"
                    )),
                    _ => match MySQLType::from_attribute_name(&upper) {
                        Some(ty) => set_mysql_explicit_type(&mut args, ty, &field_desc, diags),
                        None => diags.errors.push(format!(
                            "{field_desc}: unrecognized MySQL column attribute `{name}`"
                        )),
                    },
                },
                Meta::NameValue(value) => {
                    let raw_value = spanned_source(source, &value.value);
                    args.named_values.push((name.clone(), raw_value.clone()));
                    match upper.as_str() {
                        "NAME" => {
                            args.name = mysql_string_value(&value.value, &field_desc, "NAME", diags)
                        }
                        "DEFAULT" => {
                            let default = default_from_expr(&value.value, source);
                            if matches!(default, ParsedDefault::Unsupported(_)) {
                                diags.errors.push(format!(
                                    "{field_desc}: DEFAULT requires a string, number, boolean, or character literal; use DEFAULT_SQL for SQL expressions"
                                ));
                            } else {
                                args.default = Some(default);
                            }
                        }
                        "DEFAULT_SQL" => {
                            args.default_sql =
                                mysql_string_value(&value.value, &field_desc, "DEFAULT_SQL", diags);
                        }
                        "DEFAULT_FN" => args.default_fn = true,
                        "CHECK" => {
                            args.check =
                                mysql_string_value(&value.value, &field_desc, "CHECK", diags)
                        }
                        "REFERENCES" => {
                            args.references = reference_from_expr(&value.value);
                            if args.references.is_none() {
                                diags.errors.push(format!(
                                    "{field_desc}: REFERENCES must be in the format Table::column"
                                ));
                            }
                        }
                        "RELATION" => {
                            args.relation =
                                mysql_string_value(&value.value, &field_desc, "RELATION", diags);
                            if let Some(relation) = &args.relation
                                && syn::parse_str::<Ident>(relation).is_err()
                            {
                                diags.errors.push(format!(
                                    "{field_desc}: RELATION = \"{relation}\" must be a valid Rust identifier"
                                ));
                            }
                        }
                        "ON_DELETE" => {
                            let action = if let Expr::Path(path) = &value.value {
                                path.path.get_ident().map(ToString::to_string)
                            } else {
                                None
                            };
                            match action.and_then(|action| mysql_referential_action(&action)) {
                                Some(action) => {
                                    args.on_delete_raw = Some(raw_value);
                                    args.on_delete = Some(action);
                                }
                                None => diags.errors.push(format!(
                                    "{field_desc}: ON_DELETE expects CASCADE, SET_NULL, RESTRICT, or NO_ACTION"
                                )),
                            }
                        }
                        "ON_UPDATE" => {
                            if let Expr::Path(path) = &value.value {
                                let action = path.path.get_ident().map(ToString::to_string);
                                match action.and_then(|action| mysql_referential_action(&action)) {
                                    Some(action) => {
                                        args.on_update_raw = Some(raw_value);
                                        args.on_update = Some(action);
                                    }
                                    None => diags.errors.push(format!(
                                        "{field_desc}: ON_UPDATE as an identifier expects CASCADE, SET_NULL, RESTRICT, or NO_ACTION; use a string for a timestamp expression"
                                    )),
                                }
                            } else {
                                args.mysql_on_update = mysql_string_value(
                                    &value.value,
                                    &field_desc,
                                    "ON_UPDATE",
                                    diags,
                                );
                            }
                        }
                        "CHARSET" | "CHARACTER_SET" => {
                            args.charset = mysql_string_value(
                                &value.value,
                                &field_desc,
                                "CHARACTER_SET",
                                diags,
                            );
                        }
                        "COLLATE" => {
                            args.collate =
                                mysql_string_value(&value.value, &field_desc, "COLLATE", diags)
                        }
                        "COMMENT" => {
                            args.comment =
                                mysql_string_value(&value.value, &field_desc, "COMMENT", diags)
                        }
                        _ => diags.errors.push(format!(
                            "{field_desc}: unrecognized MySQL column attribute `{name}`"
                        )),
                    }
                }
                Meta::List(list) => match upper.as_str() {
                    "GENERATED" => {
                        args.generated = mysql_parse_generated(&list, &field_desc, diags)
                    }
                    "SET" => {
                        if let Some(values) = mysql_parse_set_values(&list, &field_desc, diags) {
                            set_mysql_explicit_type(
                                &mut args,
                                MySQLType::set_values(values.clone()),
                                &field_desc,
                                diags,
                            );
                            args.set_values = Some(values);
                        }
                    }
                    _ => match MySQLType::from_attribute_name(&upper) {
                        Some(ty) => {
                            args.type_args = mysql_parse_type_args(&list, &field_desc, diags);
                            set_mysql_explicit_type(&mut args, ty, &field_desc, diags);
                        }
                        None => diags.errors.push(format!(
                            "{field_desc}: unrecognized MySQL column attribute `{name}`"
                        )),
                    },
                },
            }
        }
    }

    if (args.on_delete_raw.is_some() || args.on_update_raw.is_some()) && args.references.is_none() {
        diags.errors.push(format!(
            "{field_desc}: on_delete/on_update require a `references = Table::column` attribute"
        ));
    }
    if args.relation.is_some() && args.references.is_none() {
        diags.errors.push(format!(
            "{field_desc}: relation requires a `references = Table::column` attribute"
        ));
    }
    if args.not_null && spec.nullable {
        diags.errors.push(format!(
            "{field_desc}: Option<T> cannot be combined with NOT_NULL"
        ));
    }
    if args.primary && spec.nullable {
        diags.errors.push(format!(
            "{field_desc}: MySQL primary-key fields cannot be nullable; remove Option<T>"
        ));
    }
    if args.default.is_some() && args.default_fn {
        diags.errors.push(format!(
            "{field_desc}: DEFAULT/DEFAULT_SQL and DEFAULT_FN are mutually exclusive"
        ));
    }
    if args.default_sql.is_some() && args.default_fn {
        diags.errors.push(format!(
            "{field_desc}: DEFAULT/DEFAULT_SQL and DEFAULT_FN are mutually exclusive"
        ));
    }

    if args.enum_marker && args.explicit_type.is_some() {
        diags.errors.push(format!(
            "{field_desc}: MySQL inline ENUM uses #[column(ENUM)] without a second SQL type override"
        ));
    }

    let base_type = unwrap_option(&field.ty);
    let (mysql_type, type_args) = if args.enum_marker {
        (MySQLType::enum_values(Vec::<String>::new()), Vec::new())
    } else if let Some(ty) = args.explicit_type.take() {
        if args.set_values.is_none() {
            mysql_validate_explicit_signedness(base_type, &ty, &field_desc, diags);
        }
        (ty, args.type_args.clone())
    } else if let Some(inferred) = mysql_inferred_type(base_type) {
        inferred
    } else {
        spec.is_custom_type = true;
        diags.errors.push(format!(
            "{field_desc}: unsupported Rust type `{}` for MySQL; add #[derive(MySQLEnum)] and #[column(ENUM)], or specify a supported MySQL type",
            type_token_string(base_type)
        ));
        (MySQLType::Text, Vec::new())
    };
    if let Some(message) = mysql_validate_type_args(&mysql_type, &type_args) {
        diags.errors.push(format!("{field_desc}: {message}"));
    }
    if (args.charset.is_some() || args.collate.is_some())
        && !mysql_supports_character_options(&mysql_type)
    {
        diags.errors.push(format!(
            "{field_desc}: CHARACTER_SET/CHARSET and COLLATE apply only to MySQL character, text, ENUM, or SET columns"
        ));
    }
    if args.mysql_on_update.is_some()
        && !matches!(mysql_type, MySQLType::Datetime | MySQLType::Timestamp)
    {
        diags.errors.push(format!(
            "{field_desc}: column ON_UPDATE applies only to MySQL DATETIME or TIMESTAMP columns"
        ));
    }
    if args.autoincrement {
        if !mysql_type.supports_auto_increment() {
            diags.errors.push(format!(
                "{field_desc}: AUTO_INCREMENT requires a signed or unsigned MySQL integer column"
            ));
        }
        if args.generated.is_some() || args.default.is_some() || args.default_sql.is_some() {
            diags.errors.push(format!(
                "{field_desc}: AUTO_INCREMENT cannot be combined with DEFAULT or GENERATED"
            ));
        }
        if !(args.primary || args.unique) {
            diags.errors.push(format!(
                "{field_desc}: MySQL requires an AUTO_INCREMENT column to be keyed; add PRIMARY or UNIQUE"
            ));
        }
    }
    if args.generated.is_some()
        && (args.default.is_some()
            || args.default_sql.is_some()
            || args.default_fn
            || args.mysql_on_update.is_some())
    {
        diags.errors.push(format!(
            "{field_desc}: GENERATED columns cannot use DEFAULT, DEFAULT_FN, or ON_UPDATE"
        ));
    }

    spec.primary = args.primary;
    spec.unique = args.unique;
    spec.not_null = args.not_null;
    spec.autoincrement = args.autoincrement;
    spec.enum_marker = args.enum_marker;
    spec.default = args.default;
    spec.default_sql = args.default_sql;
    spec.has_default_fn = args.default_fn;
    spec.generated = args.generated;
    spec.check = args.check;
    spec.references = args.references;
    spec.on_delete = args.on_delete;
    spec.on_update = args.on_update;
    spec.on_delete_raw = args.on_delete_raw;
    spec.on_update_raw = args.on_update_raw;
    spec.explicit_name = args.name;
    spec.relation = args.relation;
    spec.charset = args.charset;
    spec.collate = args.collate;
    spec.mysql_on_update = args.mysql_on_update;
    spec.comment = args.comment;
    spec.named_values = args.named_values;
    spec.mysql_inline_enum = spec.enum_marker;
    spec.mysql_set_values = args.set_values;
    spec.mysql_type = Some(mysql_render_type(&mysql_type, &type_args));

    spec
}

// =============================================================================
// Postgres column attributes (mirrors postgres::field::parse_column_attribute)
// =============================================================================

#[allow(clippy::too_many_lines)]
pub(crate) fn postgres_column_spec(
    field: &syn::Field,
    source: &str,
    table_desc: &str,
    diags: &mut Diags,
) -> ColumnSpec {
    let mut spec = ColumnSpec {
        nullable: is_option_type(&field.ty),
        comment: doc_comment(&field.attrs),
        ..ColumnSpec::default()
    };
    let field_name = field
        .ident
        .as_ref()
        .map_or_else(|| "<unnamed>".to_string(), std::string::ToString::to_string);
    let field_desc = format!("{table_desc}.{field_name}");
    let base_type = unwrap_option(&field.ty).clone();

    // The Postgres macro only processes the *first* `#[column(...)]`
    // attribute (it breaks out of the attribute loop).
    let column_attr = field.attrs.iter().find(|attr| {
        attr.path()
            .get_ident()
            .is_some_and(|ident| ident == "column")
    });

    if let Some(attr) = column_attr
        && matches!(attr.meta, Meta::List(_))
    {
        let mut default_kind: Option<&'static str> = None;
        let result = attr.parse_nested_meta(|meta| {
            let Some(path_ident) = meta.path.get_ident() else {
                return Err(meta.error("expected a column attribute name in #[column(...)]"));
            };
            let key = path_ident.to_string();
            match key.to_ascii_uppercase().as_str() {
                "SERIAL" | "BIGSERIAL" | "SMALLSERIAL" => {
                    let (kind, required) = match key.to_ascii_uppercase().as_str() {
                        "SERIAL" => (SerialKind::Serial, "i32"),
                        "BIGSERIAL" => (SerialKind::Bigserial, "i64"),
                        _ => (SerialKind::Smallserial, "i16"),
                    };
                    if last_ident_is(&base_type, required) {
                        spec.serial = Some(kind);
                    } else {
                        diags.errors.push(format!(
                            "{field_desc}: #[column({})] requires the field type to be {required}",
                            key.to_ascii_lowercase()
                        ));
                    }
                }
                "PRIMARY" | "PRIMARY_KEY" => spec.primary = true,
                "UNIQUE" => spec.unique = true,
                "IDENTITY" => {
                    let mut identity = ParsedIdentity {
                        always: true,
                        ..ParsedIdentity::default()
                    };
                    if meta.input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in meta.input);
                        let mode_ident: Ident = content.parse()?;
                        match mode_ident.to_string().to_ascii_uppercase().as_str() {
                            "ALWAYS" => identity.always = true,
                            "BY_DEFAULT" => identity.always = false,
                            other => {
                                return Err(syn::Error::new_spanned(
                                    &mode_ident,
                                    format!(
                                        "expected `always` or `by_default` for identity(...), got `{other}`"
                                    ),
                                ));
                            }
                        }
                        while content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                            if content.is_empty() {
                                break;
                            }
                            let opt_key: Ident = content.parse()?;
                            match opt_key.to_string().to_ascii_lowercase().as_str() {
                                "cycle" => identity.cycle = true,
                                "start" | "start_with" => {
                                    content.parse::<Token![=]>()?;
                                    identity.start = Some(parse_signed_int(&content)?);
                                }
                                "increment" | "increment_by" => {
                                    content.parse::<Token![=]>()?;
                                    identity.increment = Some(parse_signed_int(&content)?);
                                }
                                "min" | "min_value" | "minvalue" => {
                                    content.parse::<Token![=]>()?;
                                    identity.min_value = Some(parse_signed_int(&content)?);
                                }
                                "max" | "max_value" | "maxvalue" => {
                                    content.parse::<Token![=]>()?;
                                    identity.max_value = Some(parse_signed_int(&content)?);
                                }
                                "cache" => {
                                    content.parse::<Token![=]>()?;
                                    let value = parse_signed_int(&content)?;
                                    identity.cache = Some(value.parse::<i32>().map_err(|_| {
                                        syn::Error::new_spanned(
                                            &opt_key,
                                            "identity cache must fit in i32",
                                        )
                                    })?);
                                }
                                other => {
                                    return Err(syn::Error::new_spanned(
                                        &opt_key,
                                        format!("unrecognized identity option `{other}`"),
                                    ));
                                }
                            }
                        }
                    }
                    spec.identity = Some(identity);
                }
                "GENERATED" => {
                    if meta.input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in meta.input);
                        let type_ident: Ident = content.parse()?;
                        let stored = match type_ident.to_string().to_ascii_lowercase().as_str() {
                            "stored" => true,
                            "virtual" => false,
                            other => {
                                return Err(syn::Error::new_spanned(
                                    &type_ident,
                                    format!(
                                        "expected `stored` or `virtual` for generated(...), got `{other}`"
                                    ),
                                ));
                            }
                        };
                        content.parse::<Token![,]>()?;
                        let expr_lit: Lit = content.parse()?;
                        let Lit::Str(s) = expr_lit else {
                            return Err(meta
                                .error("expected a string literal for the generated expression"));
                        };
                        spec.generated = Some(ParsedGenerated {
                            expression: s.value(),
                            stored,
                        });
                    } else {
                        return Err(meta.error(
                            "#[column(generated(...))] requires generated(stored|virtual, \"expr\")",
                        ));
                    }
                }
                "JSON" => spec.json = true,
                "JSONB" => spec.jsonb = true,
                "ENUM" => spec.enum_marker = true,
                "NAME" => {
                    meta.input.parse::<Token![=]>()?;
                    let lit: Lit = meta.input.parse()?;
                    let Lit::Str(s) = lit else {
                        return Err(meta.error("NAME requires a string literal"));
                    };
                    spec.named_values.push((key, format!("{:?}", s.value())));
                    spec.explicit_name = Some(s.value());
                }
                "COLLATE" => {
                    meta.input.parse::<Token![=]>()?;
                    if meta.input.peek(Lit) {
                        let lit: Lit = meta.input.parse()?;
                        let Lit::Str(s) = lit else {
                            return Err(
                                meta.error("COLLATE expects a string literal collation name")
                            );
                        };
                        spec.collate = Some(s.value());
                    } else {
                        // Bare idents are kept verbatim (the Postgres macro
                        // does not uppercase them, unlike SQLite).
                        let ident: Ident = meta.input.parse()?;
                        spec.collate = Some(ident.to_string());
                    }
                }
                "DEFAULT" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        if let Some(kind) = default_kind {
                            return Err(meta.error(format!("default conflicts with existing {kind}")));
                        }
                        let expr: Expr = meta.input.parse()?;
                        let raw_value = spanned_source(source, &expr);
                        spec.named_values.push((key, raw_value.clone()));
                        let default = default_from_expr(&expr, source);
                        if matches!(default, ParsedDefault::Unsupported(_)) {
                            return Err(meta.error(
                                "unsupported default value; expected a string, integer, float, \
                                 or boolean literal",
                            ));
                        }
                        spec.default = Some(default);
                        default_kind = Some("default");
                    }
                }
                "DEFAULT_FN" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        if let Some(kind) = default_kind {
                            return Err(
                                meta.error(format!("default_fn conflicts with existing {kind}"))
                            );
                        }
                        let expr: Expr = meta.input.parse()?;
                        spec.named_values
                            .push((key, spanned_source(source, &expr)));
                        spec.has_default_fn = true;
                        default_kind = Some("default_fn");
                    }
                }
                "DEFAULT_SQL" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        if let Some(kind) = default_kind {
                            return Err(
                                meta.error(format!("default_sql conflicts with existing {kind}"))
                            );
                        }
                        let lit: Lit = meta.input.parse()?;
                        let Lit::Str(s) = lit else {
                            return Err(meta.error("DEFAULT_SQL requires a string literal"));
                        };
                        spec.named_values.push((key, format!("{:?}", s.value())));
                        spec.default_sql = Some(s.value());
                        default_kind = Some("default_sql");
                    }
                }
                "CHECK" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        let lit: Lit = meta.input.parse()?;
                        if let Lit::Str(s) = lit {
                            spec.named_values.push((key, format!("{:?}", s.value())));
                            spec.check = Some(s.value());
                        }
                    }
                }
                "REFERENCES" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        let path: syn::ExprPath = meta.input.parse()?;
                        spec.named_values
                            .push((key, spanned_source(source, &path)));
                        if path.path.segments.len() == 2 {
                            spec.references = Some(ParsedReference {
                                table: path.path.segments.first().unwrap().ident.to_string(),
                                column: path.path.segments.last().unwrap().ident.to_string(),
                            });
                        } else {
                            return Err(
                                meta.error("References must be in the format Table::column")
                            );
                        }
                    }
                }
                "RELATION" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        let lit: Lit = meta.input.parse()?;
                        let Lit::Str(s) = lit else {
                            return Err(meta.error("relation requires a string literal"));
                        };
                        spec.named_values.push((key, format!("{:?}", s.value())));
                        spec.relation = Some(s.value());
                    }
                }
                "ON_DELETE" | "ON_UPDATE" => {
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        let action_ident: Ident = meta.input.parse()?;
                        let raw = action_ident.to_string();
                        spec.named_values.push((key.clone(), raw.clone()));
                        let Some(normalized) = normalize_referential_action(&raw) else {
                            return Err(syn::Error::new_spanned(
                                &action_ident,
                                format!("invalid referential action `{raw}`"),
                            ));
                        };
                        if spec.references.is_none() {
                            return Err(meta.error(format!(
                                "{key} requires a `references = Table::column` attribute first"
                            )));
                        }
                        if key.eq_ignore_ascii_case("on_delete") {
                            spec.on_delete = Some(normalized);
                            spec.on_delete_raw = Some(raw);
                        } else {
                            spec.on_update = Some(normalized);
                            spec.on_update_raw = Some(raw);
                        }
                    }
                }
                "DEFERRABLE" => {
                    if spec.references.is_none() {
                        return Err(
                            meta.error("deferrable requires a `references = Table::column` attribute")
                        );
                    }
                    spec.deferrable = true;
                }
                "INITIALLY_DEFERRED" => {
                    if spec.references.is_none() {
                        return Err(meta.error(
                            "initially_deferred requires a `references = Table::column` attribute",
                        ));
                    }
                    spec.deferrable = true;
                    spec.initially_deferred = true;
                }
                other => {
                    return Err(meta.error(format!("unknown #[column] attribute `{other}`")));
                }
            }
            Ok(())
        });
        if let Err(err) = result {
            diags.errors.push(format!("{field_desc}: {err}"));
        }
        if spec.has_default_fn && (spec.default.is_some() || spec.default_sql.is_some()) {
            diags.errors.push(format!(
                "{field_desc}: default/default_sql and default_fn are mutually exclusive"
            ));
        }
        if spec.default_sql.is_some() && (spec.identity.is_some() || spec.generated.is_some()) {
            diags.errors.push(format!(
                "{field_desc}: default_sql cannot be combined with identity or generated columns"
            ));
        }
        if spec.relation.is_some() && spec.references.is_none() {
            diags.errors.push(format!(
                "{field_desc}: relation requires a `references = Table::column` attribute"
            ));
        }
    }

    // Resolve the PostgreSQL type exactly like the macro's precedence chain.
    let (pg_type, dimensions): (String, Option<i32>) = if let Some(serial) = spec.serial {
        (
            match serial {
                SerialKind::Smallserial => "SMALLSERIAL",
                SerialKind::Serial => "SERIAL",
                SerialKind::Bigserial => "BIGSERIAL",
            }
            .to_string(),
            None,
        )
    } else if let Some(elem) = infer_postgres_array(&field.ty) {
        (elem.to_string(), Some(1))
    } else if spec.enum_marker {
        (type_token_string(&base_type), None)
    } else if spec.json {
        ("JSON".to_string(), None)
    } else if spec.jsonb {
        ("JSONB".to_string(), None)
    } else if let Some(sql) = infer_postgres_scalar_type(&base_type) {
        (sql.to_string(), None)
    } else {
        spec.is_custom_type = true;
        diags.warnings.push(format!(
            "{field_desc}: unknown Rust type `{}` — the macro defers the SQL type to the \
             DrizzlePostgresColumn trait; the parser records TEXT",
            type_token_string(&base_type)
        ));
        ("TEXT".to_string(), None)
    };
    spec.pg_type = Some(pg_type);
    spec.pg_dimensions = dimensions;

    spec
}

fn parse_signed_int(input: ParseStream) -> syn::Result<String> {
    let negative = input.parse::<Option<Token![-]>>()?.is_some();
    let lit: syn::LitInt = input.parse()?;
    let digits = lit.base10_digits();
    Ok(if negative {
        format!("-{digits}")
    } else {
        digits.to_string()
    })
}

// =============================================================================
// Table attributes (mirrors {sqlite,postgres}::table::attributes)
// =============================================================================

struct ReferencesArg {
    table: Ident,
    columns: Vec<Ident>,
}

fn parse_references_arg(tokens: proc_macro2::TokenStream) -> syn::Result<ReferencesArg> {
    let parser = |input: ParseStream| -> syn::Result<ReferencesArg> {
        let table: Ident = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        let columns: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(input)?;
        if columns.is_empty() {
            return Err(syn::Error::new(
                table.span(),
                "references(...) must include at least one target column",
            ));
        }
        Ok(ReferencesArg {
            table,
            columns: columns.into_iter().collect(),
        })
    };
    parser.parse2(tokens)
}

fn parse_ident_list(tokens: proc_macro2::TokenStream) -> syn::Result<Vec<Ident>> {
    let idents = Punctuated::<Ident, Token![,]>::parse_terminated.parse2(tokens)?;
    Ok(idents.into_iter().collect())
}

fn lit_str_value(expr: &Expr) -> Option<String> {
    if let Expr::Lit(syn::ExprLit {
        lit: Lit::Str(s), ..
    }) = expr
    {
        Some(s.value())
    } else {
        None
    }
}

fn mysql_option_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn parse_composite_fk(
    tokens: proc_macro2::TokenStream,
    dialect: Dialect,
    desc: &str,
    diags: &mut Diags,
) -> Option<CompositeFkSpec> {
    let metas = match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(tokens) {
        Ok(metas) => metas,
        Err(err) => {
            diags
                .errors
                .push(format!("{desc}: failed to parse FOREIGN_KEY(...): {err}"));
            return None;
        }
    };

    let mut fk = CompositeFkSpec::default();
    let mut have_source = false;
    let mut have_target = false;

    for meta in metas {
        match meta {
            Meta::List(list)
                if list
                    .path
                    .get_ident()
                    .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("columns")) =>
            {
                match parse_ident_list(list.tokens.clone()) {
                    Ok(cols) if !cols.is_empty() => {
                        fk.source_columns = cols.iter().map(Ident::to_string).collect();
                        have_source = true;
                    }
                    _ => diags.errors.push(format!(
                        "{desc}: FOREIGN_KEY columns(...) must list at least one source column"
                    )),
                }
            }
            Meta::List(list) if list.path.is_ident("references") => {
                match parse_references_arg(list.tokens.clone()) {
                    Ok(r) => {
                        fk.target_table = r.table.to_string();
                        fk.target_columns = r.columns.iter().map(Ident::to_string).collect();
                        have_target = true;
                    }
                    Err(err) => diags.errors.push(format!(
                        "{desc}: invalid FOREIGN_KEY references(...): {err}"
                    )),
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("on_delete") => {
                if let Some(value) = lit_str_value(&nv.value) {
                    if dialect == Dialect::MySQL {
                        match mysql_referential_action(&value) {
                            Some(action) => fk.on_delete = Some(action),
                            None => diags.errors.push(format!(
                                "{desc}: MySQL FOREIGN_KEY on_delete expects CASCADE, SET_NULL, RESTRICT, or NO_ACTION"
                            )),
                        }
                    } else {
                        fk.on_delete = Some(value);
                    }
                } else {
                    diags.errors.push(format!(
                        "{desc}: FOREIGN_KEY on_delete must be a string literal"
                    ));
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("on_update") => {
                if let Some(value) = lit_str_value(&nv.value) {
                    if dialect == Dialect::MySQL {
                        match mysql_referential_action(&value) {
                            Some(action) => fk.on_update = Some(action),
                            None => diags.errors.push(format!(
                                "{desc}: MySQL FOREIGN_KEY on_update expects CASCADE, SET_NULL, RESTRICT, or NO_ACTION"
                            )),
                        }
                    } else {
                        fk.on_update = Some(value);
                    }
                } else {
                    diags.errors.push(format!(
                        "{desc}: FOREIGN_KEY on_update must be a string literal"
                    ));
                }
            }
            Meta::Path(path) if dialect == Dialect::PostgreSQL && path.is_ident("deferrable") => {
                fk.deferrable = true;
            }
            Meta::Path(path)
                if dialect == Dialect::PostgreSQL && path.is_ident("initially_deferred") =>
            {
                fk.deferrable = true;
                fk.initially_deferred = true;
            }
            Meta::Path(path)
                if dialect == Dialect::MySQL
                    && (path.is_ident("deferrable") || path.is_ident("initially_deferred")) =>
            {
                diags
                    .errors
                    .push(format!("{desc}: MySQL foreign keys are not deferrable"));
            }
            other => diags.errors.push(format!(
                "{desc}: unrecognized FOREIGN_KEY argument `{}`",
                other.to_token_stream()
            )),
        }
    }

    if !have_source || !have_target {
        diags.errors.push(format!(
            "{desc}: FOREIGN_KEY requires columns(...) and references(...) arguments"
        ));
        return None;
    }
    if fk.source_columns.len() != fk.target_columns.len() {
        diags.errors.push(format!(
            "{desc}: FOREIGN_KEY columns(...) and references(...) must have the same number of \
             columns"
        ));
        return None;
    }

    Some(fk)
}

fn parse_table_unique(
    tokens: proc_macro2::TokenStream,
    dialect: Dialect,
    desc: &str,
    diags: &mut Diags,
) -> Option<TableUniqueSpec> {
    let metas = match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(tokens) {
        Ok(metas) => metas,
        Err(err) => {
            diags
                .errors
                .push(format!("{desc}: failed to parse UNIQUE(...): {err}"));
            return None;
        }
    };

    let mut unique = TableUniqueSpec::default();
    let mut columns_from_list: Option<Vec<String>> = None;
    let mut direct_columns: Vec<String> = Vec::new();

    for meta in metas {
        match meta {
            Meta::List(list)
                if list
                    .path
                    .get_ident()
                    .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("columns")) =>
            {
                match parse_ident_list(list.tokens.clone()) {
                    Ok(cols) if !cols.is_empty() => {
                        columns_from_list = Some(cols.iter().map(Ident::to_string).collect());
                    }
                    _ => diags.errors.push(format!(
                        "{desc}: UNIQUE columns(...) must list at least one column"
                    )),
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") || nv.path.is_ident("NAME") => {
                if let Some(value) = lit_str_value(&nv.value) {
                    unique.name = Some(value);
                } else {
                    diags
                        .errors
                        .push(format!("{desc}: UNIQUE name must be a string literal"));
                }
            }
            Meta::Path(path)
                if dialect == Dialect::PostgreSQL && path.is_ident("nulls_not_distinct") =>
            {
                unique.nulls_not_distinct = true;
            }
            Meta::Path(path) if dialect == Dialect::PostgreSQL && path.is_ident("deferrable") => {
                unique.deferrable = true;
            }
            Meta::Path(path)
                if dialect == Dialect::PostgreSQL && path.is_ident("initially_deferred") =>
            {
                unique.deferrable = true;
                unique.initially_deferred = true;
            }
            Meta::Path(path)
                if dialect == Dialect::MySQL
                    && (path.is_ident("nulls_not_distinct")
                        || path.is_ident("deferrable")
                        || path.is_ident("initially_deferred")) =>
            {
                diags.errors.push(format!(
                    "{desc}: this PostgreSQL UNIQUE option is not supported by MySQL"
                ));
            }
            Meta::Path(path) => {
                if let Some(ident) = path.get_ident() {
                    direct_columns.push(ident.to_string());
                } else {
                    diags
                        .errors
                        .push(format!("{desc}: UNIQUE(...) columns must be identifiers"));
                }
            }
            other => diags.errors.push(format!(
                "{desc}: unrecognized UNIQUE argument `{}`",
                other.to_token_stream()
            )),
        }
    }

    unique.columns = columns_from_list.unwrap_or(direct_columns);
    if unique.columns.is_empty() {
        diags
            .errors
            .push(format!("{desc}: UNIQUE requires at least one column"));
        return None;
    }
    Some(unique)
}

fn parse_table_check(
    tokens: proc_macro2::TokenStream,
    desc: &str,
    diags: &mut Diags,
) -> Option<TableCheckSpec> {
    let metas = match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(tokens) {
        Ok(metas) => metas,
        Err(err) => {
            diags
                .errors
                .push(format!("{desc}: failed to parse CHECK(...): {err}"));
            return None;
        }
    };

    let mut check = TableCheckSpec::default();
    let mut have_expr = false;

    for meta in metas {
        match meta {
            Meta::NameValue(nv) if nv.path.is_ident("name") || nv.path.is_ident("NAME") => {
                if let Some(value) = lit_str_value(&nv.value) {
                    check.name = Some(value);
                } else {
                    diags
                        .errors
                        .push(format!("{desc}: CHECK name must be a string literal"));
                }
            }
            Meta::NameValue(nv)
                if nv.path.is_ident("expr")
                    || nv.path.is_ident("EXPR")
                    || nv.path.is_ident("value")
                    || nv.path.is_ident("VALUE") =>
            {
                if let Some(value) = lit_str_value(&nv.value) {
                    check.expr = value;
                    have_expr = true;
                } else {
                    diags
                        .errors
                        .push(format!("{desc}: CHECK expr must be a string literal"));
                }
            }
            other => diags.errors.push(format!(
                "{desc}: unrecognized CHECK argument `{}`",
                other.to_token_stream()
            )),
        }
    }

    if !have_expr {
        diags
            .errors
            .push(format!("{desc}: CHECK requires expr = \"...\""));
        return None;
    }
    Some(check)
}

/// Interpret a `#[SQLiteTable(...)]` / `#[PostgresTable(...)]` /
/// `#[MySQLTable(...)]` attribute.
pub(crate) fn table_spec(
    attr: &Attribute,
    dialect: Dialect,
    item_attrs: &[Attribute],
    desc: &str,
    diags: &mut Diags,
) -> TableSpec {
    let mut spec = TableSpec {
        comment: doc_comment(item_attrs),
        ..TableSpec::default()
    };

    let Meta::List(list) = &attr.meta else {
        return spec;
    };

    let metas = match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone()) {
        Ok(metas) => metas,
        Err(err) => {
            diags
                .errors
                .push(format!("{desc}: failed to parse table attribute: {err}"));
            return spec;
        }
    };

    for meta in metas {
        match &meta {
            Meta::NameValue(nv) => {
                if let Some(ident) = nv.path.get_ident() {
                    let key = ident.to_string();
                    match key.to_ascii_uppercase().as_str() {
                        "NAME" => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                spec.explicit_name = Some(value);
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: NAME requires a string literal"));
                            continue;
                        }
                        "SCHEMA" if dialect == Dialect::PostgreSQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                spec.schema = Some(value);
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: SCHEMA requires a string literal"));
                            continue;
                        }
                        "DATABASE" | "SCHEMA" if dialect == Dialect::MySQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                spec.database = Some(value);
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: DATABASE requires a string literal"));
                            continue;
                        }
                        "INHERITS" if dialect == Dialect::PostgreSQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                spec.inherits = Some(value);
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: INHERITS requires a string literal"));
                            continue;
                        }
                        "TABLESPACE" if dialect == Dialect::PostgreSQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                spec.tablespace = Some(value);
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: TABLESPACE requires a string literal"));
                            continue;
                        }
                        "ENGINE" if dialect == Dialect::MySQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                if mysql_option_identifier(&value) {
                                    spec.engine = Some(value);
                                } else {
                                    diags.errors.push(format!(
                                        "{desc}: ENGINE must contain only ASCII letters, digits, or underscores"
                                    ));
                                }
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: ENGINE requires a string literal"));
                            continue;
                        }
                        "CHARSET" | "DEFAULT_CHARSET" if dialect == Dialect::MySQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                if mysql_option_identifier(&value) {
                                    spec.charset = Some(value);
                                } else {
                                    diags.errors.push(format!(
                                        "{desc}: CHARSET must contain only ASCII letters, digits, or underscores"
                                    ));
                                }
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: CHARSET requires a string literal"));
                            continue;
                        }
                        "COLLATE" if dialect == Dialect::MySQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                if mysql_option_identifier(&value) {
                                    spec.collate = Some(value);
                                } else {
                                    diags.errors.push(format!(
                                        "{desc}: COLLATE must contain only ASCII letters, digits, or underscores"
                                    ));
                                }
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: COLLATE requires a string literal"));
                            continue;
                        }
                        "COMMENT" if dialect == Dialect::MySQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                spec.named_values.push((key, format!("{value:?}")));
                                spec.comment = Some(value);
                                continue;
                            }
                            diags
                                .errors
                                .push(format!("{desc}: COMMENT requires a string literal"));
                            continue;
                        }
                        // `crate = "..."` only affects generated code paths.
                        "CRATE" if dialect == Dialect::SQLite => continue,
                        _ => {}
                    }
                }
                diags.errors.push(format!(
                    "{desc}: unrecognized table attribute `{}`",
                    meta.to_token_stream()
                ));
            }
            Meta::Path(path) => {
                if let Some(ident) = path.get_ident() {
                    match ident.to_string().to_ascii_uppercase().as_str() {
                        "STRICT" if dialect == Dialect::SQLite => {
                            spec.strict = true;
                            continue;
                        }
                        "WITHOUT_ROWID" if dialect == Dialect::SQLite => {
                            spec.without_rowid = true;
                            continue;
                        }
                        "UNLOGGED" if dialect == Dialect::PostgreSQL => {
                            spec.unlogged = true;
                            continue;
                        }
                        "TEMPORARY"
                            if dialect == Dialect::PostgreSQL || dialect == Dialect::MySQL =>
                        {
                            spec.temporary = true;
                            continue;
                        }
                        "RLS" if dialect == Dialect::PostgreSQL => {
                            spec.rls = true;
                            continue;
                        }
                        _ => {}
                    }
                }
                diags.errors.push(format!(
                    "{desc}: unrecognized table attribute `{}`",
                    meta.to_token_stream()
                ));
            }
            Meta::List(inner) => {
                if let Some(ident) = inner.path.get_ident() {
                    match ident.to_string().to_ascii_uppercase().as_str() {
                        "FOREIGN_KEY" => {
                            if let Some(fk) =
                                parse_composite_fk(inner.tokens.clone(), dialect, desc, diags)
                            {
                                spec.composite_fks.push(fk);
                            }
                            continue;
                        }
                        "UNIQUE" => {
                            if let Some(unique) =
                                parse_table_unique(inner.tokens.clone(), dialect, desc, diags)
                            {
                                spec.unique_constraints.push(unique);
                            }
                            continue;
                        }
                        "CHECK" => {
                            if let Some(check) =
                                parse_table_check(inner.tokens.clone(), desc, diags)
                            {
                                spec.check_constraints.push(check);
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
                diags.errors.push(format!(
                    "{desc}: unrecognized table attribute `{}`",
                    meta.to_token_stream()
                ));
            }
        }
    }

    spec
}

// =============================================================================
// Index attributes (mirrors {sqlite,postgres}::index)
// =============================================================================

/// Interpret `#[SQLiteIndex(...)]` / `#[PostgresIndex(...)]` and the tuple
/// struct's column references.
pub(crate) fn index_spec(
    attr: &Attribute,
    item: &syn::ItemStruct,
    dialect: Dialect,
    desc: &str,
    diags: &mut Diags,
) -> IndexSpec {
    let mut spec = IndexSpec::default();

    // Index arguments are parsed with a keyword-tolerant item grammar
    // (`Ident::parse_any`) because `where` is a Rust keyword that plain
    // `Meta` path parsing rejects.
    enum IndexArg {
        Flag(String),
        NameValue(String, Lit),
    }
    fn parse_index_arg(input: ParseStream) -> syn::Result<IndexArg> {
        let ident = Ident::parse_any(input)?;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let lit: Lit = input.parse()?;
            Ok(IndexArg::NameValue(ident.to_string(), lit))
        } else {
            Ok(IndexArg::Flag(ident.to_string()))
        }
    }
    let str_of = |lit: &Lit| -> Option<String> {
        if let Lit::Str(s) = lit {
            Some(s.value())
        } else {
            None
        }
    };

    if let Meta::List(list) = &attr.meta {
        let parser = |input: ParseStream| {
            Punctuated::<IndexArg, Token![,]>::parse_terminated_with(input, parse_index_arg)
        };
        match parser.parse2(list.tokens.clone()) {
            Ok(args) => {
                for arg in args {
                    match arg {
                        IndexArg::Flag(name) if name == "unique" => spec.unique = true,
                        IndexArg::Flag(name)
                            if dialect == Dialect::PostgreSQL && name == "concurrent" =>
                        {
                            spec.concurrent = true;
                        }
                        IndexArg::NameValue(key, lit)
                            if dialect == Dialect::PostgreSQL && key == "method" =>
                        {
                            match str_of(&lit).as_deref() {
                                Some(
                                    method
                                    @ ("btree" | "hash" | "gin" | "gist" | "spgist" | "brin"),
                                ) => spec.method = Some(method.to_string()),
                                Some(other) => diags.errors.push(format!(
                                    "{desc}: invalid index method `{other}`; supported: btree, \
                                     hash, gin, gist, spgist, brin"
                                )),
                                None => diags.errors.push(format!(
                                    "{desc}: expected string literal for index method"
                                )),
                            }
                        }
                        IndexArg::NameValue(key, lit)
                            if dialect == Dialect::MySQL
                                && key.eq_ignore_ascii_case("using") =>
                        {
                            match str_of(&lit).map(|value| value.to_ascii_lowercase()) {
                                Some(value) if matches!(value.as_str(), "btree" | "hash") => {
                                    if spec.mysql_using.replace(value).is_some() {
                                        diags.errors.push(format!(
                                            "{desc}: MySQL index accepts `using` only once"
                                        ));
                                    }
                                }
                                Some(other) => diags.errors.push(format!(
                                    "{desc}: invalid MySQL index `using` `{other}`; supported: btree, hash"
                                )),
                                None => diags.errors.push(format!(
                                    "{desc}: MySQL index `using` expects a string literal"
                                )),
                            }
                        }
                        IndexArg::NameValue(key, lit)
                            if dialect == Dialect::MySQL
                                && key.eq_ignore_ascii_case("algorithm") =>
                        {
                            match str_of(&lit).map(|value| value.to_ascii_lowercase()) {
                                Some(value)
                                    if matches!(value.as_str(), "default" | "inplace" | "copy") =>
                                {
                                    if spec.mysql_algorithm.replace(value).is_some() {
                                        diags.errors.push(format!(
                                            "{desc}: MySQL index accepts `algorithm` only once"
                                        ));
                                    }
                                }
                                Some(other) => diags.errors.push(format!(
                                    "{desc}: invalid MySQL index `algorithm` `{other}`; supported: default, inplace, copy"
                                )),
                                None => diags.errors.push(format!(
                                    "{desc}: MySQL index `algorithm` expects a string literal"
                                )),
                            }
                        }
                        IndexArg::NameValue(key, lit)
                            if dialect == Dialect::MySQL && key.eq_ignore_ascii_case("lock") =>
                        {
                            match str_of(&lit).map(|value| value.to_ascii_lowercase()) {
                                Some(value)
                                    if matches!(
                                        value.as_str(),
                                        "default" | "none" | "shared" | "exclusive"
                                    ) =>
                                {
                                    if spec.mysql_lock.replace(value).is_some() {
                                        diags.errors.push(format!(
                                            "{desc}: MySQL index accepts `lock` only once"
                                        ));
                                    }
                                }
                                Some(other) => diags.errors.push(format!(
                                    "{desc}: invalid MySQL index `lock` `{other}`; supported: default, none, shared, exclusive"
                                )),
                                None => diags.errors.push(format!(
                                    "{desc}: MySQL index `lock` expects a string literal"
                                )),
                            }
                        }
                        IndexArg::NameValue(key, lit)
                            if dialect == Dialect::PostgreSQL && key == "tablespace" =>
                        {
                            if let Some(value) = str_of(&lit) {
                                spec.tablespace = Some(value);
                            } else {
                                diags.errors.push(format!(
                                    "{desc}: expected string literal for index tablespace"
                                ));
                            }
                        }
                        IndexArg::NameValue(key, lit)
                            if matches!(dialect, Dialect::PostgreSQL | Dialect::SQLite)
                                && key == "where" =>
                        {
                            if let Some(value) = str_of(&lit) {
                                if value.trim().is_empty() {
                                    diags.errors.push(format!(
                                        "{desc}: partial-index predicate cannot be empty"
                                    ));
                                } else if spec.where_clause.is_some() {
                                    diags.errors.push(format!(
                                        "{desc}: index accepts only one partial-index predicate"
                                    ));
                                } else {
                                    spec.where_clause = Some(value);
                                }
                            } else {
                                diags.errors.push(format!(
                                    "{desc}: expected string literal for index where clause"
                                ));
                            }
                        }
                        IndexArg::NameValue(key, _)
                            if dialect == Dialect::MySQL && key == "where" =>
                        {
                            diags.errors.push(format!(
                                "{desc}: MySQL does not support partial indexes; remove `where = ...`"
                            ));
                        }
                        IndexArg::NameValue(key, lit) if key.eq_ignore_ascii_case("name") => {
                            match str_of(&lit) {
                                Some(value) if value.trim().is_empty() => diags
                                    .errors
                                    .push(format!("{desc}: index `name` cannot be empty")),
                                Some(value) => {
                                    if spec.explicit_name.replace(value).is_some() {
                                        diags.errors.push(format!(
                                            "{desc}: index accepts `name` only once"
                                        ));
                                    }
                                }
                                None => diags.errors.push(format!(
                                    "{desc}: index `name` expects a string literal"
                                )),
                            }
                        }
                        IndexArg::Flag(name) => diags
                            .errors
                            .push(format!("{desc}: unrecognized index attribute `{name}`")),
                        IndexArg::NameValue(key, _) => diags
                            .errors
                            .push(format!("{desc}: unrecognized index attribute `{key}`")),
                    }
                }
            }
            Err(err) => diags
                .errors
                .push(format!("{desc}: failed to parse index attribute: {err}")),
        }
    }

    // Column references from the tuple struct fields.
    let syn::Fields::Unnamed(fields) = &item.fields else {
        diags.errors.push(format!(
            "{desc}: index must be a tuple struct of `Table::column` references"
        ));
        return spec;
    };
    for field in &fields.unnamed {
        let Type::Path(type_path) = &field.ty else {
            diags.errors.push(format!(
                "{desc}: index columns must be `Table::column` path references"
            ));
            continue;
        };
        let segments = &type_path.path.segments;
        if segments.len() < 2 {
            diags.errors.push(format!(
                "{desc}: index columns must be in the format Table::column"
            ));
            continue;
        }
        let table = segments.first().unwrap().ident.to_string();
        let column = segments.last().unwrap().ident.to_string();
        spec.column_refs.push((table, column));
    }

    if spec.column_refs.is_empty() {
        diags
            .errors
            .push(format!("{desc}: index must have at least one column"));
    } else {
        let first_table = &spec.column_refs[0].0;
        if spec
            .column_refs
            .iter()
            .any(|(table, _)| table != first_table)
        {
            diags.errors.push(format!(
                "{desc}: all columns in an index must belong to the same table"
            ));
        }
    }
    spec
}

// =============================================================================
// View attributes (mirrors {sqlite,postgres,mysql}::view::ViewAttributes)
// =============================================================================

pub(crate) struct ViewAttrData {
    pub name: Option<String>,
    pub schema: Option<String>,
    pub database: Option<String>,
    pub definition: Option<String>,
    pub has_opaque_definition: bool,
    pub materialized: bool,
    pub existing: bool,
    pub with_no_data: bool,
    pub using: Option<String>,
    pub tablespace: Option<String>,
    pub mysql_algorithm: Option<String>,
    pub mysql_sql_security: Option<String>,
    pub mysql_check_option: Option<String>,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn view_spec(
    attr: &Attribute,
    dialect: Dialect,
    desc: &str,
    diags: &mut Diags,
) -> ViewAttrData {
    let mut saw_definition = false;
    let mut saw_query = false;
    let mut data = ViewAttrData {
        name: None,
        schema: None,
        database: None,
        definition: None,
        has_opaque_definition: false,
        materialized: false,
        existing: false,
        with_no_data: false,
        using: None,
        tablespace: None,
        mysql_algorithm: None,
        mysql_sql_security: None,
        mysql_check_option: None,
    };

    let Meta::List(list) = &attr.meta else {
        return data;
    };
    let metas = match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone()) {
        Ok(metas) => metas,
        Err(err) => {
            diags
                .errors
                .push(format!("{desc}: failed to parse view attribute: {err}"));
            return data;
        }
    };

    for meta in metas {
        match &meta {
            Meta::NameValue(nv) => {
                if let Some(ident) = nv.path.get_ident() {
                    match ident.to_string().to_ascii_uppercase().as_str() {
                        "NAME" => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                data.name = Some(value);
                            } else {
                                diags
                                    .errors
                                    .push(format!("{desc}: NAME requires a string literal"));
                            }
                            continue;
                        }
                        "SCHEMA" if dialect == Dialect::PostgreSQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                data.schema = Some(value);
                            } else {
                                diags
                                    .errors
                                    .push(format!("{desc}: SCHEMA requires a string literal"));
                            }
                            continue;
                        }
                        "DATABASE" | "SCHEMA" if dialect == Dialect::MySQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                data.database = Some(value);
                            } else {
                                diags
                                    .errors
                                    .push(format!("{desc}: DATABASE requires a string literal"));
                            }
                            continue;
                        }
                        "DEFINITION" => {
                            saw_definition = true;
                            if let Some(value) = lit_str_value(&nv.value) {
                                data.definition = Some(value);
                            } else {
                                data.has_opaque_definition = true;
                                diags.warnings.push(format!(
                                    "{desc}: view DEFINITION is an expression the parser cannot \
                                     evaluate; the view definition is omitted from the snapshot"
                                ));
                            }
                            continue;
                        }
                        "USING" if dialect == Dialect::PostgreSQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                data.using = Some(value);
                            } else {
                                diags
                                    .errors
                                    .push(format!("{desc}: USING requires a string literal"));
                            }
                            continue;
                        }
                        "TABLESPACE" if dialect == Dialect::PostgreSQL => {
                            if let Some(value) = lit_str_value(&nv.value) {
                                data.tablespace = Some(value);
                            } else {
                                diags
                                    .errors
                                    .push(format!("{desc}: TABLESPACE requires a string literal"));
                            }
                            continue;
                        }
                        "WITH" | "WITH_OPTIONS" if dialect == Dialect::PostgreSQL => {
                            diags.warnings.push(format!(
                                "{desc}: view WITH options are an expression the parser cannot \
                                 evaluate; they are omitted from the snapshot"
                            ));
                            continue;
                        }
                        "ALGORITHM" if dialect == Dialect::MySQL => {
                            let Some(value) = lit_str_value(&nv.value) else {
                                diags
                                    .errors
                                    .push(format!("{desc}: ALGORITHM requires a string literal"));
                                continue;
                            };
                            let value = value.to_ascii_lowercase().replace([' ', '-'], "_");
                            if matches!(
                                value.as_str(),
                                "undefined" | "merge" | "temptable" | "temp_table"
                            ) {
                                data.mysql_algorithm = Some(if value == "temp_table" {
                                    "temptable".to_string()
                                } else {
                                    value
                                });
                            } else {
                                diags.errors.push(format!(
                                    "{desc}: ALGORITHM must be UNDEFINED, MERGE, or TEMPTABLE"
                                ));
                            }
                            continue;
                        }
                        "SQL_SECURITY" | "SECURITY" if dialect == Dialect::MySQL => {
                            let Some(value) = lit_str_value(&nv.value) else {
                                diags.errors.push(format!(
                                    "{desc}: SQL_SECURITY requires a string literal"
                                ));
                                continue;
                            };
                            let value = value.to_ascii_lowercase().replace([' ', '-'], "_");
                            if matches!(value.as_str(), "definer" | "invoker") {
                                data.mysql_sql_security = Some(value);
                            } else {
                                diags.errors.push(format!(
                                    "{desc}: SQL_SECURITY must be DEFINER or INVOKER"
                                ));
                            }
                            continue;
                        }
                        "CHECK_OPTION" | "WITH_CHECK_OPTION" if dialect == Dialect::MySQL => {
                            let Some(value) = lit_str_value(&nv.value) else {
                                diags.errors.push(format!(
                                    "{desc}: CHECK_OPTION requires a string literal"
                                ));
                                continue;
                            };
                            let value = value.to_ascii_lowercase().replace([' ', '-'], "_");
                            if matches!(value.as_str(), "cascaded" | "local") {
                                data.mysql_check_option = Some(value);
                            } else {
                                diags.errors.push(format!(
                                    "{desc}: CHECK_OPTION must be CASCADED or LOCAL"
                                ));
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
                diags.errors.push(format!(
                    "{desc}: unrecognized view attribute `{}`",
                    meta.to_token_stream()
                ));
            }
            Meta::List(inner) => {
                if let Some(ident) = inner.path.get_ident()
                    && ident.to_string().eq_ignore_ascii_case("query")
                {
                    saw_query = true;
                    data.has_opaque_definition = true;
                    diags.warnings.push(format!(
                        "{desc}: view `query(...)` definitions resolve at compile time; the view \
                         definition is omitted from the snapshot"
                    ));
                    continue;
                }
                diags.errors.push(format!(
                    "{desc}: unrecognized view attribute `{}`",
                    meta.to_token_stream()
                ));
            }
            Meta::Path(path) => {
                if let Some(ident) = path.get_ident() {
                    match ident.to_string().to_ascii_uppercase().as_str() {
                        "MATERIALIZED" if dialect == Dialect::PostgreSQL => {
                            data.materialized = true;
                            continue;
                        }
                        "EXISTING" => {
                            data.existing = true;
                            continue;
                        }
                        "CHECK_OPTION" | "WITH_CHECK_OPTION" if dialect == Dialect::MySQL => {
                            data.mysql_check_option = Some("cascaded".to_string());
                            continue;
                        }
                        "WITH_NO_DATA" if dialect == Dialect::PostgreSQL => {
                            data.with_no_data = true;
                            continue;
                        }
                        _ => {}
                    }
                }
                diags.errors.push(format!(
                    "{desc}: unrecognized view attribute `{}`",
                    meta.to_token_stream()
                ));
            }
        }
    }

    if dialect == Dialect::MySQL {
        if saw_definition && saw_query {
            diags.errors.push(format!(
                "{desc}: DEFINITION and query(...) are mutually exclusive"
            ));
        }
        if data.existing && (saw_definition || saw_query) {
            diags.errors.push(format!(
                "{desc}: EXISTING cannot be combined with DEFINITION or query(...)"
            ));
        }
    }

    data
}

// =============================================================================
// Policy attributes (mirrors postgres::policy::PolicyAttributes)
// =============================================================================

pub(crate) struct PolicyAttrData {
    pub name: Option<String>,
    pub as_clause: Option<String>,
    pub for_clause: Option<String>,
    pub to: Vec<String>,
    pub using: Option<String>,
    pub with_check: Option<String>,
}

pub(crate) fn policy_spec(attr: &Attribute, desc: &str, diags: &mut Diags) -> PolicyAttrData {
    let mut data = PolicyAttrData {
        name: None,
        as_clause: None,
        for_clause: None,
        to: Vec::new(),
        using: None,
        with_check: None,
    };

    let Meta::List(list) = &attr.meta else {
        return data;
    };
    let metas = match Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone()) {
        Ok(metas) => metas,
        Err(err) => {
            diags
                .errors
                .push(format!("{desc}: failed to parse policy attribute: {err}"));
            return data;
        }
    };

    for meta in metas {
        match &meta {
            Meta::NameValue(nv) => {
                let Some(ident) = nv.path.get_ident() else {
                    diags
                        .errors
                        .push(format!("{desc}: expected policy attribute name"));
                    continue;
                };
                let Some(value) = lit_str_value(&nv.value) else {
                    diags.errors.push(format!(
                        "{desc}: policy attribute values must be string literals"
                    ));
                    continue;
                };
                match ident.to_string().to_ascii_uppercase().as_str() {
                    "NAME" => data.name = Some(value),
                    "AS" | "AS_CLAUSE" => {
                        let upper = value.to_ascii_uppercase();
                        if matches!(upper.as_str(), "PERMISSIVE" | "RESTRICTIVE") {
                            data.as_clause = Some(upper);
                        } else {
                            diags
                                .errors
                                .push(format!("{desc}: AS must be PERMISSIVE or RESTRICTIVE"));
                        }
                    }
                    "FOR" | "FOR_CLAUSE" => {
                        let upper = value.to_ascii_uppercase();
                        if matches!(
                            upper.as_str(),
                            "ALL" | "SELECT" | "INSERT" | "UPDATE" | "DELETE"
                        ) {
                            data.for_clause = Some(upper);
                        } else {
                            diags.errors.push(format!(
                                "{desc}: FOR must be ALL, SELECT, INSERT, UPDATE, or DELETE"
                            ));
                        }
                    }
                    "TO" => data.to.push(value),
                    "USING" => data.using = Some(value),
                    "WITH_CHECK" => data.with_check = Some(value),
                    other => diags.errors.push(format!(
                        "{desc}: unrecognized PostgresPolicy attribute `{other}`"
                    )),
                }
            }
            Meta::List(inner)
                if inner
                    .path
                    .get_ident()
                    .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("TO")) =>
            {
                #[allow(clippy::items_after_statements)]
                enum RoleArg {
                    Ident(Ident),
                    Str(String),
                }
                #[allow(clippy::items_after_statements)]
                fn parse_role(input: ParseStream) -> syn::Result<RoleArg> {
                    if input.peek(syn::LitStr) {
                        let lit: syn::LitStr = input.parse()?;
                        Ok(RoleArg::Str(lit.value()))
                    } else {
                        Ok(RoleArg::Ident(input.parse()?))
                    }
                }
                let parser = |input: ParseStream| {
                    Punctuated::<RoleArg, Token![,]>::parse_terminated_with(input, parse_role)
                };
                match parser.parse2(inner.tokens.clone()) {
                    Ok(roles) => {
                        for role in roles {
                            data.to.push(match role {
                                RoleArg::Ident(ident) => ident.to_string(),
                                RoleArg::Str(s) => s,
                            });
                        }
                    }
                    Err(err) => diags
                        .errors
                        .push(format!("{desc}: invalid TO(...) roles: {err}")),
                }
            }
            other => diags.errors.push(format!(
                "{desc}: unrecognized PostgresPolicy attribute `{}`",
                other.to_token_stream()
            )),
        }
    }

    data
}

// =============================================================================
// Tests — one section per attribute family
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_field(code: &str) -> syn::Field {
        let item: syn::ItemStruct =
            syn::parse_str(&format!("struct T {{ {code} }}")).expect("valid struct");
        item.fields.into_iter().next().expect("one field")
    }

    fn sqlite_spec(code: &str) -> (ColumnSpec, Diags) {
        let field = parse_field(code);
        let mut diags = Diags::new();
        let spec = sqlite_column_spec(&field, code, "T", &mut diags);
        (spec, diags)
    }

    fn pg_spec(code: &str) -> (ColumnSpec, Diags) {
        let field = parse_field(code);
        let mut diags = Diags::new();
        let spec = postgres_column_spec(&field, code, "T", &mut diags);
        (spec, diags)
    }

    fn mysql_spec(code: &str) -> (ColumnSpec, Diags) {
        let field = parse_field(code);
        let mut diags = Diags::new();
        let spec = mysql_column_spec(&field, code, "T", &mut diags);
        (spec, diags)
    }

    // ---- markers (case-insensitivity, P1) --------------------------------

    #[test]
    fn sqlite_markers_lowercase() {
        let (spec, _) = sqlite_spec("#[column(primary, autoincrement, unique)] id: i64");
        assert!(spec.primary && spec.autoincrement && spec.unique);
    }

    #[test]
    fn sqlite_markers_uppercase() {
        let (spec, _) = sqlite_spec("#[column(PRIMARY, AUTOINCREMENT, UNIQUE)] id: i64");
        assert!(spec.primary && spec.autoincrement && spec.unique);
    }

    #[test]
    fn sqlite_primary_key_spelling() {
        let (spec, _) = sqlite_spec("#[column(PRIMARY_KEY)] id: i64");
        assert!(spec.primary);
    }

    #[test]
    fn pg_markers_case_insensitive() {
        let (spec, _) = pg_spec("#[column(Primary, UNIQUE)] id: i32");
        assert!(spec.primary && spec.unique);
    }

    // ---- no phantom markers from name/strings (P2) -----------------------

    #[test]
    fn no_phantom_primary_from_name_value() {
        let (spec, _) = sqlite_spec(r##"#[column(name = "primary_email")] email: String"##);
        assert!(!spec.primary);
        assert_eq!(spec.explicit_name.as_deref(), Some("primary_email"));
    }

    #[test]
    fn no_phantom_unique_from_default_string() {
        let (spec, _) = sqlite_spec(r##"#[column(default = "unique snowflake")] v: String"##);
        assert!(!spec.unique);
    }

    // ---- referential actions (P3) ----------------------------------------

    #[test]
    fn sqlite_actions_normalized() {
        let (spec, _) = sqlite_spec(
            "#[column(references = Users::id, on_delete = SET_NULL, on_update = Cascade)] user_id: i64",
        );
        assert_eq!(spec.on_delete.as_deref(), Some("SET NULL"));
        assert_eq!(spec.on_update.as_deref(), Some("CASCADE"));
        assert_eq!(spec.on_delete_raw.as_deref(), Some("SET_NULL"));
    }

    #[test]
    fn sqlite_invalid_action_dropped_with_warning() {
        let (spec, diags) =
            sqlite_spec("#[column(references = Users::id, on_delete = Explode)] user_id: i64");
        assert!(spec.on_delete.is_none());
        assert!(!diags.warnings.is_empty());
    }

    #[test]
    fn pg_invalid_action_is_error() {
        let (_, diags) =
            pg_spec("#[column(references = Users::id, on_delete = Explode)] user_id: i32");
        assert!(!diags.errors.is_empty());
    }

    // ---- defaults (P4 groundwork) ----------------------------------------

    #[test]
    fn string_default_captured_unquoted() {
        let (spec, _) = sqlite_spec(r##"#[column(default = "hello")] v: String"##);
        assert_eq!(spec.default, Some(ParsedDefault::Str("hello".to_string())));
    }

    #[test]
    fn default_with_braces_in_string() {
        // `}` inside the string must not truncate anything (old P5 defect).
        let (spec, _) = sqlite_spec(r##"#[column(default = "{}")] v: String"##);
        assert_eq!(spec.default, Some(ParsedDefault::Str("{}".to_string())));
    }

    #[test]
    fn negative_default_matches_macro_drop() {
        let (spec, diags) = sqlite_spec("#[column(default = -1)] v: i64");
        assert!(matches!(spec.default, Some(ParsedDefault::Unsupported(_))));
        assert!(!diags.warnings.is_empty());
    }

    #[test]
    fn default_sql_captured() {
        let (spec, _) = sqlite_spec(r##"#[column(default_sql = "CURRENT_TIMESTAMP")] v: String"##);
        assert_eq!(spec.default_sql.as_deref(), Some("CURRENT_TIMESTAMP"));
    }

    // ---- string-aware parsing (P7) ---------------------------------------

    #[test]
    fn check_with_paren_in_string() {
        let (spec, _) = sqlite_spec(r##"#[column(check = "instr(x, ')') = 0")] x: String"##);
        assert_eq!(spec.check.as_deref(), Some("instr(x, ')') = 0"));
    }

    // ---- generated / collate (P10) ---------------------------------------

    #[test]
    fn sqlite_generated_stored_and_virtual() {
        let (stored, _) = sqlite_spec(r##"#[column(generated(stored, "a + b"))] v: i64"##);
        assert_eq!(
            stored.generated,
            Some(ParsedGenerated {
                expression: "a + b".to_string(),
                stored: true
            })
        );
        let (virt, _) = sqlite_spec(r##"#[column(generated(virtual, "a + b"))] v: i64"##);
        assert!(!virt.generated.unwrap().stored);
    }

    #[test]
    fn sqlite_collate_ident_uppercased_string_verbatim() {
        let (ident_form, _) = sqlite_spec("#[column(collate = nocase)] v: String");
        assert_eq!(ident_form.collate.as_deref(), Some("NOCASE"));
        let (string_form, _) = sqlite_spec(r##"#[column(collate = "nocase")] v: String"##);
        assert_eq!(string_form.collate.as_deref(), Some("nocase"));
    }

    #[test]
    fn pg_collate_ident_kept_verbatim() {
        let (spec, _) = pg_spec("#[column(COLLATE = C)] v: String");
        assert_eq!(spec.collate.as_deref(), Some("C"));
    }

    // ---- postgres specifics (P13) ----------------------------------------

    #[test]
    fn pg_deferrable_flags() {
        let (spec, _) =
            pg_spec("#[column(references = Users::id, deferrable, initially_deferred)] u: i32");
        assert!(spec.deferrable && spec.initially_deferred);
    }

    #[test]
    fn pg_identity_modes_and_options() {
        let (spec, _) = pg_spec("#[column(identity(by_default, start = 100, cycle))] id: i64");
        let identity = spec.identity.expect("identity");
        assert!(!identity.always);
        assert_eq!(identity.start.as_deref(), Some("100"));
        assert!(identity.cycle);

        let (bare, _) = pg_spec("#[column(identity)] id: i64");
        assert!(bare.identity.expect("identity").always);
    }

    #[test]
    fn pg_serial_requires_matching_int() {
        let (ok, diags) = pg_spec("#[column(serial)] id: i32");
        assert_eq!(ok.serial, Some(SerialKind::Serial));
        assert!(diags.errors.is_empty());

        let (bad, diags) = pg_spec("#[column(serial)] id: i64");
        assert!(bad.serial.is_none());
        assert!(!diags.errors.is_empty());
    }

    #[test]
    fn mysql_explicit_integer_type_requires_matching_signedness() {
        let (_, signed_as_unsigned) = mysql_spec("#[column(INT_UNSIGNED)] value: i32");
        assert!(signed_as_unsigned.errors.iter().any(|error| {
            error.contains("INT UNSIGNED is unsigned but the Rust field is signed")
        }));

        let (_, unsigned_as_signed) = mysql_spec("#[column(INT)] value: u32");
        assert!(
            unsigned_as_signed
                .errors
                .iter()
                .any(|error| { error.contains("INT is signed but the Rust field is unsigned") })
        );

        let (_, matching) = mysql_spec("#[column(INT_UNSIGNED)] value: u32");
        assert!(matching.errors.is_empty(), "{:#?}", matching.errors);
    }

    #[test]
    fn mysql_column_comment_requires_explicit_comment_attribute() {
        let (documented, diags) = mysql_spec("/// Rust API documentation.\nvalue: String");
        assert!(diags.errors.is_empty(), "{:#?}", diags.errors);
        assert_eq!(documented.comment, None);

        let (commented, diags) = mysql_spec(
            "/// Rust API documentation.\n#[column(COMMENT = \"SQL metadata\")] value: String",
        );
        assert!(diags.errors.is_empty(), "{:#?}", diags.errors);
        assert_eq!(commented.comment.as_deref(), Some("SQL metadata"));
    }

    #[test]
    fn pg_vec_maps_to_array() {
        let (spec, _) = pg_spec("tags: Vec<String>");
        assert_eq!(spec.pg_type.as_deref(), Some("TEXT"));
        assert_eq!(spec.pg_dimensions, Some(1));

        let (blob, _) = pg_spec("data: Vec<u8>");
        assert_eq!(blob.pg_type.as_deref(), Some("BYTEA"));
        assert_eq!(blob.pg_dimensions, None);
    }

    #[test]
    fn pg_enum_column_uses_type_name() {
        let (spec, _) = pg_spec("#[column(enum)] status: OrderStatus");
        assert_eq!(spec.pg_type.as_deref(), Some("OrderStatus"));
    }

    // ---- Option handling (P12) -------------------------------------------

    #[test]
    fn qualified_option_recognized() {
        let (spec, _) = sqlite_spec("email: std::option::Option<String>");
        assert!(spec.nullable);
        assert_eq!(spec.sqlite_type.as_deref(), Some("TEXT"));
    }

    #[test]
    fn nested_option_inference() {
        let (spec, _) = sqlite_spec("id: Option<uuid::Uuid>");
        assert!(spec.nullable);
        assert_eq!(spec.sqlite_type.as_deref(), Some("BLOB"));
    }

    // ---- type inference tables -------------------------------------------

    #[test]
    fn sqlite_type_inference() {
        for (ty, expected) in [
            ("i32", "INTEGER"),
            ("i64", "INTEGER"),
            ("f64", "REAL"),
            ("String", "TEXT"),
            ("bool", "INTEGER"),
            ("Vec<u8>", "BLOB"),
            ("Uuid", "BLOB"),
            ("uuid::Uuid", "BLOB"),
            ("compact_str::CompactString", "TEXT"),
            ("bytes::Bytes", "BLOB"),
            ("smallvec::SmallVec<[u8; 16]>", "BLOB"),
            ("chrono::NaiveDateTime", "TEXT"),
        ] {
            let (spec, _) = sqlite_spec(&format!("v: {ty}"));
            assert_eq!(spec.sqlite_type.as_deref(), Some(expected), "type {ty}");
        }
    }

    #[test]
    fn sqlite_unknown_type_is_any_with_warning() {
        let (spec, diags) = sqlite_spec("v: MyCustomType");
        assert_eq!(spec.sqlite_type.as_deref(), Some("ANY"));
        assert!(spec.is_custom_type);
        assert!(!diags.warnings.is_empty());
    }

    #[test]
    fn pg_type_inference() {
        for (ty, expected) in [
            ("i16", "SMALLINT"),
            ("i32", "INTEGER"),
            ("i64", "BIGINT"),
            ("bool", "BOOLEAN"),
            ("String", "TEXT"),
            ("f64", "DOUBLE PRECISION"),
            ("Vec<u8>", "BYTEA"),
            ("Uuid", "UUID"),
            ("serde_json::Value", "JSONB"),
            ("compact_str::CompactString", "VARCHAR"),
            ("arrayvec::ArrayString<32>", "VARCHAR"),
            ("chrono::DateTime<chrono::Utc>", "TIMESTAMPTZ"),
        ] {
            let (spec, _) = pg_spec(&format!("v: {ty}"));
            assert_eq!(spec.pg_type.as_deref(), Some(expected), "type {ty}");
        }
    }

    #[test]
    fn sqlite_explicit_type_overrides() {
        let (spec, _) = sqlite_spec("#[column(text)] id: uuid::Uuid");
        assert_eq!(spec.sqlite_type.as_deref(), Some("TEXT"));
        let (legacy, _) = sqlite_spec("#[text] id: uuid::Uuid");
        assert_eq!(legacy.sqlite_type.as_deref(), Some("TEXT"));
        let (json, _) = sqlite_spec("#[column(jsonb)] doc: MyDoc");
        assert_eq!(json.sqlite_type.as_deref(), Some("BLOB"));
    }

    #[test]
    fn sqlite_explicit_custom_blob_records_blob() {
        let (spec, diags) = sqlite_spec("#[column(blob)] digest: Digest");

        assert_eq!(spec.sqlite_type.as_deref(), Some("BLOB"));
        assert!(diags.warnings.is_empty());
        assert!(diags.errors.is_empty());
    }

    // ---- table attributes -------------------------------------------------

    fn parse_attr(code: &str) -> Attribute {
        let item: syn::ItemStruct =
            syn::parse_str(&format!("{code}\nstruct T {{ id: i64 }}")).expect("valid struct");
        item.attrs.into_iter().next().expect("one attribute")
    }

    fn parse_index(code: &str, dialect: Dialect) -> (IndexSpec, Diags) {
        let item: syn::ItemStruct = syn::parse_str(code).expect("valid index struct");
        let attr = item.attrs.first().expect("one index attribute");
        let mut diags = Diags::new();
        let spec = index_spec(attr, &item, dialect, "index", &mut diags);
        (spec, diags)
    }

    #[test]
    fn index_names_are_case_insensitive_and_validated_for_every_dialect() {
        for (macro_name, dialect) in [
            ("SQLiteIndex", Dialect::SQLite),
            ("PostgresIndex", Dialect::PostgreSQL),
            ("MySQLIndex", Dialect::MySQL),
        ] {
            let (spec, diags) = parse_index(
                &format!("#[{macro_name}(NaMe = \"physical_name\")] struct T(Table::column);"),
                dialect,
            );
            assert_eq!(spec.explicit_name.as_deref(), Some("physical_name"));
            assert!(diags.errors.is_empty(), "{macro_name}: {:#?}", diags.errors);
        }

        let (_, duplicate) = parse_index(
            "#[SQLiteIndex(name = \"first\", NAME = \"second\")] struct T(Table::column);",
            Dialect::SQLite,
        );
        assert_eq!(duplicate.errors.len(), 1);

        let (_, empty) = parse_index(
            "#[PostgresIndex(NAME = \"  \" )] struct T(Table::column);",
            Dialect::PostgreSQL,
        );
        assert_eq!(empty.errors.len(), 1);
    }

    #[test]
    fn sqlite_table_options() {
        let attr = parse_attr(r##"#[SQLiteTable(strict, without_rowid, name = "t")]"##);
        let mut diags = Diags::new();
        let spec = table_spec(&attr, Dialect::SQLite, &[], "T", &mut diags);
        assert!(spec.strict && spec.without_rowid);
        assert_eq!(spec.explicit_name.as_deref(), Some("t"));
        assert!(diags.errors.is_empty());
    }

    #[test]
    fn table_name_does_not_leak_options() {
        // `name = "strict_mode"` must not flip STRICT (old P2 defect).
        let attr = parse_attr(r##"#[SQLiteTable(name = "strict_mode")]"##);
        let mut diags = Diags::new();
        let spec = table_spec(&attr, Dialect::SQLite, &[], "T", &mut diags);
        assert!(!spec.strict);
    }

    #[test]
    fn sqlite_table_level_constraints() {
        let attr = parse_attr(
            r##"#[SQLiteTable(FOREIGN_KEY(columns(a, b), references(Parent, x, y), on_delete = "CASCADE"), UNIQUE(a, b), CHECK(expr = "a > 0"))]"##,
        );
        let mut diags = Diags::new();
        let spec = table_spec(&attr, Dialect::SQLite, &[], "T", &mut diags);
        assert_eq!(spec.composite_fks.len(), 1);
        assert_eq!(spec.composite_fks[0].source_columns, vec!["a", "b"]);
        assert_eq!(spec.composite_fks[0].target_table, "Parent");
        assert_eq!(spec.composite_fks[0].on_delete.as_deref(), Some("CASCADE"));
        assert_eq!(spec.unique_constraints.len(), 1);
        assert_eq!(spec.check_constraints[0].expr, "a > 0");
        assert!(diags.errors.is_empty());
    }

    #[test]
    fn pg_table_options() {
        let attr = parse_attr(
            r##"#[PostgresTable(SCHEMA = "auth", RLS, UNLOGGED, UNIQUE(columns(a), nulls_not_distinct, deferrable))]"##,
        );
        let mut diags = Diags::new();
        let spec = table_spec(&attr, Dialect::PostgreSQL, &[], "T", &mut diags);
        assert_eq!(spec.schema.as_deref(), Some("auth"));
        assert!(spec.rls && spec.unlogged);
        let unique = &spec.unique_constraints[0];
        assert!(unique.nulls_not_distinct && unique.deferrable);
        assert!(diags.errors.is_empty());
    }

    // ---- naming helpers ---------------------------------------------------

    #[test]
    fn index_name_derivation() {
        assert_eq!(sqlite_index_name("IdxUsersEmail"), "idx_users_email");
        assert_eq!(postgres_index_name("UsersEmailIdx"), "users_email_idx");
        assert_eq!(postgres_index_name("UsersEmail"), "users_email_idx");
        assert_eq!(postgres_index_name("UsersEmailIndex"), "users_email_index");
    }

    // ---- referential action table ----------------------------------------

    #[test]
    fn referential_action_table() {
        assert_eq!(
            normalize_referential_action("set_null").as_deref(),
            Some("SET NULL")
        );
        assert_eq!(
            normalize_referential_action("NO_ACTION").as_deref(),
            Some("NO ACTION")
        );
        assert_eq!(
            normalize_referential_action("Restrict").as_deref(),
            Some("RESTRICT")
        );
        assert_eq!(normalize_referential_action("bogus"), None);
    }
}
