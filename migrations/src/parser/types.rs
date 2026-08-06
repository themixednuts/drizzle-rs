//! Parser types - shared data structures for all dialects
//!
//! Each parsed entity keeps two views of the source:
//!
//! * **Textual compat surface** — `attr` / `attrs` / `ty` hold the original
//!   source text (sliced by span), so string-oriented consumers (round-trip
//!   introspection tests, debug output) keep working unchanged.
//! * **Structured spec** — `spec` fields hold the attribute data interpreted
//!   with the *same semantics as the procedural macros* (case-insensitive
//!   markers, normalized referential actions, resolved SQL types). Snapshot
//!   building reads only the structured spec; the textual surface is never
//!   used for semantic decisions.

use std::collections::HashMap;

use drizzle_types::Dialect;

// =============================================================================
// Structured column / table specs (macro-equivalent interpretation)
// =============================================================================

/// A parsed `default = <literal>` value, mirroring the literal kinds the
/// table macros accept (`sqlite::field::build_sql_definition` /
/// `postgres::field::parse_column_attribute`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedDefault {
    /// Integer literal token text (e.g. `42`).
    Int(String),
    /// Float literal token text (e.g. `3.14`).
    Float(String),
    /// Boolean literal.
    Bool(bool),
    /// String literal *value* (unescaped, without the Rust quotes).
    Str(String),
    /// Any other expression (e.g. `default = -1`). The macros silently drop
    /// these; the snapshot builder mirrors that and emits a warning.
    Unsupported(String),
}

/// A `references = Table::column` target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedReference {
    /// Referenced table struct ident (e.g. `Users`).
    pub table: String,
    /// Referenced field ident (e.g. `id`).
    pub column: String,
}

/// `generated(stored|virtual, "expr")` data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGenerated {
    /// Raw SQL expression as written (no parenthesization applied).
    pub expression: String,
    /// `true` for STORED, `false` for VIRTUAL.
    pub stored: bool,
}

/// `identity(...)` data (`PostgreSQL` only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedIdentity {
    /// `true` for `GENERATED ALWAYS`, `false` for `BY DEFAULT`.
    pub always: bool,
    /// START WITH value.
    pub start: Option<String>,
    /// INCREMENT BY value.
    pub increment: Option<String>,
    /// MINVALUE.
    pub min_value: Option<String>,
    /// MAXVALUE.
    pub max_value: Option<String>,
    /// CACHE size.
    pub cache: Option<i32>,
    /// CYCLE flag.
    pub cycle: bool,
}

/// `PostgreSQL` serial pseudo-type markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialKind {
    Smallserial,
    Serial,
    Bigserial,
}

