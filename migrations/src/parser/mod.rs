//! Schema Parser for Drizzle Rust Schema Code
//!
//! Parses Rust schema source with [`syn`] into structured data used for
//! validation, analysis, and snapshot generation. Entities are recognized by
//! the *last segment* of their attribute / derive paths, so both
//! `#[SQLiteTable]` and `#[drizzle::SQLiteTable]` forms work. Supports
//! `SQLite`, `PostgreSQL`, and `MySQL` schema code.
//!
//! Attribute *interpretation* lives in the private `attrs` module and mirrors the procedural
//! macros' semantics exactly — see that module for the ground rules.
//!
//! # Failure model
//!
//! `SchemaParser::parse` never panics and never silently drops items:
//!
//! * Source that fails to parse as Rust records an entry in
//!   [`ParseResult::errors`] (and produces no entities).
//! * Attributes the macros would reject at compile time also record
//!   [`ParseResult::errors`]; entities are still emitted best-effort.
//! * Constructs the parser understands but cannot fully represent (opaque
//!   view definitions, `cfg`-divergent duplicates, trait-deferred column
//!   types) record [`ParseResult::warnings`].
//!
//! # Example
//!
//! ```rust
//! use drizzle_migrations::parser::SchemaParser;
//! use drizzle_types::Dialect;
//!
//! let code = r#"
//! #[SQLiteTable]
//! struct Users {
//!     #[column(primary, autoincrement)]
//!     id: i64,
//!     name: String,
//! }
//! "#;
//!
//! let result = SchemaParser::parse(code);
//! let users = result.table("Users", Dialect::SQLite).unwrap();
//! assert!(users.field("id").unwrap().is_primary_key());
//! ```

mod attrs;
mod types;

pub use types::*;

pub(crate) use attrs::{mysql_index_name, postgres_index_name, sqlite_index_name};

use attrs::{Diags, attr_last_segment, spanned_source};
use drizzle_types::Dialect;

// =============================================================================
// Schema Parser
// =============================================================================

/// Parser for Drizzle Rust schema code
pub struct SchemaParser;

impl SchemaParser {
    /// Parse Rust schema code into structured data.
    /// Automatically detects `SQLite` / `PostgreSQL` / `MySQL` entities.
    ///
    /// The input may be a concatenation of several schema files (the
    /// build-time and CLI flows join files with newlines); inner doc
    /// comments (`//!`) and single-line inner attributes (`#![...]`) from
    /// non-leading files are ignored so the concatenation stays parseable.
    #[must_use]
    pub fn parse(code: &str) -> ParseResult {
        let mut result = ParseResult::default();
        let source = prepare_source(code);

        let file = match syn::parse_file(&source) {
            Ok(file) => file,
            Err(err) => {
                let span = err.span().start();
                result.errors.push(format!(
                    "failed to parse schema source as Rust (line {}, column {}): {err}",
                    span.line,
                    span.column + 1
                ));
                return result;
            }
        };

        let mut walker = Walker {
            source: &source,
            result: &mut result,
            diags: Diags::new(),
            next_order: 0,
        };
        walker.walk_items(&file.items);

        let Walker { diags, .. } = walker;
        result.warnings.extend(diags.warnings);
        result.errors.extend(diags.errors);
        validate_mysql_database_scope(&mut result);
        result
    }
}

