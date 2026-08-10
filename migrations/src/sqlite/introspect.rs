//! `SQLite` database introspection
//!
//! This module provides functionality to introspect an existing `SQLite` database
//! and extract its schema as DDL entities, matching drizzle-kit introspect.ts

use super::ddl::{
    Column, ForeignKey, Index, IndexColumn, IndexOrigin, PrimaryKey, SqliteEntity, Table,
    UniqueConstraint, View,
};
use super::ddl::{GeneratedType, ParsedGenerated};
use super::snapshot::SQLiteSnapshot;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Error type for introspection operations
#[derive(Debug, Clone)]
pub struct IntrospectError {
    pub message: String,
    pub table: Option<String>,
}

impl std::fmt::Display for IntrospectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(table) = &self.table {
            write!(f, "Introspection error for '{}': {}", table, self.message)
        } else {
            write!(f, "Introspection error: {}", self.message)
        }
    }
}

impl std::error::Error for IntrospectError {}

/// Result type for introspection
pub type IntrospectResult<T> = Result<T, IntrospectError>;

/// Raw column info from `pragma_table_xinfo`
#[derive(Debug, Clone)]
pub struct RawColumnInfo {
    pub table: String,
    pub cid: i32,
    pub name: String,
    pub column_type: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub pk: i32,
    pub hidden: i32,
    pub sql: Option<String>,
}

/// Raw index info from `pragma_index_list`
#[derive(Debug, Clone)]
pub struct RawIndexInfo {
    pub table: String,
    pub name: String,
    pub unique: bool,
    pub origin: String, // 'c' for CREATE INDEX, 'u' for UNIQUE, 'pk' for PRIMARY KEY
    pub partial: bool,
}

/// Raw index column from `pragma_index_xinfo`
#[derive(Debug, Clone)]
pub struct RawIndexColumn {
    pub index_name: String,
    pub seqno: i32,
    pub cid: i32,
    pub name: Option<String>,
    pub desc: bool,
    pub coll: String,
    pub key: bool,
}

/// Raw foreign key info from `pragma_foreign_key_list`
#[derive(Debug, Clone)]
pub struct RawForeignKey {
    pub table: String,
    pub id: i32,
    pub seq: i32,
    pub to_table: String,
    pub from_column: String,
    pub to_column: String,
    pub on_update: String,
    pub on_delete: String,
    pub r#match: String,
}

/// Raw view info
#[derive(Debug, Clone)]
pub struct RawViewInfo {
    pub name: String,
    pub sql: String,
}

/// Transport-independent raw rows needed to assemble a complete SQLite DDL.
///
/// Drivers are responsible only for decoding these rows from their transport;
/// schema semantics live in [`assemble_ddl`].
#[derive(Debug, Clone, Default)]
pub struct RawIntrospection {
    pub tables: Vec<(String, Option<String>)>,
    pub columns: Vec<RawColumnInfo>,
    pub indexes: Vec<RawIndexInfo>,
    pub index_columns: Vec<RawIndexColumn>,
    pub foreign_keys: Vec<RawForeignKey>,
    pub views: Vec<RawViewInfo>,
    /// `(index name, CREATE INDEX sql)` rows from [`queries::INDEX_SQL_QUERY`].
    /// Recovers partial-index `WHERE` clauses and expression columns that the
    /// index PRAGMAs cannot express.
    pub index_sql: Vec<(String, String)>,
}

/// Assemble transport-decoded SQLite metadata into the canonical DDL model.
#[must_use]
pub fn assemble_ddl(raw: RawIntrospection) -> super::SQLiteDDL {
    let table_sql_map: HashMap<String, String> = raw
        .tables
        .iter()
        .filter_map(|(name, sql)| sql.as_ref().map(|sql| (name.clone(), sql.clone())))
        .collect();

    let mut generated_columns = HashMap::<String, ParsedGenerated>::new();
    for (table, sql) in &table_sql_map {
        generated_columns.extend(parse_generated_columns_from_table_sql(table, sql));
    }
    let primary_key_columns: HashSet<(String, String)> = raw
        .columns
        .iter()
        .filter(|column| column.pk > 0)
        .map(|column| (column.table.clone(), column.name.clone()))
        .collect();

    let (columns, primary_keys) =
        process_columns(&raw.columns, &generated_columns, &primary_key_columns);
    let index_sql_map: HashMap<String, String> = raw.index_sql.iter().cloned().collect();
    let indexes = process_indexes_with_sql(&raw.indexes, &raw.index_columns, &index_sql_map);
    let foreign_keys = process_foreign_keys(&raw.foreign_keys);
    let unique_constraints =
        process_unique_constraints_from_indexes(&raw.indexes, &raw.index_columns);

    let mut ddl = super::SQLiteDDL::new();
    for (table_name, table_sql) in raw.tables {
        let mut table = Table::new(table_name);
        if let Some(sql) = table_sql {
            let (strict, without_rowid) = parse_table_options(&sql);
            table.strict = strict;
            table.without_rowid = without_rowid;
        }
        ddl.tables.push(table);
    }
    for column in columns {
        ddl.columns.push(column);
    }
    for index in indexes {
        ddl.indexes.push(index);
    }
    for foreign_key in foreign_keys {
        ddl.fks.push(foreign_key);
    }
    for primary_key in primary_keys {
        ddl.pks.push(primary_key);
    }
    for unique_constraint in unique_constraints {
        ddl.uniques.push(unique_constraint);
    }

    for raw_view in raw.views {
        let mut view = View::new(raw_view.name);
        if let Some(definition) = parse_view_sql(&raw_view.sql) {
            view.definition = Some(definition.into());
        } else {
            view.error = Some("Failed to parse view SQL".into());
        }
        ddl.views.push(view);
    }

    ddl
}

/// Entity filter function type
pub type EntityFilter = Box<dyn Fn(&str, &str) -> bool>;