/// Structured, macro-equivalent interpretation of a field's column attributes
/// plus the resolved SQL type for the owning table's dialect.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnSpec {
    /// `primary` / `PRIMARY_KEY` marker.
    pub primary: bool,
    /// `unique` marker.
    pub unique: bool,
    /// `autoincrement` marker (`SQLite` only).
    pub autoincrement: bool,
    /// `json` marker.
    pub json: bool,
    /// `jsonb` marker.
    pub jsonb: bool,
    /// `enum` marker.
    pub enum_marker: bool,
    /// `serial` / `smallserial` / `bigserial` (`PostgreSQL` only).
    pub serial: Option<SerialKind>,
    /// `identity(...)` (`PostgreSQL` only).
    pub identity: Option<ParsedIdentity>,
    /// `generated(...)`.
    pub generated: Option<ParsedGenerated>,
    /// `default = <literal>`.
    pub default: Option<ParsedDefault>,
    /// `default_sql = "..."` raw SQL (not normalized; the snapshot builder
    /// applies the dialect-specific normalization the macros use).
    pub default_sql: Option<String>,
    /// `default_fn = ...` present (runtime-only default; no DDL impact).
    pub has_default_fn: bool,
    /// `check = "..."` column-level check expression.
    pub check: Option<String>,
    /// `references = Table::column`.
    pub references: Option<ParsedReference>,
    /// Normalized ON DELETE action (`CASCADE`, `SET NULL`, ...), exactly as
    /// the macros normalize (`SET_NULL` → `SET NULL`).
    pub on_delete: Option<String>,
    /// Normalized ON UPDATE action.
    pub on_update: Option<String>,
    /// ON DELETE action exactly as written in source (compat accessor).
    pub on_delete_raw: Option<String>,
    /// ON UPDATE action exactly as written in source (compat accessor).
    pub on_update_raw: Option<String>,
    /// `deferrable` FK marker (`PostgreSQL` only).
    pub deferrable: bool,
    /// `initially_deferred` FK marker (`PostgreSQL` only).
    pub initially_deferred: bool,
    /// Explicit column name from `name = "..."`.
    pub explicit_name: Option<String>,
    /// `collate = ...` (SQLite uppercases bare idents; Postgres keeps them
    /// verbatim — mirrored from the respective macros).
    pub collate: Option<String>,
    /// `relation = "..."` reverse-relation accessor name (no DDL impact).
    pub relation: Option<String>,
    /// Whether the Rust type is `Option<T>` (matched by last path segment,
    /// so `std::option::Option<T>` is recognized too).
    pub nullable: bool,
    /// Doc comment (used as the column comment for `PostgreSQL`).
    pub comment: Option<String>,
    /// Raw `key = value` pairs as written (key, raw value source text) for
    /// the textual `attr_value` compat accessor.
    pub named_values: Vec<(String, String)>,
    /// Resolved `SQLite` storage type (`INTEGER`, `TEXT`, ...). `None` for
    /// fields on non-SQLite tables.
    pub sqlite_type: Option<String>,
    /// Resolved `PostgreSQL` SQL type (`INTEGER`, `TEXT`, enum type name,
    /// ...). `None` for fields on non-Postgres tables. Array element type
    /// for `Vec<T>` columns (dimensions carried separately).
    pub pg_type: Option<String>,
    /// `PostgreSQL` array dimensions (`Some(1)` for `Vec<T>`).
    pub pg_dimensions: Option<i32>,
    /// The Rust type was unknown to the macro type tables; the macros defer
    /// the SQL type to a trait const the parser cannot evaluate.
    pub is_custom_type: bool,
}

/// Composite `FOREIGN_KEY(...)` table attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompositeFkSpec {
    /// Source field idents.
    pub source_columns: Vec<String>,
    /// Target table struct ident.
    pub target_table: String,
    /// Target field idents.
    pub target_columns: Vec<String>,
    /// Raw ON DELETE string (table-level FK actions are free-form strings in
    /// the macros; no normalization is applied there).
    pub on_delete: Option<String>,
    /// Raw ON UPDATE string.
    pub on_update: Option<String>,
    /// DEFERRABLE (`PostgreSQL` only).
    pub deferrable: bool,
    /// INITIALLY DEFERRED (`PostgreSQL` only).
    pub initially_deferred: bool,
}

/// Table-level `UNIQUE(...)` attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableUniqueSpec {
    /// Field idents.
    pub columns: Vec<String>,
    /// Explicit constraint name.
    pub name: Option<String>,
    /// NULLS NOT DISTINCT (`PostgreSQL` only).
    pub nulls_not_distinct: bool,
    /// DEFERRABLE (`PostgreSQL` only).
    pub deferrable: bool,
    /// INITIALLY DEFERRED (`PostgreSQL` only).
    pub initially_deferred: bool,
}

/// Table-level `CHECK(...)` attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableCheckSpec {
    /// Explicit constraint name.
    pub name: Option<String>,
    /// Check expression.
    pub expr: String,
}