/// Strip a UTF-8 BOM and blank out inner doc comments / single-line inner
/// attributes so concatenated schema files still parse. Replaced spans keep
/// their byte length, so source slicing by span stays correct.
fn prepare_source(code: &str) -> String {
    let code = code.strip_prefix('\u{feff}').unwrap_or(code);
    let mut out = String::with_capacity(code.len());
    for line in code.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let is_inner_doc = trimmed.starts_with("//!");
        let is_inner_attr = trimmed.starts_with("#![") && trimmed.trim_end().ends_with(']');
        if is_inner_doc || is_inner_attr {
            // Preserve byte offsets: replace the content with spaces, keep
            // the trailing newline.
            let (content, newline) = match line.strip_suffix("\r\n") {
                Some(content) => (content, "\r\n"),
                None => match line.strip_suffix('\n') {
                    Some(content) => (content, "\n"),
                    None => (line, ""),
                },
            };
            out.extend(std::iter::repeat_n(' ', content.len()));
            out.push_str(newline);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// MySQL migration snapshots represent one selected database at a time. The
/// table macro permits a `DATABASE` qualifier because it is needed for normal
/// query DDL, but a migration diff cannot safely turn that into an implicit
/// cross-database move. Restrict the parsed *schema selection* to one explicit
/// database; unqualified tables share the configured/default scope and are
/// therefore allowed alongside that one name.
fn validate_mysql_database_scope(result: &mut ParseResult) {
    let selected = result
        .schema
        .as_ref()
        .filter(|schema| schema.dialect == Dialect::MySQL)
        .map(|schema| {
            schema
                .member_types
                .iter()
                .map(String::as_str)
                .collect::<std::collections::HashSet<_>>()
        });
    let mut databases: Vec<&str> = result
        .tables
        .values()
        .filter(|table| table.dialect == Dialect::MySQL)
        .filter(|table| {
            selected
                .as_ref()
                .is_none_or(|members| members.contains(table.name.as_str()))
        })
        .filter_map(|table| table.spec.database.as_deref())
        .collect();
    databases.extend(
        result
            .views
            .values()
            .filter(|view| view.dialect == Dialect::MySQL)
            .filter(|view| {
                selected
                    .as_ref()
                    .is_none_or(|members| members.contains(view.name.as_str()))
            })
            .filter_map(|view| view.database.as_deref()),
    );
    databases.sort_unstable();
    databases.dedup();
    if databases.len() > 1 {
        result.errors.push(format!(
            "MySQL migration schema selects multiple databases ({}) — migrations operate on one database scope and do not support cross-database moves",
            databases.join(", ")
        ));
    }
}

struct Walker<'a> {
    source: &'a str,
    result: &'a mut ParseResult,
    diags: Diags,
    next_order: usize,
}

impl Walker<'_> {
    fn next_order(&mut self) -> usize {
        let order = self.next_order;
        self.next_order += 1;
        order
    }

    fn walk_items(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::Struct(item_struct) => self.visit_struct(item_struct),
                syn::Item::Enum(item_enum) => self.visit_enum(item_enum),
                syn::Item::Mod(item_mod) => {
                    if let Some((_, nested)) = &item_mod.content {
                        self.walk_items(nested);
                    }
                }
                _ => {}
            }
        }
    }

    fn visit_struct(&mut self, item: &syn::ItemStruct) {
        // Dialect-marker attributes are matched by *last* path segment so
        // path-form attributes (`#[drizzle::SQLiteTable]`) are recognized.
        for attr in &item.attrs {
            let Some(last) = attr_last_segment(attr) else {
                continue;
            };
            match last.as_str() {
                "SQLiteTable" => return self.visit_table(item, attr, Dialect::SQLite),
                "PostgresTable" => return self.visit_table(item, attr, Dialect::PostgreSQL),
                "MySQLTable" => return self.visit_table(item, attr, Dialect::MySQL),
                "SQLiteIndex" => return self.visit_index(item, attr, Dialect::SQLite),
                "PostgresIndex" => return self.visit_index(item, attr, Dialect::PostgreSQL),
                "MySQLIndex" => return self.visit_index(item, attr, Dialect::MySQL),
                "SQLiteView" => return self.visit_view(item, attr, Dialect::SQLite),
                "PostgresView" => return self.visit_view(item, attr, Dialect::PostgreSQL),
                "MySQLView" => return self.visit_view(item, attr, Dialect::MySQL),
                "PostgresPolicy" => return self.visit_policy(item, attr),
                _ => {}
            }
        }

        // Schema structs are recognized through their derive list.
        for dialect in derive_dialects(&item.attrs, "SQLiteSchema", "PostgresSchema", "MySQLSchema")
        {
            self.visit_schema(item, dialect);
        }
    }

    fn visit_table(&mut self, item: &syn::ItemStruct, attr: &syn::Attribute, dialect: Dialect) {
        let name = item.ident.to_string();
        let desc = format!("table `{name}`");
        let spec = attrs::table_spec(attr, dialect, &item.attrs, &desc, &mut self.diags);

        let syn::Fields::Named(named) = &item.fields else {
            self.diags
                .errors
                .push(format!("{desc}: table structs must have named fields"));
            return;
        };

        let mut fields = Vec::with_capacity(named.named.len());
        for field in &named.named {
            let Some(ident) = &field.ident else { continue };
            let column_spec = match dialect {
                Dialect::PostgreSQL => {
                    attrs::postgres_column_spec(field, self.source, &desc, &mut self.diags)
                }
                Dialect::SQLite => {
                    attrs::sqlite_column_spec(field, self.source, &desc, &mut self.diags)
                }
                Dialect::MySQL => {
                    attrs::mysql_column_spec(field, self.source, &desc, &mut self.diags)
                }
            };
            fields.push(ParsedField {
                name: ident.to_string(),
                ty: spanned_source(self.source, &field.ty),
                attrs: field_attr_texts(self.source, &field.attrs),
                spec: column_spec,
            });
        }

        let table = ParsedTable {
            name: name.clone(),
            attr: spanned_source(self.source, attr),
            fields,
            dialect,
            spec,
            order: self.next_order(),
        };

        let key = entity_key(dialect, &name);
        if let Some(existing) = self.result.tables.get(&key) {
            self.note_duplicate(
                &desc,
                &item.attrs,
                format!(
                    "{:?}|{:?}|{:?}",
                    existing.attr, existing.fields, existing.spec
                ),
                format!("{:?}|{:?}|{:?}", table.attr, table.fields, table.spec),
            );
            return;
        }

        if self.result.dialect == Dialect::default() {
            self.result.dialect = dialect;
        }
        self.result.tables.insert(key, table);
    }

    fn visit_index(&mut self, item: &syn::ItemStruct, attr: &syn::Attribute, dialect: Dialect) {
        let name = item.ident.to_string();
        let desc = format!("index `{name}`");
        let spec = attrs::index_spec(attr, item, dialect, &desc, &mut self.diags);

        let index = ParsedIndex {
            name: name.clone(),
            attr: spanned_source(self.source, attr),
            columns: spec
                .column_refs
                .iter()
                .map(|(table, column)| format!("{table}::{column}"))
                .collect(),
            dialect,
            spec,
            order: self.next_order(),
        };

        let key = entity_key(dialect, &name);
        if let Some(existing) = self.result.indexes.get(&key) {
            self.note_duplicate(
                &desc,
                &item.attrs,
                format!("{:?}|{:?}", existing.attr, existing.spec),
                format!("{:?}|{:?}", index.attr, index.spec),
            );
            return;
        }
        self.result.indexes.insert(key, index);
    }

    fn visit_view(&mut self, item: &syn::ItemStruct, attr: &syn::Attribute, dialect: Dialect) {
        let name = item.ident.to_string();
        let desc = format!("view `{name}`");
        let data = attrs::view_spec(attr, dialect, &desc, &mut self.diags);

        if data.definition.is_none() && !data.has_opaque_definition && !data.existing {
            self.diags.errors.push(format!(
                "{desc}: views require a DEFINITION attribute unless marked EXISTING"
            ));
        }

        let view = ParsedView {
            name: name.clone(),
            attr: spanned_source(self.source, attr),
            dialect,
            explicit_name: data.name,
            schema: data.schema,
            database: data.database,
            definition: data.definition,
            has_opaque_definition: data.has_opaque_definition,
            materialized: data.materialized,
            existing: data.existing,
            with_no_data: data.with_no_data,
            using: data.using,
            tablespace: data.tablespace,
            mysql_algorithm: data.mysql_algorithm,
            mysql_sql_security: data.mysql_sql_security,
            mysql_check_option: data.mysql_check_option,
            order: self.next_order(),
        };

        let key = entity_key(dialect, &name);
        if let Some(existing) = self.result.views.get(&key) {
            self.note_duplicate(
                &desc,
                &item.attrs,
                format!("{existing:?}"),
                format!("{view:?}"),
            );
            return;
        }
        self.result.views.insert(key, view);
    }

    fn visit_policy(&mut self, item: &syn::ItemStruct, attr: &syn::Attribute) {
        let name = item.ident.to_string();
        let desc = format!("policy `{name}`");
        let data = attrs::policy_spec(attr, &desc, &mut self.diags);

        // The policy macro requires a tuple struct with exactly one table
        // reference.
        let table = match &item.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().expect("one field");
                if let syn::Type::Path(p) = &field.ty {
                    p.path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default()
                } else {
                    self.diags.errors.push(format!(
                        "{desc}: PostgresPolicy must reference a table type, e.g. \
                         struct UsersPolicy(Users);"
                    ));
                    return;
                }
            }
            _ => {
                self.diags.errors.push(format!(
                    "{desc}: PostgresPolicy must be a tuple struct with one table reference"
                ));
                return;
            }
        };

        let policy = ParsedPolicy {
            name: name.clone(),
            attr: spanned_source(self.source, attr),
            dialect: Dialect::PostgreSQL,
            table,
            explicit_name: data.name,
            as_clause: data.as_clause,
            for_clause: data.for_clause,
            to: data.to,
            using: data.using,
            with_check: data.with_check,
            order: self.next_order(),
        };

        let key = entity_key(Dialect::PostgreSQL, &name);
        if let Some(existing) = self.result.policies.get(&key) {
            self.note_duplicate(
                &desc,
                &item.attrs,
                format!("{existing:?}"),
                format!("{policy:?}"),
            );
            return;
        }
        self.result.policies.insert(key, policy);
    }

    fn visit_schema(&mut self, item: &syn::ItemStruct, dialect: Dialect) {
        let name = item.ident.to_string();
        let syn::Fields::Named(named) = &item.fields else {
            self.diags.errors.push(format!(
                "schema `{name}`: schema structs must have named fields"
            ));
            return;
        };

        let mut members = std::collections::HashMap::new();
        let mut member_types = Vec::new();
        for field in &named.named {
            let Some(ident) = &field.ident else { continue };
            members.insert(ident.to_string(), spanned_source(self.source, &field.ty));
            if let syn::Type::Path(p) = &field.ty
                && let Some(last) = p.path.segments.last()
            {
                member_types.push(last.ident.to_string());
            }
        }

        let schema = ParsedSchema {
            name,
            members,
            dialect,
            member_types,
        };

        // Match the previous parser's behavior: the first schema struct wins
        // and pins the result dialect.
        if self.result.schema.is_none() {
            self.result.dialect = dialect;
            self.result.schema = Some(schema);
        }
    }

    fn visit_enum(&mut self, item: &syn::ItemEnum) {
        for dialect in derive_dialects(&item.attrs, "SQLiteEnum", "PostgresEnum", "MySQLEnum") {
            let name = item.ident.to_string();
            if dialect == Dialect::MySQL {
                if item.variants.is_empty() {
                    self.diags.errors.push(format!(
                        "enum `{name}`: MySQLEnum requires at least one variant"
                    ));
                }
                if item.attrs.iter().any(|attr| attr.path().is_ident("repr")) {
                    self.diags.errors.push(format!(
                        "enum `{name}`: MySQLEnum does not support #[repr(...)]"
                    ));
                }
                for variant in &item.variants {
                    if !matches!(variant.fields, syn::Fields::Unit) {
                        self.diags.errors.push(format!(
                            "enum `{name}`: MySQLEnum only supports fieldless variants"
                        ));
                    }
                    if variant.discriminant.is_some() {
                        self.diags.errors.push(format!(
                            "enum `{name}`: MySQLEnum variants cannot have explicit discriminants"
                        ));
                    }
                    let label = variant.ident.to_string();
                    let label = label.trim_start_matches("r#");
                    if label.contains(['\'', '\\']) {
                        self.diags.errors.push(format!(
                            "enum `{name}`: MySQLEnum label `{label}` contains a quote or backslash"
                        ));
                    }
                }
            }
            let schema = if dialect == Dialect::PostgreSQL {
                enum_schema_attr(&item.attrs)
            } else {
                None
            };
            let parsed = ParsedEnum {
                name: name.clone(),
                variants: item
                    .variants
                    .iter()
                    .map(|variant| {
                        let variant = variant.ident.to_string();
                        if dialect == Dialect::MySQL {
                            variant.trim_start_matches("r#").to_string()
                        } else {
                            variant
                        }
                    })
                    .collect(),
                dialect,
                schema,
                order: self.next_order(),
            };
            let key = entity_key(dialect, &name);
            if let Some(existing) = self.result.enums.get(&key) {
                self.note_duplicate(
                    &format!("enum `{name}`"),
                    &item.attrs,
                    format!("{existing:?}"),
                    format!("{parsed:?}"),
                );
                continue;
            }
            self.result.enums.insert(key, parsed);
        }
    }

    /// Duplicate definitions keep the first occurrence. This is how
    /// mutually-exclusive `#[cfg(...)]`-gated variants surface to the parser
    /// (it sees pre-cfg source; the macros see post-cfg source, so perfect
    /// fidelity is impossible). If the duplicates differ, warn so the user
    /// knows which variant the snapshot is based on.
    fn note_duplicate(
        &mut self,
        desc: &str,
        attrs_of_new: &[syn::Attribute],
        existing_fingerprint: String,
        new_fingerprint: String,
    ) {
        if existing_fingerprint == new_fingerprint {
            return;
        }
        let cfg_note = if attrs::has_cfg(attrs_of_new) {
            " (the duplicate is #[cfg]-gated; the parser cannot evaluate cfg predicates and \
             keeps the first definition)"
        } else {
            ""
        };
        self.diags.warnings.push(format!(
            "{desc}: duplicate definition differs from the first one; keeping the first{cfg_note}"
        ));
    }
}