/// Default entity filter that allows everything
pub fn default_filter() -> EntityFilter {
    Box::new(|_entity_type, _name| true)
}

/// System table filter - excludes `SQLite` system tables and drizzle migrations
#[must_use]
pub fn system_table_filter(name: &str) -> bool {
    !name.starts_with("sqlite_")
        && !name.starts_with("_cf_")
        && !name.starts_with("_litestream_")
        && !name.starts_with("libsql_")
        && !name.starts_with("d1_")
        && name != "__drizzle_migrations"
}

/// Introspection result containing all extracted entities
#[derive(Debug, Clone, Default)]
pub struct IntrospectionResult {
    pub tables: Vec<Table>,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub primary_keys: Vec<PrimaryKey>,
    pub unique_constraints: Vec<UniqueConstraint>,
    pub views: Vec<View>,
    pub errors: Vec<IntrospectError>,
}

impl IntrospectionResult {
    /// Convert to a snapshot
    #[must_use]
    pub fn to_snapshot(&self) -> SQLiteSnapshot {
        let mut snapshot = SQLiteSnapshot::new();

        for table in &self.tables {
            snapshot.add_entity(SqliteEntity::Table(table.clone()));
        }
        for column in &self.columns {
            snapshot.add_entity(SqliteEntity::Column(column.clone()));
        }
        for index in &self.indexes {
            snapshot.add_entity(SqliteEntity::Index(index.clone()));
        }
        for fk in &self.foreign_keys {
            snapshot.add_entity(SqliteEntity::ForeignKey(fk.clone()));
        }
        for pk in &self.primary_keys {
            snapshot.add_entity(SqliteEntity::PrimaryKey(pk.clone()));
        }
        for unique in &self.unique_constraints {
            snapshot.add_entity(SqliteEntity::UniqueConstraint(unique.clone()));
        }
        for view in &self.views {
            snapshot.add_entity(SqliteEntity::View(view.clone()));
        }

        snapshot
    }

    /// Check if introspection had any errors
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Process raw column info into Column entities
#[must_use]
pub fn process_columns<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    raw_columns: &[RawColumnInfo],
    generated_columns: &std::collections::HashMap<String, super::ddl::ParsedGenerated, S1>,
    _pk_columns: &std::collections::HashSet<(String, String), S2>, // (table, column) - reserved for future use
) -> (Vec<Column>, Vec<PrimaryKey>) {
    // Precompute AUTOINCREMENT columns once per table (avoids per-column regex compilation).
    let mut autoinc_by_table: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for c in raw_columns {
        if autoinc_by_table.contains_key(&c.table) {
            continue;
        }
        let Some(sql) = c.sql.as_deref() else {
            continue;
        };
        autoinc_by_table.insert(
            c.table.clone(),
            parse_autoincrement_columns_from_table_sql(sql),
        );
    }

    let columns: Vec<Column> = raw_columns
        .iter()
        // pragma_table_xinfo hidden values: 0 = normal, 1 = hidden (virtual
        // table implementation columns), 2 = VIRTUAL generated, 3 = STORED
        // generated. Generated columns are real schema and must be kept —
        // their Generated info is attached from the parsed CREATE TABLE SQL.
        .filter(|c| c.hidden != 1)
        .map(|c| {
            let key = format!("{}:{}", c.table, c.name);
            let generated = generated_columns.get(&key).map(|g| super::ddl::Generated {
                expression: g.expression.clone().into(),
                gen_type: g.gen_type,
            });

            let is_autoincrement = autoinc_by_table
                .get(&c.table)
                .is_some_and(|set| set.contains(&c.name));

            Column {
                table: c.table.clone().into(),
                name: c.name.clone().into(),
                sql_type: normalize_sql_type(&c.column_type).into(),
                not_null: c.not_null,
                autoincrement: if is_autoincrement { Some(true) } else { None },
                primary_key: None, // Handled via PrimaryKey entity
                unique: None,      // Handled via UniqueConstraint entity
                default: c.default_value.clone().map(std::convert::Into::into),
                generated,
                // PRAGMA table_info doesn't expose the collation per column;
                // introspection-derived snapshots leave it as the default.
                // Migration paths that need to detect collation drift will
                // need to parse the CREATE TABLE SQL stored in sqlite_schema.
                collate: None,
                ordinal_position: Some(c.cid),
            }
        })
        .collect();

    // Extract primary keys from raw columns. BTreeMap keeps the emitted PK
    // entity order deterministic; the `pk` value from pragma_table_xinfo is
    // the column's 1-based position within the PRIMARY KEY, so sorting by it
    // preserves the declared PK column order (which can differ from cid order
    // for composite keys like PRIMARY KEY(b, a)).
    let mut pk_map: BTreeMap<String, Vec<(i32, String)>> = BTreeMap::new();

    for c in raw_columns.iter().filter(|c| c.pk > 0) {
        pk_map
            .entry(c.table.clone())
            .or_default()
            .push((c.pk, c.name.clone()));
    }

    let primary_keys: Vec<PrimaryKey> = pk_map
        .into_iter()
        .map(|(table, mut cols)| {
            cols.sort_by_key(|(pk_pos, _)| *pk_pos);
            let name = super::ddl::name_for_pk(&table);
            PrimaryKey {
                table: table.into(),
                name: name.into(),
                name_explicit: false,
                columns: cols.into_iter().map(|(_, name)| name.into()).collect(),
            }
        })
        .collect();

    (columns, primary_keys)
}

/// Normalize a SQL type to lowercase canonical form
fn normalize_sql_type(sql_type: &str) -> String {
    sql_type.to_lowercase()
}