/// Structured, macro-equivalent interpretation of the table attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableSpec {
    /// Explicit table name from `name = "..."`.
    pub explicit_name: Option<String>,
    /// `schema = "..."` (`PostgreSQL` only).
    pub schema: Option<String>,
    /// `strict` (`SQLite` only).
    pub strict: bool,
    /// `without_rowid` (`SQLite` only).
    pub without_rowid: bool,
    /// `UNLOGGED` (`PostgreSQL` only).
    pub unlogged: bool,
    /// `TEMPORARY` (`PostgreSQL` only).
    pub temporary: bool,
    /// `RLS` (`PostgreSQL` only).
    pub rls: bool,
    /// `INHERITS = "..."` (`PostgreSQL` only).
    pub inherits: Option<String>,
    /// `TABLESPACE = "..."` (`PostgreSQL` only).
    pub tablespace: Option<String>,
    /// Composite `FOREIGN_KEY(...)` attributes in declaration order.
    pub composite_fks: Vec<CompositeFkSpec>,
    /// Table-level `UNIQUE(...)` attributes in declaration order.
    pub unique_constraints: Vec<TableUniqueSpec>,
    /// Table-level `CHECK(...)` attributes in declaration order.
    pub check_constraints: Vec<TableCheckSpec>,
    /// Doc comment (used as the table comment for `PostgreSQL`).
    pub comment: Option<String>,
    /// Raw `key = value` pairs for the textual `attr_value` compat accessor.
    pub named_values: Vec<(String, String)>,
}

/// Structured index attribute data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexSpec {
    /// `unique` marker.
    pub unique: bool,
    /// `concurrent` marker (`PostgreSQL` only).
    pub concurrent: bool,
    /// Explicit `method = "..."` (`PostgreSQL` only). `None` when unwritten —
    /// PostgreSQL's implicit btree default is never materialized (the macro
    /// and the renderer both treat absence as btree).
    pub method: Option<String>,
    /// `where = "..."` partial-index predicate (`PostgreSQL` only).
    pub where_clause: Option<String>,
    /// `tablespace = "..."` (`PostgreSQL` only).
    pub tablespace: Option<String>,
    /// `name = "..."` — parser extension: the index macros derive the name
    /// from the struct ident and accept no `name` attribute, but the parser
    /// honors one when present so hand-annotated schema files keep working.
    pub explicit_name: Option<String>,
    /// `(table struct ident, field ident)` pairs in declaration order.
    pub column_refs: Vec<(String, String)>,
}

// =============================================================================
// Parsed entities
// =============================================================================

/// Parsed table struct from schema code
#[derive(Debug, Clone, Default)]
pub struct ParsedTable {
    /// Struct name (`PascalCase`)
    pub name: String,
    /// Full table attribute source text (e.g. `#[SQLiteTable(strict)]`)
    pub attr: String,
    /// Parsed fields
    pub fields: Vec<ParsedField>,
    /// Detected dialect
    pub dialect: Dialect,
    /// Structured table attribute data (macro-equivalent semantics).
    pub spec: TableSpec,
    /// Position of the item in the parsed source (used for deterministic
    /// snapshot entity ordering).
    pub(crate) order: usize,
}

/// Parsed index struct from schema code
#[derive(Debug, Clone, Default)]
pub struct ParsedIndex {
    /// Struct name (`PascalCase`)
    pub name: String,
    /// Full index attribute source text (e.g. `#[SQLiteIndex(unique)]`)
    pub attr: String,
    /// Column references (e.g. `["Users::id", "Users::name"]`)
    pub columns: Vec<String>,
    /// Detected dialect
    pub dialect: Dialect,
    /// Structured index attribute data.
    pub spec: IndexSpec,
    /// Source position, see [`ParsedTable::order`].
    pub(crate) order: usize,
}

/// Parsed schema struct from schema code
#[derive(Debug, Clone, Default)]
pub struct ParsedSchema {
    /// Struct name (e.g., "`AppSchema`")
    pub name: String,
    /// Schema members (`field_name` -> `type_name` source text)
    pub members: HashMap<String, String>,
    /// Detected dialect
    pub dialect: Dialect,
    /// Member type names reduced to their last path segment, in field order.
    pub(crate) member_types: Vec<String>,
}

/// Parsed field from a table/view struct
#[derive(Debug, Clone, Default)]
pub struct ParsedField {
    /// Field name (as written in Rust)
    pub name: String,
    /// Rust type source text (e.g., "i64", "Option<String>")
    pub ty: String,
    /// Field attribute source texts (e.g., `#[column(primary)]`); doc
    /// comments and `cfg` attributes are not included.
    pub attrs: Vec<String>,
    /// Structured column attribute data (macro-equivalent semantics).
    pub spec: ColumnSpec,
}