/// `#[postgres_enum(schema = "...")]` helper-attribute value on an enum item,
/// matching the derive's own parsing (only the `schema` key is recognized).
fn enum_schema_attr(attrs: &[syn::Attribute]) -> Option<String> {
    let mut schema = None;
    for attr in attrs {
        if !attr.path().is_ident("postgres_enum") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("schema")
                && let Ok(value) = meta.value()
                && let Ok(lit) = value.parse::<syn::LitStr>()
            {
                let name = lit.value();
                if !name.is_empty() {
                    schema = Some(name);
                }
            }
            Ok(())
        });
    }
    schema
}

/// Dialects for which the item derives the given schema/enum markers,
/// matched by last derive-path segment.
fn derive_dialects(
    attrs: &[syn::Attribute],
    sqlite: &str,
    postgres: &str,
    mysql: &str,
) -> Vec<Dialect> {
    let mut dialects = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if let Some(last) = meta.path.segments.last() {
                let name = last.ident.to_string();
                if name == sqlite {
                    dialects.push(Dialect::SQLite);
                } else if name == postgres {
                    dialects.push(Dialect::PostgreSQL);
                } else if name == mysql {
                    dialects.push(Dialect::MySQL);
                }
            }
            Ok(())
        });
    }
    dialects
}

/// Textual attribute list for the compat surface: every attribute except doc
/// comments and `cfg`/`cfg_attr` (whose text would otherwise pollute the
/// substring-based `has_attr`).
fn field_attr_texts(source: &str, attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|attr| {
            !attr.path().is_ident("doc")
                && !attr.path().is_ident("cfg")
                && !attr.path().is_ident("cfg_attr")
        })
        .map(|attr| spanned_source(source, attr))
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_table() {
        let code = r#"
#[SQLiteTable]
struct Users {
    #[column(primary, autoincrement)]
    id: i64,
    name: String,
    email: Option<String>,
}
"#;
        let result = SchemaParser::parse(code);
        assert_eq!(result.dialect, Dialect::SQLite);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert_eq!(users.name, "Users");
        assert_eq!(users.fields.len(), 3);

        let id_field = users.field("id").unwrap();
        assert!(id_field.has_attr("primary"));
        assert!(id_field.has_attr("autoincrement"));
        assert!(id_field.is_primary_key());
        assert!(id_field.is_autoincrement());

        let email_field = users.field("email").unwrap();
        assert!(email_field.is_nullable());
    }

    #[test]
    fn test_parse_table_with_options() {
        let code = r#"
#[SQLiteTable(strict, without_rowid)]
struct Products {
    #[column(primary)]
    id: String,
    name: String,
}
"#;
        let result = SchemaParser::parse(code);
        let products = result.table("Products", Dialect::SQLite).unwrap();
        assert!(products.has_table_attr("strict"));
        assert!(products.has_table_attr("without_rowid"));
        assert!(products.is_strict());
        assert!(products.is_without_rowid());
    }

    #[test]
    fn test_uppercase_markers_are_recognized() {
        // P1: the macros match markers case-insensitively; `#[column(PRIMARY)]`
        // and friends must not be lost.
        let code = r#"
#[SQLiteTable(STRICT)]
struct Users {
    #[column(PRIMARY, AUTOINCREMENT)]
    id: i64,
    #[column(UNIQUE)]
    email: String,
}
"#;
        let result = SchemaParser::parse(code);
        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert!(users.is_strict());
        assert!(users.field("id").unwrap().is_primary_key());
        assert!(users.field("id").unwrap().is_autoincrement());
        assert!(users.field("email").unwrap().is_unique());
    }

    #[test]
    fn test_no_phantom_constraints() {
        // P2: substring matches must not produce phantom constraints.
        let code = r#"
#[SQLiteTable(name = "strict_mode")]
struct Config {
    #[column(name = "primary_email")]
    email: String,
}
"#;
        let result = SchemaParser::parse(code);
        let config = result.table("Config", Dialect::SQLite).unwrap();
        assert!(!config.is_strict(), "table name must not imply STRICT");
        assert!(
            !config.field("email").unwrap().is_primary_key(),
            "column name must not imply PRIMARY KEY"
        );
    }

    #[test]
    fn test_parse_index() {
        let code = r#"
#[SQLiteIndex(unique)]
struct IdxUsersEmail(Users::email);
"#;
        let result = SchemaParser::parse(code);
        let idx = result.index("IdxUsersEmail", Dialect::SQLite).unwrap();
        assert!(idx.is_unique());
        assert_eq!(idx.columns, vec!["Users::email"]);
    }

    #[test]
    fn test_parse_mysql_index_options() {
        let code = r#"
#[MySQLIndex(unique, using = "HASH", algorithm = "INPLACE", lock = "NONE")]
struct IdxUsersEmail(Users::email);
"#;
        let result = SchemaParser::parse(code);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let idx = result.index("IdxUsersEmail", Dialect::MySQL).unwrap();
        assert!(idx.is_unique());
        assert_eq!(idx.mysql_using().as_deref(), Some("hash"));
        assert_eq!(idx.mysql_algorithm().as_deref(), Some("inplace"));
        assert_eq!(idx.mysql_lock().as_deref(), Some("none"));
    }

    #[test]
    fn test_parse_mysql_schema_producer_metadata() {
        let code = r#"
#[derive(MySQLEnum)]
enum Status {
    Draft,
    Published,
}

#[MySQLTable(
    DATABASE = "app",
    TEMPORARY,
    ENGINE = "InnoDB",
    CHARSET = "utf8mb4",
    COLLATE = "utf8mb4_0900_ai_ci",
    COMMENT = "application users"
)]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: i64,
    #[column(ENUM, CHARSET = "utf8mb4", COLLATE = "utf8mb4_bin", COMMENT = "lifecycle")]
    status: Status,
    #[column(SET("admin", "editor"))]
    roles: String,
    #[column(DATETIME(6), ON_UPDATE = "CURRENT_TIMESTAMP(6)")]
    updated_at: chrono::NaiveDateTime,
    #[column(INT, generated(stored, "id + 1"))]
    derived_id: i32,
}
"#;

        let result = SchemaParser::parse(code);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let users = result.table("Users", Dialect::MySQL).unwrap();
        assert_eq!(users.spec.database.as_deref(), Some("app"));
        assert!(users.spec.temporary);
        assert_eq!(users.spec.engine.as_deref(), Some("InnoDB"));
        assert_eq!(users.spec.charset.as_deref(), Some("utf8mb4"));
        assert_eq!(users.spec.collate.as_deref(), Some("utf8mb4_0900_ai_ci"));
        assert_eq!(users.spec.comment.as_deref(), Some("application users"));

        let id = users.field("id").unwrap();
        assert!(id.spec.primary);
        assert!(id.spec.autoincrement);
        assert_eq!(id.spec.mysql_type.as_deref(), Some("BIGINT"));

        let status = users.field("status").unwrap();
        assert_eq!(status.spec.mysql_type.as_deref(), Some("ENUM"));
        assert!(status.spec.mysql_inline_enum);
        assert_eq!(status.spec.charset.as_deref(), Some("utf8mb4"));
        assert_eq!(status.spec.collate.as_deref(), Some("utf8mb4_bin"));
        assert_eq!(status.spec.comment.as_deref(), Some("lifecycle"));

        let roles = users.field("roles").unwrap();
        assert_eq!(
            roles.spec.mysql_type.as_deref(),
            Some("SET('admin', 'editor')")
        );
        assert_eq!(
            roles.spec.mysql_set_values,
            Some(vec!["admin".to_string(), "editor".to_string()])
        );

        let updated_at = users.field("updated_at").unwrap();
        assert_eq!(updated_at.spec.mysql_type.as_deref(), Some("DATETIME(6)"));
        assert_eq!(
            updated_at.spec.mysql_on_update.as_deref(),
            Some("CURRENT_TIMESTAMP(6)")
        );
        assert_eq!(
            users.field("derived_id").unwrap().spec.generated,
            Some(ParsedGenerated {
                expression: "id + 1".to_string(),
                stored: true,
            })
        );

        assert_eq!(
            result
                .parsed_enum("Status", Dialect::MySQL)
                .unwrap()
                .variants,
            vec!["Draft", "Published"]
        );
    }

    #[test]
    fn test_mysql_schema_rejects_multiple_database_scopes() {
        let code = r#"
#[MySQLTable(DATABASE = "catalog")]
struct Products { id: i64 }

#[MySQLTable(DATABASE = "identity")]
struct Users { id: i64 }

#[derive(MySQLSchema)]
struct AppSchema {
    products: Products,
    users: Users,
}
"#;

        let result = SchemaParser::parse(code);
        assert!(result.errors.iter().any(|error| {
            error.contains("multiple databases") && error.contains("cross-database moves")
        }));
    }

    #[test]
    fn test_parse_schema() {
        let code = r#"
#[SQLiteTable]
struct Users {
    id: i64,
}

#[derive(SQLiteSchema)]
struct AppSchema {
    users: Users,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert_eq!(schema.name, "AppSchema");
        assert!(schema.members.contains_key("users"));
    }

    #[test]
    fn test_parse_postgres_table() {
        let code = r#"
#[PostgresTable(schema = "auth")]
struct Users {
    #[column(primary, identity)]
    id: i32,
    name: String,
}
"#;
        let result = SchemaParser::parse(code);
        assert_eq!(result.dialect, Dialect::PostgreSQL);

        let users = result.table("Users", Dialect::PostgreSQL).unwrap();
        assert_eq!(users.dialect, Dialect::PostgreSQL);
        assert_eq!(users.schema_name().as_deref(), Some("auth"));
        assert!(users.field("id").unwrap().spec.identity.is_some());
    }

    #[test]
    fn test_parse_field_with_references() {
        let code = r#"
#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i64,
    #[column(references = Users::id, on_delete = Cascade)]
    user_id: i64,
}
"#;
        let result = SchemaParser::parse(code);
        let posts = result.table("Posts", Dialect::SQLite).unwrap();
        let user_id = posts.field("user_id").unwrap();

        assert_eq!(user_id.references(), Some("Users::id".to_string()));
        // Compat accessor returns the raw spelling; the normalized action
        // lives in the structured spec.
        assert_eq!(user_id.on_delete(), Some("Cascade".to_string()));
        assert_eq!(user_id.spec.on_delete.as_deref(), Some("CASCADE"));
    }

    #[test]
    fn test_multi_dialect_schema() {
        let code = r#"
#[SQLiteTable]
struct SqliteUsers {
    id: i64,
}

#[PostgresTable]
struct PostgresUsers {
    id: i32,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.table("SqliteUsers", Dialect::SQLite).is_some());
        assert!(result.table("PostgresUsers", Dialect::PostgreSQL).is_some());
    }

    #[test]
    fn test_attr_values_with_defaults() {
        let code = r#"
#[SQLiteTable]
struct Config {
    #[column(default = 42)]
    count: i64,
    #[column(default = "hello")]
    message: String,
}
"#;
        let result = SchemaParser::parse(code);
        let config = result.table("Config", Dialect::SQLite).unwrap();

        let count = config.field("count").unwrap();
        assert_eq!(count.default_value(), Some("42".to_string()));

        // The parser-level accessor deliberately returns the raw Rust
        // literal (quotes included); SQL quoting is the snapshot builder's
        // job. The structured spec carries the unquoted value.
        let message = config.field("message").unwrap();
        assert_eq!(message.default_value(), Some("\"hello\"".to_string()));
        assert_eq!(
            message.spec.default,
            Some(ParsedDefault::Str("hello".to_string()))
        );
    }

    #[test]
    fn test_brace_inside_attribute_string() {
        // P5: `}` inside an attribute string must not truncate the struct.
        let code = r#"
#[SQLiteTable]
struct Docs {
    #[column(default = "{}")]
    payload: String,
    trailing: i64,
}
"#;
        let result = SchemaParser::parse(code);
        let docs = result.table("Docs", Dialect::SQLite).unwrap();
        assert_eq!(docs.fields.len(), 2);
        assert!(docs.field("trailing").is_some());
    }

    #[test]
    fn test_pub_crate_fields_are_kept() {
        // P6: restricted-visibility fields must not be dropped.
        let code = r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub(crate) id: i64,
    pub(in crate::schema) name: String,
    email: String,
}
"#;
        let result = SchemaParser::parse(code);
        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert_eq!(users.fields.len(), 3);
    }

    #[test]
    fn test_block_comments_and_derives_between_attrs() {
        // P16: derives, doc comments, and block comments between the table
        // attribute and the struct must not drop the table.
        let code = r#"
#[SQLiteTable]
/* block comment */
#[derive(Debug, Clone)]
/// Doc comment.
struct Users {
    /* another */ id: i64,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.table("Users", Dialect::SQLite).is_some());
    }

    #[test]
    fn test_path_form_attribute() {
        // P12: `#[drizzle::SQLiteTable]` must be recognized.
        let code = r#"
#[drizzle::SQLiteTable(name = "users")]
struct Users {
    id: i64,
}
"#;
        let result = SchemaParser::parse(code);
        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert_eq!(users.spec.explicit_name.as_deref(), Some("users"));
    }

    #[test]
    fn test_parse_error_is_loud() {
        let result = SchemaParser::parse("#[SQLiteTable]\nstruct Broken {");
        assert!(result.tables.is_empty());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_bom_and_inner_docs() {
        let code = "\u{feff}//! module docs\n#[SQLiteTable]\nstruct Users { id: i64 }\n//! inner doc from a concatenated file\n#[SQLiteTable]\nstruct Posts { id: i64 }\n";
        let result = SchemaParser::parse(code);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(result.table("Users", Dialect::SQLite).is_some());
        assert!(result.table("Posts", Dialect::SQLite).is_some());
    }

    #[test]
    fn test_cfg_duplicates_first_wins_with_warning() {
        // P11: mutually-exclusive cfg-gated duplicates dedupe
        // deterministically (first definition wins) with a warning when the
        // variants differ.
        let code = r#"
#[cfg(feature = "uuid")]
#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: uuid::Uuid,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
}
"#;
        let result = SchemaParser::parse(code);
        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert_eq!(users.field("id").unwrap().ty, "uuid::Uuid");
        assert!(
            result.warnings.iter().any(|w| w.contains("duplicate")),
            "expected duplicate warning, got {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_cfg_gated_column_is_included() {
        let code = r#"
#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
    #[cfg(feature = "emails")]
    email: String,
}
"#;
        let result = SchemaParser::parse(code);
        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert!(users.field("email").is_some());
    }

    #[test]
    fn test_parse_postgres_index() {
        let code = r#"
#[PostgresIndex(unique, concurrent, method = "gin", where = "name IS NOT NULL")]
struct IdxUsersName(Users::name);
"#;
        let result = SchemaParser::parse(code);
        let idx = result.index("IdxUsersName", Dialect::PostgreSQL).unwrap();
        assert_eq!(idx.dialect, Dialect::PostgreSQL);
        assert!(idx.is_unique());
        assert!(idx.is_concurrent());
        assert_eq!(idx.method().as_deref(), Some("gin"));
        assert_eq!(idx.where_clause().as_deref(), Some("name IS NOT NULL"));
    }

    #[test]
    fn test_parse_sqlite_partial_unique_index() {
        let code = r#"
#[SQLiteIndex(unique, where = "builder IS NULL")]
struct IdxPlanJobs(Jobs::workspace, Jobs::asset, Jobs::kind);
"#;
        let result = SchemaParser::parse(code);
        let idx = result.index("IdxPlanJobs", Dialect::SQLite).unwrap();
        assert!(idx.is_unique());
        assert_eq!(idx.where_clause().as_deref(), Some("builder IS NULL"));
        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn test_parse_postgres_index_where_clause_with_commas() {
        let code = r#"
#[PostgresIndex(where = "coalesce(first_name, last_name) IS NOT NULL")]
struct IdxUsersDisplayName(Users::first_name);
"#;

        let result = SchemaParser::parse(code);
        let idx = result
            .index("IdxUsersDisplayName", Dialect::PostgreSQL)
            .unwrap();

        assert_eq!(
            idx.where_clause().as_deref(),
            Some("coalesce(first_name, last_name) IS NOT NULL")
        );
    }

    #[test]
    fn test_parse_postgres_schema() {
        let code = r#"
#[derive(PostgresSchema)]
struct DbSchema {
    users: Users,
    posts: Posts,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert_eq!(schema.dialect, Dialect::PostgreSQL);
        assert_eq!(schema.members.len(), 2);
    }

    #[test]
    fn test_dialect_detection() {
        let sqlite_code = r#"
#[SQLiteTable]
struct T { id: i64 }
"#;
        let pg_code = r#"
#[PostgresTable]
struct T { id: i32 }
"#;
        assert_eq!(SchemaParser::parse(sqlite_code).dialect, Dialect::SQLite);
        assert_eq!(SchemaParser::parse(pg_code).dialect, Dialect::PostgreSQL);
    }

    #[test]
    fn test_parse_enums() {
        // P8: enum derives are recognized (derive form is what the macros
        // register — SQLiteEnum / PostgresEnum are derive macros).
        let code = r#"
#[derive(PostgresEnum, Default, Clone)]
enum OrderStatus {
    #[default]
    Pending,
    Shipped,
    Delivered,
}

#[derive(SQLiteEnum, Default)]
enum Role {
    #[default]
    User,
    Admin,
}
"#;
        let result = SchemaParser::parse(code);
        let status = result
            .parsed_enum("OrderStatus", Dialect::PostgreSQL)
            .unwrap();
        assert_eq!(status.variants, vec!["Pending", "Shipped", "Delivered"]);
        assert!(result.parsed_enum("Role", Dialect::SQLite).is_some());
    }

    #[test]
    fn test_parse_views() {
        let code = r#"
#[SQLiteView(definition = "SELECT id FROM users WHERE active = 1")]
struct ActiveUsers {
    id: i64,
}

#[PostgresView(schema = "app", materialized, definition = "SELECT 1 AS one")]
struct Ones {
    one: i32,
}

#[MySQLView(
    DATABASE = "app",
    NAME = "active_accounts",
    DEFINITION = "SELECT id FROM accounts WHERE active = 1",
    ALGORITHM = "TEMP_TABLE",
    SECURITY = "INVOKER",
    WITH_CHECK_OPTION = "LOCAL"
)]
struct ActiveAccounts {
    id: u64,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.errors.is_empty(), "{:#?}", result.errors);
        let active = result.view("ActiveUsers", Dialect::SQLite).unwrap();
        assert_eq!(
            active.definition.as_deref(),
            Some("SELECT id FROM users WHERE active = 1")
        );
        let ones = result.view("Ones", Dialect::PostgreSQL).unwrap();
        assert!(ones.materialized);
        assert_eq!(ones.schema.as_deref(), Some("app"));
        let accounts = result.view("ActiveAccounts", Dialect::MySQL).unwrap();
        assert_eq!(accounts.explicit_name.as_deref(), Some("active_accounts"));
        assert_eq!(accounts.database.as_deref(), Some("app"));
        assert_eq!(accounts.mysql_algorithm.as_deref(), Some("temptable"));
        assert_eq!(accounts.mysql_sql_security.as_deref(), Some("invoker"));
        assert_eq!(accounts.mysql_check_option.as_deref(), Some("local"));
    }

    #[test]
    fn mysql_view_rejects_conflicting_definition_sources() {
        let result = SchemaParser::parse(
            r#"
#[MySQLView(EXISTING, DEFINITION = "SELECT 1", query(Accounts::id))]
struct InvalidView { id: u64 }
"#,
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("mutually exclusive"))
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("EXISTING cannot be combined"))
        );
    }

    #[test]
    fn mysql_view_rejects_non_rc_source_options() {
        let result = SchemaParser::parse(
            r#"
#[MySQLView(
    DEFINITION = "SELECT 1",
    DEFINER = "root@localhost",
    CHARSET = "utf8mb4",
    COLLATE = "utf8mb4_bin"
)]
struct InvalidView { one: i32 }
"#,
        );
        assert_eq!(
            result
                .errors
                .iter()
                .filter(|error| error.contains("unrecognized view attribute"))
                .count(),
            3
        );
    }

    #[test]
    fn test_parse_policy() {
        let code = r#"
#[PostgresPolicy(NAME = "user_isolation", AS = "permissive", FOR = "select", TO(authenticated), USING = "user_id = current_user_id()")]
struct UsersPolicy(Users);
"#;
        let result = SchemaParser::parse(code);
        let policy = result.policy("UsersPolicy", Dialect::PostgreSQL).unwrap();
        assert_eq!(policy.table, "Users");
        assert_eq!(policy.explicit_name.as_deref(), Some("user_isolation"));
        assert_eq!(policy.as_clause.as_deref(), Some("PERMISSIVE"));
        assert_eq!(policy.for_clause.as_deref(), Some("SELECT"));
        assert_eq!(policy.to, vec!["authenticated"]);
    }

    #[test]
    fn test_std_option_nullability() {
        let code = r#"
#[SQLiteTable]
struct Users {
    email: std::option::Option<String>,
}
"#;
        let result = SchemaParser::parse(code);
        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert!(users.field("email").unwrap().is_nullable());
    }
}