/// Extract the text between the outermost balanced parentheses of a CREATE TABLE body.
///
/// Returns the slice between the first '(' and its matching ')', or `None` if
/// either is missing.
fn extract_table_body(sql: &str) -> Option<&str> {
    let sql = sql.trim();
    let start = sql.find('(')?;

    let mut depth = 0i32;
    let mut end: Option<usize> = None;
    for (i, ch) in sql.char_indices().skip(start) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    Some(&sql[start + 1..end?])
}

/// Returns the text AFTER the balanced closing paren of the table body.
///
/// This is where table options (`STRICT`, `WITHOUT ROWID`) legally appear;
/// scanning the whole statement would false-positive on columns named
/// `strict` or string content inside the body.
fn table_options_tail(sql: &str) -> Option<&str> {
    let start = sql.find('(')?;
    let mut depth = 0i32;
    for (i, ch) in sql.char_indices().skip(start) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&sql[i + ch.len_utf8()..]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse `(strict, without_rowid)` table options from a CREATE TABLE statement.
///
/// Only the text after the balanced closing paren of the table body is
/// scanned, using word-token matching.
#[must_use]
pub fn parse_table_options(sql: &str) -> (bool, bool) {
    let Some(tail) = table_options_tail(sql) else {
        return (false, false);
    };
    let upper = tail.to_uppercase();
    let tokens: Vec<&str> = upper
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|t| !t.is_empty())
        .collect();
    let strict = tokens.contains(&"STRICT");
    let without_rowid = tokens
        .windows(2)
        .any(|w| w[0] == "WITHOUT" && w[1] == "ROWID");
    (strict, without_rowid)
}