/// Parsed enum deriving `SQLiteEnum` / `PostgresEnum`.
#[derive(Debug, Clone, Default)]
pub struct ParsedEnum {
    /// Enum ident. For `PostgreSQL` this is also the SQL enum type name —
    /// the derive emits `CREATE TYPE <ident> AS ENUM (...)` verbatim.
    pub name: String,
    /// Variant idents in declaration order.
    pub variants: Vec<String>,
    /// Detected dialect
    pub dialect: Dialect,
    /// `#[postgres_enum(schema = "...")]` (`PostgreSQL` only). `None` means
    /// the type is created in `public`.
    pub schema: Option<String>,
    /// Source position, see [`ParsedTable::order`].
    pub(crate) order: usize,
}

/// Parsed `#[SQLiteView]` / `#[PostgresView]` struct.
#[derive(Debug, Clone, Default)]
pub struct ParsedView {
    /// Struct name (`PascalCase`)
    pub name: String,
    /// Full view attribute source text
    pub attr: String,
    /// Detected dialect
    pub dialect: Dialect,
    /// Explicit `name = "..."`.
    pub explicit_name: Option<String>,
    /// `schema = "..."` (`PostgreSQL` only).
    pub schema: Option<String>,
    /// `definition = "..."` literal SQL. `None` when the view uses the
    /// expression / `query(...)` forms, which the parser cannot evaluate
    /// (a warning is recorded in that case).
    pub definition: Option<String>,
    /// The view has a definition the parser cannot represent (expression or
    /// `query(...)` DSL form).
    pub has_opaque_definition: bool,
    /// `MATERIALIZED` (`PostgreSQL` only).
    pub materialized: bool,
    /// `EXISTING`.
    pub existing: bool,
    /// `WITH_NO_DATA` (`PostgreSQL` only).
    pub with_no_data: bool,
    /// `USING = "..."` (`PostgreSQL` only).
    pub using: Option<String>,
    /// `TABLESPACE = "..."` (`PostgreSQL` only).
    pub tablespace: Option<String>,
    /// Source position, see [`ParsedTable::order`].
    pub(crate) order: usize,
}

/// Parsed `#[PostgresPolicy]` struct.
#[derive(Debug, Clone, Default)]
pub struct ParsedPolicy {
    /// Struct name (`PascalCase`)
    pub name: String,
    /// Full policy attribute source text
    pub attr: String,
    /// Detected dialect
    pub dialect: Dialect,
    /// Target table struct ident (the single tuple field).
    pub table: String,
    /// Explicit `NAME = "..."`.
    pub explicit_name: Option<String>,
    /// Normalized `AS` clause (`PERMISSIVE` / `RESTRICTIVE`).
    pub as_clause: Option<String>,
    /// Normalized `FOR` clause (`ALL` / `SELECT` / ...).
    pub for_clause: Option<String>,
    /// `TO(...)` roles in declaration order.
    pub to: Vec<String>,
    /// `USING = "..."` expression.
    pub using: Option<String>,
    /// `WITH_CHECK = "..."` expression.
    pub with_check: Option<String>,
    /// Source position, see [`ParsedTable::order`].
    pub(crate) order: usize,
}

/// Result of parsing schema code
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    /// Parsed table structs, keyed `"<dialect>:<StructName>"`
    pub tables: HashMap<String, ParsedTable>,
    /// Parsed index structs, keyed `"<dialect>:<StructName>"`
    pub indexes: HashMap<String, ParsedIndex>,
    /// Parsed schema struct (if present)
    pub schema: Option<ParsedSchema>,
    /// Detected dialect (based on first found table/schema)
    pub dialect: Dialect,
    /// Parsed enums, keyed `"<dialect>:<EnumName>"`
    pub enums: HashMap<String, ParsedEnum>,
    /// Parsed views, keyed `"<dialect>:<StructName>"`
    pub views: HashMap<String, ParsedView>,
    /// Parsed policies, keyed `"<dialect>:<StructName>"`
    pub policies: HashMap<String, ParsedPolicy>,
    /// Non-fatal notes: constructs the parser recognized but cannot fully
    /// represent (opaque view definitions, cfg-divergent duplicates,
    /// custom column types, unsupported default expressions, ...).
    pub warnings: Vec<String>,
    /// Hard failures: source that failed to parse or attributes the macros
    /// would reject. Entities are still emitted best-effort, but callers
    /// should surface these to the user.
    pub errors: Vec<String>,
}

// =============================================================================
// Implementations
// =============================================================================

impl ParsedTable {
    /// Get a field by name
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&ParsedField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Textual check whether the table attribute source contains `attr`.
    ///
    /// This is a loose substring check kept for compatibility; semantic
    /// queries should use [`ParsedTable::is_strict`] and friends, which read
    /// the structured spec.
    #[must_use]
    pub fn has_table_attr(&self, attr: &str) -> bool {
        self.attr.contains(attr)
    }

    /// Get the raw value of a table attribute assignment (e.g. `schema = "auth"`).
    #[must_use]
    pub fn attr_value(&self, key: &str) -> Option<String> {
        self.spec
            .named_values
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    }

    /// Get the `PostgreSQL` schema name if set on `#[PostgresTable(...)]`.
    #[must_use]
    pub fn schema_name(&self) -> Option<String> {
        self.spec.schema.clone()
    }

    /// Check if table is marked as `SQLite` STRICT.
    #[must_use]
    pub const fn is_strict(&self) -> bool {
        self.spec.strict
    }

    /// Check if table is marked as `SQLite` WITHOUT ROWID.
    #[must_use]
    pub const fn is_without_rowid(&self) -> bool {
        self.spec.without_rowid
    }

    /// Get all field names
    #[must_use]
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }
}

impl ParsedIndex {
    /// Check if index is unique
    #[must_use]
    pub const fn is_unique(&self) -> bool {
        self.spec.unique
    }

    /// Check if `PostgreSQL` index is created concurrently.
    #[must_use]
    pub const fn is_concurrent(&self) -> bool {
        self.spec.concurrent
    }

    /// Get `PostgreSQL` index method (e.g. `btree`, `gin`) if explicitly set.
    #[must_use]
    pub fn method(&self) -> Option<String> {
        self.spec.method.clone()
    }

    /// Get `PostgreSQL` partial-index WHERE clause if explicitly set.
    #[must_use]
    pub fn where_clause(&self) -> Option<String> {
        self.spec.where_clause.clone()
    }

    /// Get the table name from the first column reference
    #[must_use]
    pub fn table_name(&self) -> Option<&str> {
        self.spec
            .column_refs
            .first()
            .map(|(table, _)| table.as_str())
    }
}

impl ParsedField {
    /// Textual check whether any attribute source contains `attr`.
    ///
    /// Loose substring check kept for compatibility (round-trip tests match
    /// exact generated attribute text with it). Semantic queries should use
    /// [`ParsedField::is_primary_key`] and friends.
    #[must_use]
    pub fn has_attr(&self, attr: &str) -> bool {
        self.attrs.iter().any(|a| a.contains(attr))
    }

    /// Get the raw value of an attribute assignment (e.g., `default = 42` -> Some("42"))
    #[must_use]
    pub fn attr_value(&self, key: &str) -> Option<String> {
        self.spec
            .named_values
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    }

    /// Get all raw attribute key-value pairs as a `HashMap`
    #[must_use]
    pub fn attr_values(&self) -> HashMap<String, String> {
        self.spec.named_values.iter().cloned().collect()
    }

    /// Get the combined column attribute string
    #[must_use]
    pub fn column_attr(&self) -> String {
        self.attrs.join(", ")
    }