/// Split a string on top-level commas (commas not inside parentheses).
fn split_top_level_commas(body: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    let mut part_start = 0usize;
    let mut p_depth = 0i32;
    for (i, ch) in body.char_indices() {
        match ch {
            '(' => p_depth += 1,
            ')' => p_depth -= 1,
            ',' if p_depth == 0 => {
                parts.push(body[part_start..i].trim());
                part_start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(body[part_start..].trim());
    parts
}

/// Returns true if the column definition is actually a table-level constraint
/// rather than a column definition we should parse.
fn is_table_level_constraint(upper: &str) -> bool {
    upper.starts_with("CONSTRAINT ")
        || upper.starts_with("PRIMARY ")
        || upper.starts_with("UNIQUE ")
        || upper.starts_with("CHECK ")
        || upper.starts_with("FOREIGN ")
}

/// Parse a leading column name from a column-definition string.
///
/// Returns the `(name, remainder_after_name)` pair. Handles `"…"`, `` `…` ``,
/// `[…]` quoted identifiers and bare identifiers.
fn take_column_name(def: &str) -> Option<(String, &str)> {
    let def = def.trim();
    if let Some(r) = def.strip_prefix('"') {
        let endq = r.find('"')?;
        return Some((r[..endq].to_string(), r[endq + 1..].trim_start()));
    }
    if let Some(r) = def.strip_prefix('`') {
        let endq = r.find('`')?;
        return Some((r[..endq].to_string(), r[endq + 1..].trim_start()));
    }
    if let Some(r) = def.strip_prefix('[') {
        let endq = r.find(']')?;
        return Some((r[..endq].to_string(), r[endq + 1..].trim_start()));
    }
    let name = def.split_whitespace().next()?;
    let rest = def[name.len()..].trim_start();
    Some((name.to_string(), rest))
}

/// Parse AUTOINCREMENT columns from a CREATE TABLE SQL statement.
///
/// This avoids regex compilation in hot paths and is tolerant of common quoting styles.
fn parse_autoincrement_columns_from_table_sql(sql: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();

    let Some(body) = extract_table_body(sql) else {
        return out;
    };

    for item in split_top_level_commas(body) {
        if item.is_empty() {
            continue;
        }

        let upper = item.to_uppercase();
        if is_table_level_constraint(&upper) {
            continue;
        }

        if !upper.contains("AUTOINCREMENT") {
            continue;
        }
        if !(upper.contains("INTEGER") && upper.contains("PRIMARY") && upper.contains("KEY")) {
            continue;
        }

        if let Some((col_name, _rest)) = take_column_name(item) {
            out.insert(col_name);
        }
    }

    out
}

/// Metadata recovered from a `CREATE INDEX` statement's verbatim SQL.
#[derive(Debug, Clone, Default)]
pub struct ParsedIndexSql {
    /// Index columns (including expression columns, in declaration order)
    pub columns: Vec<IndexColumn>,
    /// Partial-index WHERE clause (verbatim, without the `WHERE` keyword)
    pub where_clause: Option<String>,
}

/// Returns true if `s` is a bare identifier (letters/digits/underscore).
fn is_bare_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Parse a single item of a CREATE INDEX column list into an [`IndexColumn`].
///
/// Named columns may carry `ASC`/`DESC`/`COLLATE <name>` modifiers, which are
/// currently dropped — `IndexColumn` cannot represent per-column direction or
/// collation yet.
// TODO: extend `IndexColumn` with `desc`/`collate` so introspected indexes
// round-trip those modifiers. Until then, comparison cannot churn on them
// because no producer stores them.
fn parse_index_item(item: &str) -> IndexColumn {
    let expression = || IndexColumn {
        value: item.trim().to_string().into(),
        is_expression: true,
    };

    let Some((name, rest)) = take_column_name(item) else {
        return expression();
    };

    // Quoted identifiers were unwrapped by take_column_name; a bare token has
    // to look like an identifier (e.g. `lower(email)` must stay an expression).
    let quoted = matches!(item.trim_start().chars().next(), Some('"' | '`' | '['));
    if !quoted && !is_bare_identifier(&name) {
        return expression();
    }

    // Only ASC/DESC/COLLATE <collation> may follow a plain column reference.
    let mut tokens = rest.split_whitespace();
    loop {
        match tokens.next().map(str::to_ascii_uppercase) {
            None => break,
            Some(t) if t == "ASC" || t == "DESC" => {}
            Some(t) if t == "COLLATE" => {
                if tokens.next().is_none() {
                    return expression();
                }
            }
            Some(_) => return expression(),
        }
    }

    IndexColumn {
        value: name.into(),
        is_expression: false,
    }
}

/// Parse a `CREATE INDEX` statement (from `sqlite_master.sql`) to recover the
/// column list (including expression columns) and any partial-index WHERE
/// clause. This is a tolerant parser, not a full SQL parser.
#[must_use]
pub fn parse_index_sql(sql: &str) -> ParsedIndexSql {
    let mut parsed = ParsedIndexSql::default();

    // Find the first '(' outside quoted identifiers/strings, then its
    // balanced closing paren.
    let mut in_quote: Option<char> = None;
    let mut open: Option<usize> = None;
    for (i, ch) in sql.char_indices() {
        match (in_quote, ch) {
            (Some(q), _) if quote_closer(q) == ch => in_quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"' | '`' | '[') => in_quote = Some(ch),
            (None, '(') => {
                open = Some(i);
                break;
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return parsed;
    };

    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    let mut in_quote: Option<char> = None;
    for (i, ch) in sql.char_indices().skip(open) {
        match (in_quote, ch) {
            (Some(q), _) if quote_closer(q) == ch => in_quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"' | '`' | '[') => in_quote = Some(ch),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return parsed;
    };

    let body = &sql[open + 1..close];
    parsed.columns = split_top_level_commas(body)
        .into_iter()
        .filter(|item| !item.is_empty())
        .map(parse_index_item)
        .collect();

    // Scan the tail (quote-aware) for the WHERE keyword.
    let tail = &sql[close + 1..];
    let mut in_quote: Option<char> = None;
    let bytes = tail.as_bytes();
    for (i, ch) in tail.char_indices() {
        match (in_quote, ch) {
            (Some(q), _) if quote_closer(q) == ch => in_quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"' | '`' | '[') => in_quote = Some(ch),
            (None, 'w' | 'W') => {
                let end = i + 5;
                if end <= tail.len()
                    && tail[i..end].eq_ignore_ascii_case("where")
                    && (i == 0 || !is_ident_byte(bytes[i - 1]))
                    && (end == tail.len() || !is_ident_byte(bytes[end]))
                {
                    let clause = tail[end..].trim().trim_end_matches(';').trim();
                    if !clause.is_empty() {
                        parsed.where_clause = Some(clause.to_string());
                    }
                    break;
                }
            }
            _ => {}
        }
    }

    parsed
}

const fn quote_closer(open: char) -> char {
    match open {
        '[' => ']',
        other => other,
    }
}

const fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Process raw index info into Index entities.
///
/// This variant cannot recover partial-index WHERE clauses or expression
/// columns; prefer [`process_indexes_with_sql`] when the indexes' CREATE SQL
/// (from `sqlite_master`) is available.
#[must_use]
pub fn process_indexes<S: std::hash::BuildHasher>(
    raw_indexes: &[RawIndexInfo],
    index_columns: &[RawIndexColumn],
    _table_sql_map: &std::collections::HashMap<String, String, S>,
) -> Vec<Index> {
    let empty: HashMap<String, String> = HashMap::new();
    process_indexes_with_sql(raw_indexes, index_columns, &empty)
}

/// Process raw index info into Index entities, using each index's own CREATE
/// SQL (keyed by index name) to recover what PRAGMA cannot express:
///
/// - partial-index WHERE clauses (`pragma_index_list` only reports a flag)
/// - expression columns (`pragma_index_xinfo` returns NULL names for them)
#[must_use]
pub fn process_indexes_with_sql<S: std::hash::BuildHasher>(
    raw_indexes: &[RawIndexInfo],
    index_columns: &[RawIndexColumn],
    index_sql_map: &std::collections::HashMap<String, String, S>,
) -> Vec<Index> {
    raw_indexes
        .iter()
        .filter(|idx| idx.origin == "c") // Only CREATE INDEX indexes
        .map(|idx| {
            let parsed = index_sql_map.get(&idx.name).map(|sql| parse_index_sql(sql));

            let mut key_columns: Vec<&RawIndexColumn> = index_columns
                .iter()
                .filter(|c| c.index_name == idx.name && c.key)
                .collect();
            key_columns.sort_by_key(|c| c.seqno);

            let has_expression = key_columns.iter().any(|c| c.name.is_none());
            let columns: Vec<IndexColumn> = match &parsed {
                // Expression columns (or missing xinfo rows): the parsed SQL
                // has the verbatim column list, including expression text.
                Some(parsed)
                    if (has_expression || key_columns.is_empty()) && !parsed.columns.is_empty() =>
                {
                    parsed.columns.clone()
                }
                _ => key_columns
                    .iter()
                    .filter_map(|c| {
                        c.name.clone().map(|name| IndexColumn {
                            value: name.into(),
                            is_expression: false,
                        })
                    })
                    .collect(),
            };

            let where_clause = if idx.partial {
                parsed
                    .as_ref()
                    .and_then(|p| p.where_clause.clone())
                    .map(std::convert::Into::into)
            } else {
                None
            };

            Index {
                table: idx.table.clone().into(),
                name: idx.name.clone().into(),
                columns,
                is_unique: idx.unique,
                where_clause,
                origin: IndexOrigin::Manual,
            }
        })
        .collect()
}

/// Extract unique constraints from pragma index list + `index_xinfo`.
///
/// `SQLite` reports UNIQUE constraints (including inline column UNIQUE and table-level UNIQUE)
/// as indexes with `origin == "u"`. These should be represented as `UniqueConstraint` entities
/// so codegen can emit `#[column(unique)]` for single-column uniques.
#[must_use]
pub fn process_unique_constraints_from_indexes(
    raw_indexes: &[RawIndexInfo],
    index_columns: &[RawIndexColumn],
) -> Vec<UniqueConstraint> {
    use std::borrow::Cow;

    raw_indexes
        .iter()
        .filter(|idx| idx.origin == "u")
        .filter_map(|idx| {
            let mut cols: Vec<(i32, Cow<'static, str>)> = index_columns
                .iter()
                .filter(|c| c.index_name == idx.name && c.key)
                .filter_map(|c| {
                    c.name
                        .as_ref()
                        .map(|name| (c.seqno, Cow::Owned(name.clone())))
                })
                .collect();

            cols.sort_by_key(|(seq, _)| *seq);
            let columns: Vec<Cow<'static, str>> = cols.into_iter().map(|(_, c)| c).collect();
            if columns.is_empty() {
                return None;
            }

            let columns_refs: Vec<&str> = columns.iter().map(std::convert::AsRef::as_ref).collect();
            let name = super::ddl::name_for_unique(&idx.table, &columns_refs);

            Some(UniqueConstraint {
                table: Cow::Owned(idx.table.clone()),
                name: Cow::Owned(name),
                name_explicit: false,
                columns: Cow::Owned(columns),
            })
        })
        .collect()
}

/// Process raw foreign key info into `ForeignKey` entities
#[must_use]
pub fn process_foreign_keys(raw_fks: &[RawForeignKey]) -> Vec<ForeignKey> {
    use std::borrow::Cow;

    // Group by table and id. BTreeMap keeps the emitted FK order (and thus
    // snapshot JSON and generated SQL) deterministic.
    let mut grouped: BTreeMap<(String, i32), Vec<&RawForeignKey>> = BTreeMap::new();

    for fk in raw_fks {
        grouped
            .entry((fk.table.clone(), fk.id))
            .or_default()
            .push(fk);
    }

    grouped
        .into_iter()
        .filter_map(|((table, _id), fks)| {
            let mut fks = fks;
            fks.sort_by_key(|f| f.seq);

            // Groups built via `entry(...).or_default().push(fk)` always have at
            // least one entry, but this makes that invariant explicit in code.
            let first = fks.first()?;

            let columns: Vec<&str> = fks.iter().map(|f| f.from_column.as_str()).collect();
            let columns_to: Vec<&str> = fks.iter().map(|f| f.to_column.as_str()).collect();

            let name = super::ddl::name_for_fk(&table, &columns, &first.to_table, &columns_to);

            // Convert columns to Cow
            let columns_cow: Vec<Cow<'static, str>> = fks
                .iter()
                .map(|f| Cow::Owned(f.from_column.clone()))
                .collect();
            let columns_to_cow: Vec<Cow<'static, str>> = fks
                .iter()
                .map(|f| Cow::Owned(f.to_column.clone()))
                .collect();

            Some(ForeignKey {
                table: table.into(),
                name: name.into(),
                name_explicit: false,
                columns: Cow::Owned(columns_cow),
                table_to: first.to_table.clone().into(),
                columns_to: Cow::Owned(columns_to_cow),
                on_update: Some(first.on_update.clone().into()),
                on_delete: Some(first.on_delete.clone().into()),
            })
        })
        .collect()
}

/// Create primary key constraint from column info
///
/// Note: Primary keys are now extracted directly in `process_columns()` along with columns
/// since the raw column info contains pk field
pub fn create_primary_key(table: &str, pk_columns: Vec<String>) -> PrimaryKey {
    use std::borrow::Cow;

    let name = super::ddl::name_for_pk(table);
    let columns_cow: Vec<Cow<'static, str>> = pk_columns.into_iter().map(Cow::Owned).collect();

    PrimaryKey {
        table: table.to_string().into(),
        name: name.into(),
        name_explicit: false,
        columns: Cow::Owned(columns_cow),
    }
}

/// Create a unique constraint from parsed info
pub fn create_unique_constraint(
    table: &str,
    name: &str,
    columns: Vec<String>,
    name_explicit: bool,
) -> UniqueConstraint {
    use std::borrow::Cow;

    let columns_cow: Vec<Cow<'static, str>> = columns.into_iter().map(Cow::Owned).collect();

    UniqueConstraint {
        table: Cow::Owned(table.to_string()),
        name: Cow::Owned(name.to_string()),
        name_explicit,
        columns: Cow::Owned(columns_cow),
    }
}

/// Extract unique constraints from parsed table info
#[must_use]
pub fn process_unique_constraints_from_parsed(
    table: &str,
    parsed_uniques: &[super::ddl::ParsedUnique],
) -> Vec<UniqueConstraint> {
    use std::borrow::Cow;

    parsed_uniques
        .iter()
        .map(|parsed| {
            let columns_refs: Vec<&str> = parsed
                .columns
                .iter()
                .map(std::string::String::as_str)
                .collect();
            let (name, name_explicit) = parsed.name.as_ref().map_or_else(
                || (super::ddl::name_for_unique(table, &columns_refs), false),
                |n| (n.clone(), true),
            );
            let columns_cow: Vec<Cow<'static, str>> = parsed
                .columns
                .iter()
                .map(|c| Cow::Owned(c.clone()))
                .collect();

            UniqueConstraint {
                table: table.to_string().into(),
                name: name.into(),
                name_explicit,
                columns: Cow::Owned(columns_cow),
            }
        })
        .collect()
}

/// SQL queries for `SQLite` introspection
pub mod queries {
    /// Query to get all tables
    pub const TABLES_QUERY: &str = r"
        SELECT name, sql
        FROM sqlite_master
        WHERE type = 'table'
          AND name != '__drizzle_migrations'
          AND name NOT LIKE '\_cf\_%' ESCAPE '\'
          AND name NOT LIKE '\_litestream\_%' ESCAPE '\'
          AND name NOT LIKE 'libsql\_%' ESCAPE '\'
          AND name NOT LIKE 'sqlite\_%' ESCAPE '\'
          AND name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY name COLLATE NOCASE
    ";

    /// Query to get all columns for a table using `pragma_table_xinfo`
    pub const COLUMNS_QUERY: &str = r#"
        SELECT 
            m.name as "table", 
            p.cid as "cid",
            p.name as "name", 
            p.type as "columnType",
            p."notnull" as "notNull", 
            p.dflt_value as "defaultValue",
            p.pk as pk,
            p.hidden as hidden,
            m.sql
        FROM sqlite_master AS m 
            JOIN pragma_table_xinfo(m.name) AS p
        WHERE 
            m.type = 'table'
            AND m.tbl_name != '__drizzle_migrations' 
            AND m.tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY p.cid
    "#;

    /// Query to get all views
    pub const VIEWS_QUERY: &str = r"
        SELECT name, sql
        FROM sqlite_master
        WHERE type = 'view'
          AND name != '__drizzle_migrations'
          AND name NOT LIKE '\_cf\_%' ESCAPE '\'
          AND name NOT LIKE '\_litestream\_%' ESCAPE '\'
          AND name NOT LIKE 'libsql\_%' ESCAPE '\'
          AND name NOT LIKE 'sqlite\_%' ESCAPE '\'
          AND name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY name COLLATE NOCASE
    ";

    /// Query to get all columns for views using `pragma_table_xinfo`
    pub const VIEW_COLUMNS_QUERY: &str = r#"
        SELECT
            m.name as "table",
            p.cid as "cid",
            p.name as "name",
            p.type as "columnType",
            p."notnull" as "notNull",
            p.dflt_value as "defaultValue",
            p.pk as pk,
            p.hidden as hidden,
            m.sql
        FROM sqlite_master AS m
            JOIN pragma_table_xinfo(m.name) AS p
        WHERE
            m.type = 'view'
            AND m.tbl_name != '__drizzle_migrations'
            AND m.tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY m.name, p.cid
    "#;

    /// Query all indexes in one round trip.
    pub const INDEXES_QUERY: &str = r#"
        SELECT
            m.name AS "table",
            p.name,
            p."unique",
            p.origin,
            p.partial
        FROM sqlite_master AS m
            JOIN pragma_index_list(m.name) AS p
        WHERE m.type = 'table'
            AND m.tbl_name != '__drizzle_migrations'
            AND m.tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY m.name COLLATE NOCASE, p.seq
    "#;

    /// Query the verbatim CREATE INDEX SQL for every user-created index.
    ///
    /// Feed the resulting `(name, sql)` rows into
    /// [`super::process_indexes_with_sql`] to recover partial-index WHERE
    /// clauses and expression columns, which PRAGMA cannot express. Auto
    /// indexes (`sqlite_autoindex_*`) have NULL sql and are excluded.
    pub const INDEX_SQL_QUERY: &str = r"
        SELECT name, sql
        FROM sqlite_master
        WHERE type = 'index'
          AND sql IS NOT NULL
          AND tbl_name != '__drizzle_migrations'
          AND tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
          AND tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
          AND tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
          AND tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
          AND tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY name COLLATE NOCASE
    ";

    /// Query all indexed columns in one round trip.
    pub const INDEX_COLUMNS_QUERY: &str = r#"
        SELECT
            indexes.name AS index_name,
            columns.seqno,
            columns.cid,
            columns.name,
            columns."desc",
            columns.coll,
            columns."key"
        FROM sqlite_master AS m
            JOIN pragma_index_list(m.name) AS indexes
            JOIN pragma_index_xinfo(indexes.name) AS columns
        WHERE m.type = 'table'
            AND m.tbl_name != '__drizzle_migrations'
            AND m.tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY m.name COLLATE NOCASE, indexes.seq, columns.seqno
    "#;

    /// Query all foreign keys in one round trip.
    pub const FOREIGN_KEYS_QUERY: &str = r#"
        SELECT
            m.name AS "table",
            p.id,
            p.seq,
            p."table" AS to_table,
            p."from" AS from_column,
            p."to" AS to_column,
            p.on_update,
            p.on_delete,
            p."match"
        FROM sqlite_master AS m
            JOIN pragma_foreign_key_list(m.name) AS p
        WHERE m.type = 'table'
            AND m.tbl_name != '__drizzle_migrations'
            AND m.tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY m.name COLLATE NOCASE, p.id, p.seq
    "#;
}

/// Parse a view SQL to extract the definition.
///
/// Finds the top-level `AS` keyword — quote-aware (so a view named
/// `"my as view"` doesn't split early) and paren-aware (so a column-name list
/// `CREATE VIEW v(a, b) AS ...` is skipped over).
#[must_use]
pub fn parse_view_sql(sql: &str) -> Option<String> {
    let bytes = sql.as_bytes();
    let mut in_quote: Option<char> = None;
    let mut depth = 0i32;

    for (i, ch) in sql.char_indices() {
        match (in_quote, ch) {
            (Some(q), _) if quote_closer(q) == ch => in_quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"' | '`' | '[') => in_quote = Some(ch),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, 'a' | 'A') if depth == 0 => {
                let end = i + 2;
                if end <= sql.len()
                    && sql[i..end].eq_ignore_ascii_case("as")
                    && (i == 0 || !is_ident_byte(bytes[i - 1]))
                    && (end == sql.len() || !is_ident_byte(bytes[end]))
                {
                    let definition = sql[end..].trim().trim_end_matches(';').trim();
                    if definition.is_empty() {
                        return None;
                    }
                    return Some(definition.to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse the `AS (expr) [STORED|VIRTUAL]` tail of a column definition.
///
/// `rest` should point just past the column name. Returns `(expression, gen_type)`
/// or `None` if the column definition is not a generated column.
fn parse_generated_tail(rest: &str) -> Option<(String, GeneratedType)> {
    let upper_rest = rest.to_uppercase();
    let as_pos = upper_rest.find(" AS ")?;
    let after_as = &rest[as_pos + 4..];
    let expr_start_rel = after_as.find('(')?;
    let expr_start = as_pos + 4 + expr_start_rel;

    let mut expr_depth = 0i32;
    let mut expr_end: Option<usize> = None;
    for (i, ch) in rest.char_indices().skip(expr_start) {
        match ch {
            '(' => expr_depth += 1,
            ')' => {
                expr_depth -= 1;
                if expr_depth == 0 {
                    expr_end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let expr_end = expr_end?;

    let expression = rest[expr_start + 1..expr_end].trim().to_string();
    let after_expr = rest[expr_end + 1..].to_uppercase();
    let gen_type = if after_expr.contains("STORED") {
        GeneratedType::Stored
    } else {
        GeneratedType::Virtual
    };
    Some((expression, gen_type))
}

/// Parse generated columns from a CREATE TABLE SQL statement.
///
/// Returns a map keyed by `"table:column"` matching the key format used by `process_columns`.
///
/// This is intentionally a small, tolerant parser (not a full SQL parser). It handles common
/// `SQLite` syntax for generated columns:
/// - `col TYPE GENERATED ALWAYS AS (expr) STORED`
/// - `col TYPE GENERATED ALWAYS AS (expr) VIRTUAL`
#[must_use]
pub fn parse_generated_columns_from_table_sql(
    table: &str,
    sql: &str,
) -> HashMap<String, ParsedGenerated> {
    let mut out: HashMap<String, ParsedGenerated> = HashMap::new();

    let Some(body) = extract_table_body(sql) else {
        return out;
    };

    for item in split_top_level_commas(body) {
        if item.is_empty() {
            continue;
        }
        let upper = item.to_uppercase();
        if !upper.contains("GENERATED") || is_table_level_constraint(&upper) {
            continue;
        }

        let Some((col_name, rest)) = take_column_name(item) else {
            continue;
        };
        let Some((expression, gen_type)) = parse_generated_tail(rest) else {
            continue;
        };

        out.insert(
            format!("{table}:{col_name}"),
            ParsedGenerated {
                expression,
                gen_type,
            },
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_table_filter() {
        assert!(!system_table_filter("sqlite_master"));
        assert!(!system_table_filter("__drizzle_migrations"));
        assert!(!system_table_filter("_cf_something"));
        assert!(system_table_filter("users"));
        assert!(system_table_filter("posts"));
    }

    #[test]
    fn test_parse_view_sql() {
        let sql = "CREATE VIEW active_users AS SELECT * FROM users WHERE active = 1";
        let definition = parse_view_sql(sql);
        assert_eq!(
            definition,
            Some("SELECT * FROM users WHERE active = 1".to_string())
        );
    }

    #[test]
    fn test_parse_autoincrement_columns_from_table_sql() {
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)";
        let cols = parse_autoincrement_columns_from_table_sql(sql);
        assert!(cols.contains("id"));
        assert!(!cols.contains("name"));
    }

    #[test]
    fn test_parse_generated_columns_from_table_sql() {
        let sql = r#"
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  first TEXT,
  last TEXT,
  full TEXT GENERATED ALWAYS AS (first || ' ' || last) VIRTUAL,
  total INT GENERATED ALWAYS AS ((id + 1) * 2) STORED
);
"#;
        let map = parse_generated_columns_from_table_sql("users", sql);
        let full = map.get("users:full").expect("full generated");
        assert_eq!(full.gen_type, GeneratedType::Virtual);
        assert!(full.expression.contains("first"));

        let total = map.get("users:total").expect("total generated");
        assert_eq!(total.gen_type, GeneratedType::Stored);
        assert!(total.expression.contains("id"));
    }

    #[test]
    fn test_parse_table_options_ignores_body_content() {
        // Column named `strict` must not trigger the STRICT option.
        let sql = "CREATE TABLE t (strict TEXT, without_rowid INTEGER)";
        assert_eq!(parse_table_options(sql), (false, false));

        // String content in the body must not trigger options either.
        let sql = "CREATE TABLE t (note TEXT DEFAULT 'WITHOUT ROWID STRICT')";
        assert_eq!(parse_table_options(sql), (false, false));

        let sql = "CREATE TABLE t (id INTEGER) STRICT";
        assert_eq!(parse_table_options(sql), (true, false));

        let sql = "CREATE TABLE t (id INTEGER) WITHOUT ROWID";
        assert_eq!(parse_table_options(sql), (false, true));

        let sql = "CREATE TABLE t (id INTEGER) STRICT, WITHOUT ROWID;";
        assert_eq!(parse_table_options(sql), (true, true));

        let sql = "CREATE TABLE t (id INTEGER) WITHOUT ROWID, STRICT;";
        assert_eq!(parse_table_options(sql), (true, true));
    }

    #[test]
    fn test_parse_index_sql_recovers_where_and_expressions() {
        let parsed =
            parse_index_sql("CREATE INDEX idx_c_positive ON multi_indexed(col_c) WHERE col_c > 0");
        assert_eq!(parsed.columns.len(), 1);
        assert_eq!(parsed.columns[0].value, "col_c");
        assert!(!parsed.columns[0].is_expression);
        assert_eq!(parsed.where_clause.as_deref(), Some("col_c > 0"));

        let parsed = parse_index_sql("CREATE UNIQUE INDEX i ON t(lower(email), name)");
        assert_eq!(parsed.columns.len(), 2);
        assert_eq!(parsed.columns[0].value, "lower(email)");
        assert!(parsed.columns[0].is_expression);
        assert_eq!(parsed.columns[1].value, "name");
        assert!(!parsed.columns[1].is_expression);
        assert!(parsed.where_clause.is_none());

        // Quoted identifiers and ASC/DESC/COLLATE modifiers stay named columns.
        let parsed =
            parse_index_sql("CREATE INDEX i ON t(`email` DESC, \"name\" COLLATE NOCASE ASC)");
        assert_eq!(parsed.columns.len(), 2);
        assert_eq!(parsed.columns[0].value, "email");
        assert!(!parsed.columns[0].is_expression);
        assert_eq!(parsed.columns[1].value, "name");
        assert!(!parsed.columns[1].is_expression);

        // WHERE inside a string literal in an expression must not be picked up.
        let parsed = parse_index_sql("CREATE INDEX i ON t(coalesce(kind, 'WHERE x'))");
        assert!(parsed.where_clause.is_none());
        assert_eq!(parsed.columns.len(), 1);
        assert!(parsed.columns[0].is_expression);
    }

    #[test]
    fn test_process_columns_keeps_generated_columns() {
        let table_sql = "CREATE TABLE g (id INTEGER PRIMARY KEY, v TEXT GENERATED ALWAYS AS (id + 1) VIRTUAL, s TEXT GENERATED ALWAYS AS (id + 2) STORED)";
        let raw = vec![
            RawColumnInfo {
                table: "g".to_string(),
                cid: 0,
                name: "id".to_string(),
                column_type: "INTEGER".to_string(),
                not_null: false,
                default_value: None,
                pk: 1,
                hidden: 0,
                sql: Some(table_sql.to_string()),
            },
            RawColumnInfo {
                table: "g".to_string(),
                cid: 1,
                name: "v".to_string(),
                column_type: "TEXT".to_string(),
                not_null: false,
                default_value: None,
                pk: 0,
                hidden: 2, // VIRTUAL generated
                sql: Some(table_sql.to_string()),
            },
            RawColumnInfo {
                table: "g".to_string(),
                cid: 2,
                name: "s".to_string(),
                column_type: "TEXT".to_string(),
                not_null: false,
                default_value: None,
                pk: 0,
                hidden: 3, // STORED generated
                sql: Some(table_sql.to_string()),
            },
        ];

        let generated = parse_generated_columns_from_table_sql("g", table_sql);
        let pk_columns: HashSet<(String, String)> = HashSet::new();
        let (columns, _pks) = process_columns(&raw, &generated, &pk_columns);

        assert_eq!(columns.len(), 3, "generated columns must be kept");
        let v = columns.iter().find(|c| c.name == "v").expect("v column");
        let v_generated = v.generated.as_ref().expect("v generated info");
        assert_eq!(v_generated.gen_type, GeneratedType::Virtual);
        assert_eq!(v_generated.expression, "id + 1");
        let s = columns.iter().find(|c| c.name == "s").expect("s column");
        let s_generated = s.generated.as_ref().expect("s generated info");
        assert_eq!(s_generated.gen_type, GeneratedType::Stored);
        assert_eq!(s_generated.expression, "id + 2");
    }

    #[test]
    fn test_composite_pk_columns_ordered_by_pk_position() {
        // PRIMARY KEY (b, a): cid order is a, b but pk positions are b=1, a=2.
        let raw = vec![
            RawColumnInfo {
                table: "t".to_string(),
                cid: 0,
                name: "a".to_string(),
                column_type: "INTEGER".to_string(),
                not_null: true,
                default_value: None,
                pk: 2,
                hidden: 0,
                sql: None,
            },
            RawColumnInfo {
                table: "t".to_string(),
                cid: 1,
                name: "b".to_string(),
                column_type: "INTEGER".to_string(),
                not_null: true,
                default_value: None,
                pk: 1,
                hidden: 0,
                sql: None,
            },
        ];
        let generated = HashMap::new();
        let pk_columns: HashSet<(String, String)> = HashSet::new();
        let (_cols, pks) = process_columns(&raw, &generated, &pk_columns);
        assert_eq!(pks.len(), 1);
        let cols: Vec<&str> = pks[0].columns.iter().map(AsRef::as_ref).collect();
        assert_eq!(cols, vec!["b", "a"]);
    }

    #[test]
    fn test_parse_view_sql_is_quote_aware() {
        let sql = r#"CREATE VIEW "my as view" AS SELECT * FROM users"#;
        assert_eq!(parse_view_sql(sql), Some("SELECT * FROM users".to_string()));

        // Column-name list before AS is skipped over.
        let sql = "CREATE VIEW v(a, b) AS SELECT 1, 2";
        assert_eq!(parse_view_sql(sql), Some("SELECT 1, 2".to_string()));

        // `AS` must be a word, not a substring.
        let sql = "CREATE VIEW basics AS SELECT 1";
        assert_eq!(parse_view_sql(sql), Some("SELECT 1".to_string()));
    }

    #[test]
    fn set_based_metadata_queries_cover_all_tables_and_indexes() {
        let connection = rusqlite::Connection::open_in_memory().expect("open SQLite");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE parents(id INTEGER PRIMARY KEY);
                 CREATE TABLE children(
                    id INTEGER PRIMARY KEY,
                    parent_id INTEGER NOT NULL,
                    email TEXT UNIQUE,
                    FOREIGN KEY(parent_id) REFERENCES parents(id)
                 );
                 CREATE INDEX children_parent_idx ON children(parent_id);",
            )
            .expect("create schema");

        let index_count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM ({})", queries::INDEXES_QUERY),
                [],
                |row| row.get(0),
            )
            .expect("query all indexes");
        let indexed_column_count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM ({})", queries::INDEX_COLUMNS_QUERY),
                [],
                |row| row.get(0),
            )
            .expect("query all index columns");
        let foreign_key_count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM ({})", queries::FOREIGN_KEYS_QUERY),
                [],
                |row| row.get(0),
            )
            .expect("query all foreign keys");

        assert!(index_count >= 2);
        assert!(indexed_column_count >= 2);
        assert_eq!(foreign_key_count, 1);
    }
}