    /// Check if field is nullable (`Option<T>`, matched by last path segment)
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        self.spec.nullable
    }

    /// Check if field is a primary key
    #[must_use]
    pub const fn is_primary_key(&self) -> bool {
        self.spec.primary
    }

    /// Check if field has autoincrement
    #[must_use]
    pub const fn is_autoincrement(&self) -> bool {
        self.spec.autoincrement
    }

    /// Check if field is unique
    #[must_use]
    pub const fn is_unique(&self) -> bool {
        self.spec.unique
    }

    /// Get the default value if present, as the raw source text of the
    /// literal (string defaults keep their Rust quotes here; SQL quoting
    /// happens in the snapshot builder).
    #[must_use]
    pub fn default_value(&self) -> Option<String> {
        self.attr_value("default")
            .or_else(|| match self.spec.default.as_ref()? {
                ParsedDefault::Int(s) | ParsedDefault::Float(s) => Some(s.clone()),
                ParsedDefault::Bool(b) => Some(b.to_string()),
                ParsedDefault::Str(s) => Some(format!("{s:?}")),
                ParsedDefault::Unsupported(raw) => Some(raw.clone()),
            })
    }

    /// Get the references target if present (e.g., "`Users::id`")
    #[must_use]
    pub fn references(&self) -> Option<String> {
        self.spec
            .references
            .as_ref()
            .map(|r| format!("{}::{}", r.table, r.column))
    }

    /// Get the `on_delete` action exactly as written (e.g. `cascade`).
    ///
    /// The normalized SQL action (`CASCADE`, `SET NULL`, ...) lives in
    /// `spec.on_delete`.
    #[must_use]
    pub fn on_delete(&self) -> Option<String> {
        self.spec.on_delete_raw.clone()
    }

    /// Get the `on_update` action exactly as written.
    #[must_use]
    pub fn on_update(&self) -> Option<String> {
        self.spec.on_update_raw.clone()
    }
}

impl ParseResult {
    /// Get a table by name and dialect
    #[must_use]
    pub fn table(&self, name: &str, dialect: Dialect) -> Option<&ParsedTable> {
        self.tables.get(&entity_key(dialect, name))
    }

    /// Get an index by name and dialect
    #[must_use]
    pub fn index(&self, name: &str, dialect: Dialect) -> Option<&ParsedIndex> {
        self.indexes.get(&entity_key(dialect, name))
    }

    /// Get an enum by name and dialect
    #[must_use]
    pub fn parsed_enum(&self, name: &str, dialect: Dialect) -> Option<&ParsedEnum> {
        self.enums.get(&entity_key(dialect, name))
    }

    /// Get a view by name and dialect
    #[must_use]
    pub fn view(&self, name: &str, dialect: Dialect) -> Option<&ParsedView> {
        self.views.get(&entity_key(dialect, name))
    }

    /// Get a policy by name and dialect
    #[must_use]
    pub fn policy(&self, name: &str, dialect: Dialect) -> Option<&ParsedPolicy> {
        self.policies.get(&entity_key(dialect, name))
    }

    /// Get all tables for a specific dialect
    pub fn tables_for_dialect(&self, dialect: Dialect) -> impl Iterator<Item = &ParsedTable> {
        let prefix = format!("{}:", dialect_key(dialect));
        self.tables
            .iter()
            .filter(move |(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v)
    }

    /// Get all indexes for a specific dialect
    pub fn indexes_for_dialect(&self, dialect: Dialect) -> impl Iterator<Item = &ParsedIndex> {
        let prefix = format!("{}:", dialect_key(dialect));
        self.indexes
            .iter()
            .filter(move |(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v)
    }

    /// Get all table names (without dialect prefix)
    #[must_use]
    pub fn table_names(&self) -> Vec<&str> {
        self.tables
            .keys()
            .filter_map(|s| s.split(':').nth(1))
            .collect()
    }

    /// Get all index names (without dialect prefix)
    #[must_use]
    pub fn index_names(&self) -> Vec<&str> {
        self.indexes
            .keys()
            .filter_map(|s| s.split(':').nth(1))
            .collect()
    }
}

/// Get the key prefix for a dialect
pub(crate) const fn dialect_key(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::SQLite => "sqlite",
        Dialect::PostgreSQL => "postgres",
        Dialect::MySQL => "mysql",
    }
}

/// Build a `"<dialect>:<name>"` map key.
pub(crate) fn entity_key(dialect: Dialect, name: &str) -> String {
    format!("{}:{}", dialect_key(dialect), name)
}
