//! Statement-level reconciliation for interrupted ("dirty") migrations.
//!
//! On engines and code paths without reliable transactional DDL, a process
//! killed part-way through a migration leaves some statements applied and the
//! rest not. Two-phase tracking (see
//! [`Migrations::record_migration_started_sql`](crate::Migrations::record_migration_started_sql))
//! records that fact as a tracking row with a `NULL` `applied_at`, and the next
//! `migrate()` refuses to continue rather than blindly re-running DDL that
//! would fail with `table already exists`.
//!
//! This module turns that stalemate into a plan. Given the migration's
//! statements and a snapshot of the *live* database objects
//! ([`Catalog`]), [`plan`] classifies each statement:
//!
//! * **Skip** — introspection proves the statement's effect is already
//!   present (`CREATE TABLE` whose table exists with the same columns,
//!   `CREATE [UNIQUE] INDEX` whose index exists over the same columns,
//!   `CREATE VIEW` with the same definition, `CREATE TYPE ... AS ENUM` with
//!   the same labels).
//! * **Execute** — the object provably does not exist yet, so the statement
//!   (and everything after it) still has to run.
//! * **Unresolvable** — the statement is not a provable `CREATE` (an `ALTER`,
//!   `INSERT`, `DROP`, data backfill, or anything unparseable) *and* it sits
//!   inside the region that may already have run. Repair refuses to guess and
//!   asks for manual resolution.
//!
//! Statements are ordered, so the classification is a two-phase walk: while
//! still inside the proven-applied prefix an unclassifiable statement is fatal
//! (it might or might not have run); once a statement is proven *missing*,
//! everything after it is known not to have run and is simply executed.
//!
//! The database-facing half is deliberately data-only: drivers run the
//! constant queries in [`sqlite`] / [`postgres`], fold the rows into a
//! [`Catalog`] with that module's [`catalog`](sqlite::catalog) constructor,
//! and hand it to [`plan`]. Nothing here touches a connection, so the whole
//! planner is unit-testable and shared by every driver.

use crate::migrator::{Migration, MigratorError, quote_identifier};
use drizzle_types::Dialect;

/// Kind of live database object recorded in a [`Catalog`] snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// Base table.
    Table,
    /// Index (unique or not — see [`CatalogObject::unique`]).
    Index,
    /// View.
    View,
    /// Trigger.
    Trigger,
    /// `PostgreSQL` enum type.
    Enum,
}

/// A single object observed in the live database.
#[derive(Debug, Clone)]
pub struct CatalogObject {
    /// What kind of object this is.
    pub kind: ObjectKind,
    /// Schema the object lives in (`None` for `SQLite`).
    pub schema: Option<String>,
    /// Object name, unqualified.
    pub name: String,
    /// Stored DDL text when the engine keeps one (`sqlite_master.sql`,
    /// `pg_indexes.indexdef`). `None` when it does not.
    pub sql: Option<String>,
    /// Ordered members: column names for tables and indexes, labels for enums.
    pub members: Vec<String>,
    /// Whether an index is unique. Meaningless for other kinds.
    pub unique: bool,
}

/// Snapshot of the live database objects repair needs to reason about.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    /// Every observed object.
    pub objects: Vec<CatalogObject>,
}

impl Catalog {
    /// Create an empty snapshot.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    /// Record an object.
    pub fn push(&mut self, object: CatalogObject) {
        self.objects.push(object);
    }

    /// Look up an object by kind and (optionally schema-qualified) name.
    ///
    /// An unqualified lookup prefers `public` (the `PostgreSQL` default
    /// `search_path` head) and otherwise accepts a unique name match.
    #[must_use]
    pub fn find(
        &self,
        kind: ObjectKind,
        schema: Option<&str>,
        name: &str,
    ) -> Option<&CatalogObject> {
        let same_name =
            |object: &&CatalogObject| object.kind == kind && object.name.eq_ignore_ascii_case(name);

        if let Some(schema) = schema {
            return self.objects.iter().find(|object| {
                same_name(object)
                    && object
                        .schema
                        .as_deref()
                        .is_some_and(|live| live.eq_ignore_ascii_case(schema))
            });
        }

        if let Some(object) = self.objects.iter().find(|object| {
            same_name(object)
                && object
                    .schema
                    .as_deref()
                    .is_none_or(|live| live.eq_ignore_ascii_case("public"))
        }) {
            return Some(object);
        }

        let mut matches = self.objects.iter().filter(same_name);
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }
}

/// What a migration statement creates, when that is decidable from its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementTarget {
    /// `CREATE TABLE [IF NOT EXISTS] <name> (<columns>)`.
    Table {
        /// Qualifying schema, when the statement carries one.
        schema: Option<String>,
        /// Table name.
        name: String,
        /// Declared column names, in order (table constraints excluded).
        columns: Vec<String>,
    },
    /// `CREATE [UNIQUE] INDEX [CONCURRENTLY] [IF NOT EXISTS] <name> ON ...`.
    Index {
        /// Qualifying schema, when the statement carries one.
        schema: Option<String>,
        /// Index name.
        name: String,
        /// Whether the index is unique.
        unique: bool,
        /// Leading token of each indexed element, in order.
        columns: Vec<String>,
    },
    /// `CREATE VIEW <name> AS ...`.
    View {
        /// Qualifying schema, when the statement carries one.
        schema: Option<String>,
        /// View name.
        name: String,
    },
    /// `CREATE TYPE <name> AS ENUM (<values>)`.
    Enum {
        /// Qualifying schema, when the statement carries one.
        schema: Option<String>,
        /// Type name.
        name: String,
        /// Enum labels, in declaration order.
        values: Vec<String>,
    },
    /// Anything else. Never provable, therefore never skippable.
    Unclassified,
}

impl StatementTarget {
    /// Whether this statement kind can ever be proven already-applied.
    #[must_use]
    pub const fn is_classified(&self) -> bool {
        !matches!(self, Self::Unclassified)
    }
}

/// What repair decided to do with one statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    /// Introspection proves the effect is already present.
    Skip {
        /// Human-readable justification, surfaced in `--repair` output.
        reason: String,
    },
    /// The statement still has to run.
    Execute,
    /// Repair cannot prove anything; a human has to resolve it.
    Unresolvable {
        /// Why the statement could not be reconciled.
        reason: String,
    },
}

/// One statement of a dirty migration plus its repair decision.
#[derive(Debug, Clone)]
pub struct Step {
    /// Zero-based index into the migration's statement list.
    pub index: usize,
    /// The statement SQL.
    pub sql: String,
    /// What repair decided.
    pub disposition: Disposition,
}

/// The full reconciliation plan for one dirty migration.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Migration tag (folder name) being repaired.
    pub tag: String,
    /// Dialect controls identifier quoting in manual recovery instructions.
    pub dialect: Dialect,
    /// Per-statement decisions, in statement order.
    pub steps: Vec<Step>,
}

impl Plan {
    /// Statements repair refused to reconcile.
    #[must_use]
    pub fn unresolvable(&self) -> Vec<&Step> {
        self.steps
            .iter()
            .filter(|step| matches!(step.disposition, Disposition::Unresolvable { .. }))
            .collect()
    }

    /// Number of statements proven already applied.
    #[must_use]
    pub fn skipped_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| matches!(step.disposition, Disposition::Skip { .. }))
            .count()
    }

    /// Whether every statement was reconciled (nothing needs a human).
    #[must_use]
    pub fn is_resolvable(&self) -> bool {
        self.unresolvable().is_empty()
    }

    /// The statements that still need to run, in order.
    ///
    /// # Errors
    ///
    /// Returns [`MigratorError::UnrepairableMigration`] listing every statement
    /// that could not be reconciled, together with the manual completion SQL.
    pub fn into_executable(self, table_ident: &str) -> Result<Vec<String>, MigratorError> {
        let blockers = self.unresolvable();
        if !blockers.is_empty() {
            let mut message = format!(
                "cannot repair migration `{}`: {} statement(s) could not be reconciled against \
                 the live schema.\n\
                 Repair only skips CREATE TABLE / CREATE [UNIQUE] INDEX / CREATE VIEW / \
                 CREATE TYPE ... AS ENUM statements that introspection proves are already \
                 present; anything else inside the interrupted region must be resolved by hand.",
                self.tag,
                blockers.len()
            );
            for step in blockers {
                let reason = match &step.disposition {
                    Disposition::Unresolvable { reason } => reason.as_str(),
                    _ => unreachable!("filtered to unresolvable"),
                };
                message.push_str(&format!(
                    "\n  statement {}: {reason}\n    {}",
                    step.index + 1,
                    single_line(&step.sql)
                ));
            }
            let name_column = quote_identifier(self.dialect, "name");
            let applied_at_column = quote_identifier(self.dialect, "applied_at");
            message.push_str(&format!(
                "\nResolve those statements manually, then mark the migration complete:\n  \
                 UPDATE {table_ident} SET {applied_at_column} = CURRENT_TIMESTAMP WHERE {name_column} = '{}';\n\
                 …or discard the marker and re-run from scratch:\n  \
                 DELETE FROM {table_ident} WHERE {name_column} = '{}';",
                self.tag.replace('\'', "''"),
                self.tag.replace('\'', "''"),
            ));
            return Err(MigratorError::UnrepairableMigration(message));
        }

        Ok(self
            .steps
            .into_iter()
            .filter(|step| step.disposition == Disposition::Execute)
            .map(|step| step.sql)
            .collect())
    }
}

/// Classify every statement of `migration` against the live database.
///
/// See the [module docs](self) for the two-phase walk this implements.
#[must_use]
pub fn plan(dialect: Dialect, migration: &Migration, catalog: &Catalog) -> Plan {
    let mut steps = Vec::new();
    // True while every statement so far was proven already-applied. Once a
    // statement is proven *missing*, nothing after it can have run.
    let mut in_applied_prefix = true;

    for (index, sql) in migration.statements().iter().enumerate() {
        if sql.trim().is_empty() {
            continue;
        }

        let target = classify_statement(dialect, sql);
        let disposition = match &target {
            StatementTarget::Unclassified => {
                if in_applied_prefix {
                    Disposition::Unresolvable {
                        reason: "not a provable CREATE statement, and it may already have run \
                                 before the interruption"
                            .to_string(),
                    }
                } else {
                    Disposition::Execute
                }
            }
            _ => match reconcile(dialect, &target, sql, catalog) {
                Reconciled::Present => Disposition::Skip {
                    reason: describe_present(&target),
                },
                Reconciled::Absent => {
                    in_applied_prefix = false;
                    Disposition::Execute
                }
                Reconciled::Conflict(reason) => Disposition::Unresolvable { reason },
            },
        };

        steps.push(Step {
            index,
            sql: sql.clone(),
            disposition,
        });
    }

    Plan {
        tag: migration.tag().to_string(),
        dialect,
        steps,
    }
}

enum Reconciled {
    /// The live database already has an equivalent object.
    Present,
    /// The object does not exist; the statement still has to run.
    Absent,
    /// An object with that name exists but is not equivalent.
    Conflict(String),
}

fn reconcile(
    dialect: Dialect,
    target: &StatementTarget,
    sql: &str,
    catalog: &Catalog,
) -> Reconciled {
    match target {
        StatementTarget::Table {
            schema,
            name,
            columns,
        } => {
            let Some(object) = catalog.find(ObjectKind::Table, schema.as_deref(), name) else {
                return Reconciled::Absent;
            };
            if object_matches_sql(dialect, object, sql) || members_match(&object.members, columns) {
                Reconciled::Present
            } else {
                Reconciled::Conflict(format!(
                    "table `{name}` already exists but its columns ({}) do not match the ones this \
                     statement creates ({})",
                    join_or_unknown(&object.members),
                    join_or_unknown(columns)
                ))
            }
        }
        StatementTarget::Index {
            schema,
            name,
            unique,
            columns,
        } => {
            let Some(object) = catalog.find(ObjectKind::Index, schema.as_deref(), name) else {
                return Reconciled::Absent;
            };
            // Only meaningful once we actually know something about the live
            // index; an object with neither DDL text nor columns falls through
            // to the (failing) equivalence check below.
            let have_evidence = object.sql.is_some() || !object.members.is_empty();
            if have_evidence && object.unique != *unique {
                return Reconciled::Conflict(format!(
                    "index `{name}` already exists but its uniqueness differs (live: {}, \
                     statement: {})",
                    object.unique, unique
                ));
            }
            if object_matches_sql(dialect, object, sql) || members_match(&object.members, columns) {
                Reconciled::Present
            } else {
                Reconciled::Conflict(format!(
                    "index `{name}` already exists but covers ({}) instead of ({})",
                    join_or_unknown(&object.members),
                    join_or_unknown(columns)
                ))
            }
        }
        StatementTarget::View { schema, name } => {
            let Some(object) = catalog.find(ObjectKind::View, schema.as_deref(), name) else {
                return Reconciled::Absent;
            };
            if object_matches_sql(dialect, object, sql) {
                Reconciled::Present
            } else {
                Reconciled::Conflict(format!(
                    "view `{name}` already exists but its definition differs from this statement"
                ))
            }
        }
        StatementTarget::Enum {
            schema,
            name,
            values,
        } => {
            let Some(object) = catalog.find(ObjectKind::Enum, schema.as_deref(), name) else {
                return Reconciled::Absent;
            };
            if object.members == *values {
                Reconciled::Present
            } else {
                Reconciled::Conflict(format!(
                    "enum type `{name}` already exists with labels ({}) instead of ({})",
                    join_or_unknown(&object.members),
                    join_or_unknown(values)
                ))
            }
        }
        StatementTarget::Unclassified => Reconciled::Conflict("unclassified statement".to_string()),
    }
}

fn object_matches_sql(dialect: Dialect, object: &CatalogObject, sql: &str) -> bool {
    object.sql.as_deref().is_some_and(|live_sql| {
        canonical_tokens(dialect, live_sql) == canonical_tokens(dialect, sql)
    })
}

fn members_match(live: &[String], target: &[String]) -> bool {
    !live.is_empty()
        && live.len() == target.len()
        && live
            .iter()
            .zip(target)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn join_or_unknown(values: &[String]) -> String {
    if values.is_empty() {
        "unknown".to_string()
    } else {
        values.join(", ")
    }
}

fn describe_present(target: &StatementTarget) -> String {
    match target {
        StatementTarget::Table { name, .. } => {
            format!("table `{name}` already exists with a matching definition")
        }
        StatementTarget::Index { name, .. } => {
            format!("index `{name}` already exists with a matching definition")
        }
        StatementTarget::View { name, .. } => {
            format!("view `{name}` already exists with a matching definition")
        }
        StatementTarget::Enum { name, .. } => {
            format!("enum type `{name}` already exists with matching labels")
        }
        StatementTarget::Unclassified => "already applied".to_string(),
    }
}

fn single_line(sql: &str) -> String {
    let collapsed = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 160 {
        let truncated: String = collapsed.chars().take(157).collect();
        format!("{truncated}...")
    } else {
        collapsed
    }
}

// =============================================================================
// Statement classification
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// Unquoted identifier or keyword, already lowercased.
    Word(String),
    /// Quoted identifier (`"x"`, `` `x` ``, `[x]`), inner text preserved.
    Quoted(String),
    /// String literal, inner text preserved.
    Str(String),
    /// Single punctuation character.
    Punct(char),
}

impl Tok {
    fn ident(&self) -> Option<&str> {
        match self {
            Self::Word(value) | Self::Quoted(value) => Some(value),
            _ => None,
        }
    }

    fn is_word(&self, keyword: &str) -> bool {
        matches!(self, Self::Word(value) if value == keyword)
    }

    fn is_punct(&self, character: char) -> bool {
        matches!(self, Self::Punct(value) if *value == character)
    }

    fn canonical(&self) -> String {
        match self {
            Self::Word(value) => value.clone(),
            Self::Quoted(value) => value.to_lowercase(),
            Self::Str(value) => format!("'{value}'"),
            Self::Punct(value) => value.to_string(),
        }
    }
}

/// Tokenize SQL far enough to recognize `CREATE` headers.
///
/// Comments are dropped; quoted identifiers and string literals keep their
/// inner text; everything else becomes a lowercased word or a punctuation
/// character.
fn tokenize(sql: &str) -> Vec<Tok> {
    let bytes: Vec<char> = sql.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let ch = bytes[index];

        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        // Comments
        if ch == '-' && bytes.get(index + 1) == Some(&'-') {
            while index < bytes.len() && bytes[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if ch == '/' && bytes.get(index + 1) == Some(&'*') {
            index += 2;
            while index < bytes.len() {
                if bytes[index] == '*' && bytes.get(index + 1) == Some(&'/') {
                    index += 2;
                    break;
                }
                index += 1;
            }
            continue;
        }

        // Quoted identifiers and string literals
        if let Some((closing, is_string)) = match ch {
            '"' => Some(('"', false)),
            '`' => Some(('`', false)),
            '[' => Some((']', false)),
            '\'' => Some(('\'', true)),
            _ => None,
        } {
            index += 1;
            let mut value = String::new();
            while index < bytes.len() {
                if bytes[index] == closing {
                    // Doubled delimiter is an escape (except for `[...]`).
                    if closing != ']' && bytes.get(index + 1) == Some(&closing) {
                        value.push(closing);
                        index += 2;
                        continue;
                    }
                    index += 1;
                    break;
                }
                value.push(bytes[index]);
                index += 1;
            }
            tokens.push(if is_string {
                Tok::Str(value)
            } else {
                Tok::Quoted(value)
            });
            continue;
        }

        if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            let mut value = String::new();
            while index < bytes.len()
                && (bytes[index].is_alphanumeric() || bytes[index] == '_' || bytes[index] == '$')
            {
                value.push(bytes[index]);
                index += 1;
            }
            tokens.push(Tok::Word(value.to_lowercase()));
            continue;
        }

        tokens.push(Tok::Punct(ch));
        index += 1;
    }

    tokens
}

/// Canonical token stream used for "is this the same DDL?" comparisons.
///
/// Drops the noise that never changes an object's shape: trailing semicolons,
/// `IF NOT EXISTS`, and `CONCURRENTLY`.
fn canonical_tokens(_dialect: Dialect, sql: &str) -> Vec<String> {
    let tokens = tokenize(sql);
    let mut out = Vec::with_capacity(tokens.len());
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].is_word("if")
            && tokens.get(index + 1).is_some_and(|t| t.is_word("not"))
            && tokens.get(index + 2).is_some_and(|t| t.is_word("exists"))
        {
            index += 3;
            continue;
        }
        if tokens[index].is_word("concurrently") {
            index += 1;
            continue;
        }
        if tokens[index].is_punct(';') && index + 1 == tokens.len() {
            break;
        }
        out.push(tokens[index].canonical());
        index += 1;
    }

    out
}

/// Table-level constraint keywords that start a non-column entry in a
/// `CREATE TABLE` body.
const CONSTRAINT_KEYWORDS: [&str; 8] = [
    "constraint",
    "primary",
    "unique",
    "foreign",
    "check",
    "exclude",
    "like",
    "period",
];

/// Classify a single migration statement.
///
/// Returns [`StatementTarget::Unclassified`] for anything that is not a
/// `CREATE TABLE` / `CREATE [UNIQUE] INDEX` / `CREATE VIEW` /
/// `CREATE TYPE ... AS ENUM`. Being conservative here is the point: repair only
/// ever skips statements it can prove.
#[must_use]
pub fn classify_statement(dialect: Dialect, sql: &str) -> StatementTarget {
    let tokens = tokenize(sql);
    let mut index = 0;

    if !tokens.first().is_some_and(|t| t.is_word("create")) {
        return StatementTarget::Unclassified;
    }
    index += 1;

    let mut unique = false;
    loop {
        match tokens.get(index) {
            Some(t) if t.is_word("or") => {
                // OR REPLACE
                index += 2;
            }
            Some(t) if t.is_word("unique") => {
                unique = true;
                index += 1;
            }
            Some(t)
                if t.is_word("temp")
                    || t.is_word("temporary")
                    || t.is_word("unlogged")
                    || t.is_word("global")
                    || t.is_word("local") =>
            {
                index += 1;
            }
            _ => break,
        }
    }

    let Some(kind) = tokens.get(index).and_then(Tok::ident).map(str::to_string) else {
        return StatementTarget::Unclassified;
    };
    index += 1;

    // CONCURRENTLY / IF NOT EXISTS may follow the object kind.
    let skip_noise = |tokens: &[Tok], mut index: usize| {
        loop {
            match tokens.get(index) {
                Some(t) if t.is_word("concurrently") => index += 1,
                Some(t)
                    if t.is_word("if")
                        && tokens.get(index + 1).is_some_and(|t| t.is_word("not"))
                        && tokens.get(index + 2).is_some_and(|t| t.is_word("exists")) =>
                {
                    index += 3;
                }
                _ => return index,
            }
        }
    };

    match kind.as_str() {
        "table" => {
            index = skip_noise(&tokens, index);
            let Some((schema, name, next)) = read_qualified_name(&tokens, index) else {
                return StatementTarget::Unclassified;
            };
            let Some(columns) = read_table_columns(&tokens, next) else {
                return StatementTarget::Unclassified;
            };
            StatementTarget::Table {
                schema,
                name,
                columns,
            }
        }
        "index" => {
            index = skip_noise(&tokens, index);
            // An anonymous `CREATE INDEX ON t (...)` has no name to match on.
            if tokens.get(index).is_some_and(|t| t.is_word("on")) {
                return StatementTarget::Unclassified;
            }
            let Some((schema, name, mut next)) = read_qualified_name(&tokens, index) else {
                return StatementTarget::Unclassified;
            };
            if !tokens.get(next).is_some_and(|t| t.is_word("on")) {
                return StatementTarget::Unclassified;
            }
            next += 1;
            let Some((_, _, after_table)) = read_qualified_name(&tokens, next) else {
                return StatementTarget::Unclassified;
            };
            // Skip `USING <method>` and anything else before the column list.
            let mut cursor = after_table;
            while cursor < tokens.len() && !tokens[cursor].is_punct('(') {
                cursor += 1;
            }
            let Some(columns) = read_paren_list_heads(&tokens, cursor, false) else {
                return StatementTarget::Unclassified;
            };
            StatementTarget::Index {
                schema,
                name,
                unique,
                columns,
            }
        }
        "view" | "materialized" => {
            let mut cursor = index;
            if kind == "materialized" {
                if !tokens.get(cursor).is_some_and(|t| t.is_word("view")) {
                    return StatementTarget::Unclassified;
                }
                cursor += 1;
            }
            cursor = skip_noise(&tokens, cursor);
            let Some((schema, name, _)) = read_qualified_name(&tokens, cursor) else {
                return StatementTarget::Unclassified;
            };
            StatementTarget::View { schema, name }
        }
        "type" if dialect == Dialect::PostgreSQL => {
            let Some((schema, name, mut next)) = read_qualified_name(&tokens, index) else {
                return StatementTarget::Unclassified;
            };
            if !tokens.get(next).is_some_and(|t| t.is_word("as")) {
                return StatementTarget::Unclassified;
            }
            next += 1;
            if !tokens.get(next).is_some_and(|t| t.is_word("enum")) {
                return StatementTarget::Unclassified;
            }
            next += 1;
            let Some(values) = read_enum_values(&tokens, next) else {
                return StatementTarget::Unclassified;
            };
            StatementTarget::Enum {
                schema,
                name,
                values,
            }
        }
        _ => StatementTarget::Unclassified,
    }
}

/// Read `name` or `schema.name` starting at `index`.
///
/// Returns the parts plus the index just past the name.
fn read_qualified_name(tokens: &[Tok], index: usize) -> Option<(Option<String>, String, usize)> {
    let first = tokens.get(index)?.ident()?.to_string();
    if tokens.get(index + 1).is_some_and(|t| t.is_punct('.')) {
        let second = tokens.get(index + 2)?.ident()?.to_string();
        return Some((Some(first), second, index + 3));
    }
    Some((None, first, index + 1))
}

/// Split the parenthesized list at `index` and return the leading token of each
/// entry. `skip_constraints` drops table-constraint entries.
fn read_paren_list_heads(
    tokens: &[Tok],
    index: usize,
    skip_constraints: bool,
) -> Option<Vec<String>> {
    if !tokens.get(index)?.is_punct('(') {
        return None;
    }

    let mut heads = Vec::new();
    let mut depth = 0usize;
    let mut entry_start = true;
    let mut cursor = index;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.is_punct('(') {
            depth += 1;
            if depth == 1 {
                entry_start = true;
                cursor += 1;
                continue;
            }
        } else if token.is_punct(')') {
            depth -= 1;
            if depth == 0 {
                return Some(heads);
            }
        } else if depth == 1 && token.is_punct(',') {
            entry_start = true;
            cursor += 1;
            continue;
        }

        if depth == 1 && entry_start {
            entry_start = false;
            if let Some(ident) = token.ident() {
                let is_constraint = skip_constraints
                    && matches!(token, Tok::Word(_))
                    && CONSTRAINT_KEYWORDS.contains(&ident);
                if !is_constraint {
                    heads.push(ident.to_string());
                }
            }
        }

        cursor += 1;
    }

    None
}

fn read_table_columns(tokens: &[Tok], index: usize) -> Option<Vec<String>> {
    read_paren_list_heads(tokens, index, true)
}

fn read_enum_values(tokens: &[Tok], index: usize) -> Option<Vec<String>> {
    if !tokens.get(index)?.is_punct('(') {
        return None;
    }
    let mut values = Vec::new();
    let mut cursor = index + 1;
    while cursor < tokens.len() {
        match &tokens[cursor] {
            Tok::Str(value) => values.push(value.clone()),
            Tok::Punct(')') => return Some(values),
            Tok::Punct(',') => {}
            _ => return None,
        }
        cursor += 1;
    }
    None
}

// =============================================================================
// Per-dialect introspection
// =============================================================================

/// `SQLite` introspection for repair.
pub mod sqlite {
    use super::{Catalog, CatalogObject, ObjectKind, StatementTarget, classify_statement};
    use drizzle_types::Dialect;

    /// Every user object plus its stored DDL text.
    ///
    /// Columns: `type`, `name`, `sql` (nullable — auto-indexes have none).
    pub const OBJECTS_QUERY: &str = "SELECT type, name, sql FROM sqlite_master \
         WHERE type IN ('table', 'index', 'view', 'trigger') AND name NOT LIKE 'sqlite_%'";

    /// Fold [`OBJECTS_QUERY`] rows into a [`Catalog`] snapshot.
    ///
    /// `SQLite` stores the original `CREATE` text, so each object carries both
    /// its DDL and the column list parsed back out of it — repair can then
    /// compare either the text or the structure.
    ///
    /// Unknown `type` values are skipped so future object kinds do not break
    /// the snapshot.
    #[must_use]
    pub fn catalog(rows: &[(String, String, Option<String>)]) -> Catalog {
        let mut catalog = Catalog::new();
        for (kind, name, sql) in rows {
            let kind = match kind.as_str() {
                "table" => ObjectKind::Table,
                "index" => ObjectKind::Index,
                "view" => ObjectKind::View,
                "trigger" => ObjectKind::Trigger,
                _ => continue,
            };

            let parsed = sql
                .as_deref()
                .map(|sql| classify_statement(Dialect::SQLite, sql));
            let members = match &parsed {
                Some(
                    StatementTarget::Table { columns, .. } | StatementTarget::Index { columns, .. },
                ) => columns.clone(),
                _ => Vec::new(),
            };
            let unique = matches!(parsed, Some(StatementTarget::Index { unique: true, .. }));

            catalog.push(CatalogObject {
                kind,
                schema: None,
                name: name.clone(),
                sql: sql.clone(),
                members,
                unique,
            });
        }
        catalog
    }
}

/// `PostgreSQL` introspection for repair.
///
/// # Why base catalogs instead of `information_schema` / `pg_indexes`
///
/// Those views call reconstruction and privilege functions
/// (`pg_get_indexdef`, `has_table_privilege`) row by row. If another session
/// drops an object mid-scan the function raises
/// `could not open relation with OID …` and the whole scan fails rather than
/// just omitting the row. Repair runs against a database in an unknown state,
/// often alongside other work, so every query here reads base catalogs only,
/// and casts `name` columns to `text` so no domain type reaches the decoder.
///
/// Every query filters user schemas with
/// `n.nspname NOT IN ('pg_catalog', 'information_schema') AND n.nspname NOT
/// LIKE 'pg\_%'` (the second clause drops `pg_toast` / `pg_temp_N` /
/// `pg_toast_temp_N`).
pub mod postgres {
    use super::{Catalog, CatalogObject, ObjectKind};

    /// Base and partitioned tables. Columns: `schema`, `name`.
    pub const TABLES_QUERY: &str = "SELECT n.nspname::text, c.relname::text \
         FROM pg_catalog.pg_class c \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relkind IN ('r', 'p') \
           AND n.nspname NOT IN ('pg_catalog', 'information_schema') \
           AND n.nspname NOT LIKE 'pg\\_%'";

    /// Live table columns in attribute order.
    /// Columns: `schema`, `table`, `column`.
    pub const COLUMNS_QUERY: &str = "SELECT n.nspname::text, c.relname::text, a.attname::text \
         FROM pg_catalog.pg_attribute a \
         JOIN pg_catalog.pg_class c ON c.oid = a.attrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relkind IN ('r', 'p') AND a.attnum > 0 AND NOT a.attisdropped \
           AND n.nspname NOT IN ('pg_catalog', 'information_schema') \
           AND n.nspname NOT LIKE 'pg\\_%' \
         ORDER BY n.nspname, c.relname, a.attnum";

    /// Views and materialized views. Columns: `schema`, `name`.
    pub const VIEWS_QUERY: &str = "SELECT n.nspname::text, c.relname::text \
         FROM pg_catalog.pg_class c \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relkind IN ('v', 'm') \
           AND n.nspname NOT IN ('pg_catalog', 'information_schema') \
           AND n.nspname NOT LIKE 'pg\\_%'";

    /// Indexes. Columns: `schema`, `name`, `is_unique` (bool).
    pub const INDEXES_QUERY: &str = "SELECT n.nspname::text, c.relname::text, i.indisunique \
         FROM pg_catalog.pg_index i \
         JOIN pg_catalog.pg_class c ON c.oid = i.indexrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema') \
           AND n.nspname NOT LIKE 'pg\\_%'";

    /// Indexed column names in index order.
    /// Columns: `schema`, `index`, `column`.
    ///
    /// Expression elements have no `pg_attribute` row and are therefore
    /// omitted, which makes an expression index look shorter than the
    /// statement that creates it — repair then refuses to skip it instead of
    /// wrongly proving it. That is the safe direction.
    pub const INDEX_COLUMNS_QUERY: &str = "SELECT n.nspname::text, c.relname::text, a.attname::text \
         FROM pg_catalog.pg_index i \
         JOIN pg_catalog.pg_class c ON c.oid = i.indexrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord) ON true \
         JOIN pg_catalog.pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.attnum \
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema') \
           AND n.nspname NOT LIKE 'pg\\_%' \
         ORDER BY n.nspname, c.relname, k.ord";

    /// Enum types with labels in sort order.
    /// Columns: `schema`, `type`, `label`.
    pub const ENUMS_QUERY: &str = "SELECT n.nspname::text, t.typname::text, e.enumlabel::text \
         FROM pg_catalog.pg_type t \
         JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace \
         JOIN pg_catalog.pg_enum e ON e.enumtypid = t.oid \
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema') \
           AND n.nspname NOT LIKE 'pg\\_%' \
         ORDER BY n.nspname, t.typname, e.enumsortorder";

    /// Fold catalog rows into a [`Catalog`] snapshot.
    ///
    /// Each argument holds the rows of one query above, in column order:
    ///
    /// | argument | query |
    /// |---|---|
    /// | `tables` | [`TABLES_QUERY`] |
    /// | `columns` | [`COLUMNS_QUERY`] |
    /// | `views` | [`VIEWS_QUERY`] |
    /// | `indexes` | [`INDEXES_QUERY`] |
    /// | `index_columns` | [`INDEX_COLUMNS_QUERY`] |
    /// | `enums` | [`ENUMS_QUERY`] |
    ///
    /// Member rows must keep the order their query's `ORDER BY` produced.
    #[must_use]
    pub fn catalog(
        tables: &[(String, String)],
        columns: &[(String, String, String)],
        views: &[(String, String)],
        indexes: &[(String, String, bool)],
        index_columns: &[(String, String, String)],
        enums: &[(String, String, String)],
    ) -> Catalog {
        /// Member names belonging to `(schema, owner)`, in row order.
        fn members_of(rows: &[(String, String, String)], schema: &str, owner: &str) -> Vec<String> {
            rows.iter()
                .filter(|(row_schema, row_owner, _)| row_schema == schema && row_owner == owner)
                .map(|(_, _, member)| member.clone())
                .collect()
        }

        let mut catalog = Catalog::new();

        for (schema, name) in tables {
            catalog.push(CatalogObject {
                kind: ObjectKind::Table,
                schema: Some(schema.clone()),
                name: name.clone(),
                sql: None,
                members: members_of(columns, schema, name),
                unique: false,
            });
        }

        for (schema, name) in views {
            catalog.push(CatalogObject {
                kind: ObjectKind::View,
                schema: Some(schema.clone()),
                name: name.clone(),
                sql: None,
                members: Vec::new(),
                unique: false,
            });
        }

        for (schema, name, unique) in indexes {
            catalog.push(CatalogObject {
                kind: ObjectKind::Index,
                schema: Some(schema.clone()),
                name: name.clone(),
                // PostgreSQL keeps no DDL text we can safely read mid-repair,
                // so index equivalence is decided by the column list alone.
                sql: None,
                members: members_of(index_columns, schema, name),
                unique: *unique,
            });
        }

        let mut enum_names: Vec<(String, String)> = Vec::new();
        for (schema, name, _) in enums {
            let key = (schema.clone(), name.clone());
            if !enum_names.contains(&key) {
                enum_names.push(key);
            }
        }
        for (schema, name) in enum_names {
            let members = members_of(enums, &schema, &name);
            catalog.push(CatalogObject {
                kind: ObjectKind::Enum,
                schema: Some(schema),
                name,
                sql: None,
                members,
                unique: false,
            });
        }

        catalog
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Catalog, Disposition, ObjectKind, StatementTarget, classify_statement, plan, postgres,
        sqlite,
    };
    use crate::migrator::Migration;
    use drizzle_types::Dialect;

    /// A catalog holding one SQLite table with the given stored DDL.
    fn sqlite_table(name: &str, sql: &str) -> Catalog {
        sqlite::catalog(&[("table".to_string(), name.to_string(), Some(sql.to_string()))])
    }

    fn migration(statements: &[&str]) -> Migration {
        Migration::with_hash(
            "20240101010101_test",
            "hash",
            1,
            statements.iter().map(|s| (*s).to_string()).collect(),
        )
    }

    #[test]
    fn classifies_create_table_columns_without_constraints() {
        let target = classify_statement(
            Dialect::SQLite,
            "CREATE TABLE `users` (\n\t`id` integer PRIMARY KEY NOT NULL,\n\t`email` text NOT NULL,\n\tFOREIGN KEY (`id`) REFERENCES `other`(`id`),\n\tCONSTRAINT `uq` UNIQUE(`email`)\n)",
        );
        assert_eq!(
            target,
            StatementTarget::Table {
                schema: None,
                name: "users".to_string(),
                columns: vec!["id".to_string(), "email".to_string()],
            }
        );
    }

    #[test]
    fn classifies_if_not_exists_and_schema_qualified_tables() {
        let target = classify_statement(
            Dialect::PostgreSQL,
            "CREATE TABLE IF NOT EXISTS \"app\".\"users\" (\"id\" serial PRIMARY KEY, \"name\" text)",
        );
        assert_eq!(
            target,
            StatementTarget::Table {
                schema: Some("app".to_string()),
                name: "users".to_string(),
                columns: vec!["id".to_string(), "name".to_string()],
            }
        );
    }

    #[test]
    fn classifies_unique_and_concurrent_indexes() {
        assert_eq!(
            classify_statement(
                Dialect::SQLite,
                "CREATE UNIQUE INDEX `users_email_idx` ON `users` (`email`)"
            ),
            StatementTarget::Index {
                schema: None,
                name: "users_email_idx".to_string(),
                unique: true,
                columns: vec!["email".to_string()],
            }
        );

        assert_eq!(
            classify_statement(
                Dialect::PostgreSQL,
                "CREATE INDEX CONCURRENTLY IF NOT EXISTS \"users_email_idx\" ON \"users\" USING btree (\"email\", \"name\")"
            ),
            StatementTarget::Index {
                schema: None,
                name: "users_email_idx".to_string(),
                unique: false,
                columns: vec!["email".to_string(), "name".to_string()],
            }
        );
    }

    #[test]
    fn classifies_postgres_enum_types() {
        assert_eq!(
            classify_statement(
                Dialect::PostgreSQL,
                "CREATE TYPE \"public\".\"status\" AS ENUM('active', 'archived')"
            ),
            StatementTarget::Enum {
                schema: Some("public".to_string()),
                name: "status".to_string(),
                values: vec!["active".to_string(), "archived".to_string()],
            }
        );
        // SQLite has no CREATE TYPE — never classify it there.
        assert_eq!(
            classify_statement(Dialect::SQLite, "CREATE TYPE status AS ENUM('a')"),
            StatementTarget::Unclassified
        );
    }

    #[test]
    fn leaves_non_create_statements_unclassified() {
        for sql in [
            "ALTER TABLE `users` ADD COLUMN `age` integer",
            "DROP TABLE `users`",
            "INSERT INTO `users` (`id`) VALUES (1)",
            "UPDATE `users` SET `id` = 2",
            "CREATE TRIGGER t AFTER INSERT ON users BEGIN SELECT 1; END",
            "PRAGMA foreign_keys=OFF",
        ] {
            assert_eq!(
                classify_statement(Dialect::SQLite, sql),
                StatementTarget::Unclassified,
                "unexpectedly classified: {sql}"
            );
        }
    }

    #[test]
    fn skips_the_applied_prefix_and_executes_the_rest() {
        let migration = migration(&[
            "CREATE TABLE `a` (`id` integer PRIMARY KEY)",
            "CREATE TABLE `b` (`id` integer PRIMARY KEY)",
        ]);

        let catalog = sqlite_table("a", "CREATE TABLE `a` (`id` integer PRIMARY KEY)");

        let plan = plan(Dialect::SQLite, &migration, &catalog);
        assert_eq!(plan.skipped_count(), 1);
        assert!(plan.is_resolvable());
        assert!(matches!(
            plan.steps[0].disposition,
            Disposition::Skip { .. }
        ));
        assert_eq!(plan.steps[1].disposition, Disposition::Execute);

        let executable = plan
            .into_executable("\"__drizzle_migrations\"")
            .expect("resolvable");
        assert_eq!(
            executable,
            vec!["CREATE TABLE `b` (`id` integer PRIMARY KEY)"]
        );
    }

    #[test]
    fn matches_tables_structurally_when_formatting_differs() {
        let migration =
            migration(&["CREATE TABLE `a` (\n  `id` integer PRIMARY KEY,\n  `name` text\n)"]);
        let catalog = sqlite_table(
            "a",
            "CREATE TABLE \"a\" (\"id\" INTEGER PRIMARY KEY, \"name\" TEXT)",
        );

        let plan = plan(Dialect::SQLite, &migration, &catalog);
        assert!(plan.is_resolvable());
        assert_eq!(plan.skipped_count(), 1);
    }

    #[test]
    fn refuses_when_an_existing_table_does_not_match() {
        let migration = migration(&["CREATE TABLE `a` (`id` integer, `email` text)"]);
        let catalog = sqlite_table("a", "CREATE TABLE `a` (`id` integer)");

        let plan = plan(Dialect::SQLite, &migration, &catalog);
        assert!(!plan.is_resolvable());
        let error = plan
            .into_executable("\"__drizzle_migrations\"")
            .expect_err("must refuse");
        assert!(error.to_string().contains("do not match"));
        assert!(error.to_string().contains("UPDATE"));
    }

    #[test]
    fn mysql_manual_recovery_uses_backticked_tracking_columns() {
        let migration = migration(&["ALTER TABLE `a` ADD COLUMN `age` integer"]);
        let error = plan(Dialect::MySQL, &migration, &Catalog::new())
            .into_executable("`__drizzle_migrations`")
            .expect_err("must refuse")
            .to_string();
        assert!(
            error.contains("SET `applied_at` = CURRENT_TIMESTAMP WHERE `name`"),
            "{error}"
        );
        assert!(!error.contains("\"applied_at\""), "{error}");
    }

    #[test]
    fn refuses_unprovable_statements_inside_the_applied_prefix() {
        let migration = migration(&[
            "ALTER TABLE `a` ADD COLUMN `age` integer",
            "CREATE TABLE `b` (`id` integer)",
        ]);
        let catalog = Catalog::new();

        let plan = plan(Dialect::SQLite, &migration, &catalog);
        assert!(!plan.is_resolvable());
        assert!(matches!(
            plan.steps[0].disposition,
            Disposition::Unresolvable { .. }
        ));
    }

    #[test]
    fn executes_unprovable_statements_after_a_proven_gap() {
        let migration = migration(&[
            "CREATE TABLE `a` (`id` integer)",
            "CREATE TABLE `b` (`id` integer)",
            "ALTER TABLE `b` ADD COLUMN `age` integer",
        ]);
        let catalog = sqlite_table("a", "CREATE TABLE `a` (`id` integer)");

        let plan = plan(Dialect::SQLite, &migration, &catalog);
        assert!(plan.is_resolvable(), "{:?}", plan.steps);
        let executable = plan.into_executable("\"t\"").expect("resolvable");
        assert_eq!(executable.len(), 2);
    }

    #[test]
    fn postgres_catalog_round_trips_through_the_planner() {
        let catalog = postgres::catalog(
            &[("public".to_string(), "users".to_string())],
            &[
                ("public".to_string(), "users".to_string(), "id".to_string()),
                (
                    "public".to_string(),
                    "users".to_string(),
                    "email".to_string(),
                ),
            ],
            &[],
            &[("public".to_string(), "users_email_idx".to_string(), true)],
            &[(
                "public".to_string(),
                "users_email_idx".to_string(),
                "email".to_string(),
            )],
            &[(
                "public".to_string(),
                "status".to_string(),
                "active".to_string(),
            )],
        );

        assert!(
            catalog.find(ObjectKind::Table, None, "users").is_some(),
            "unqualified lookup resolves to public"
        );

        let migration = migration(&[
            "CREATE TABLE \"users\" (\"id\" serial PRIMARY KEY, \"email\" text)",
            "CREATE UNIQUE INDEX CONCURRENTLY \"users_email_idx\" ON \"users\" (\"email\")",
            "CREATE TYPE \"public\".\"status\" AS ENUM('active')",
            "CREATE TABLE \"posts\" (\"id\" serial PRIMARY KEY)",
        ]);

        let plan = plan(Dialect::PostgreSQL, &migration, &catalog);
        assert!(plan.is_resolvable(), "{:?}", plan.steps);
        assert_eq!(plan.skipped_count(), 3);
        let executable = plan
            .into_executable("\"drizzle\".\"t\"")
            .expect("resolvable");
        assert_eq!(executable.len(), 1);
        assert!(executable[0].contains("posts"));
    }

    #[test]
    fn postgres_enum_label_mismatch_is_unresolvable() {
        let catalog = postgres::catalog(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[(
                "public".to_string(),
                "status".to_string(),
                "active".to_string(),
            )],
        );
        let migration = migration(&["CREATE TYPE \"status\" AS ENUM('active', 'archived')"]);

        let plan = plan(Dialect::PostgreSQL, &migration, &catalog);
        assert!(!plan.is_resolvable());
    }

    #[test]
    fn postgres_index_uniqueness_mismatch_is_unresolvable() {
        // PostgreSQL keeps no readable DDL text for indexes, so uniqueness has
        // to come from the catalog flag rather than a text comparison.
        let catalog = postgres::catalog(
            &[],
            &[],
            &[],
            &[("public".to_string(), "users_email_idx".to_string(), false)],
            &[(
                "public".to_string(),
                "users_email_idx".to_string(),
                "email".to_string(),
            )],
            &[],
        );
        let migration =
            migration(&["CREATE UNIQUE INDEX \"users_email_idx\" ON \"users\" (\"email\")"]);

        let plan = plan(Dialect::PostgreSQL, &migration, &catalog);
        assert!(!plan.is_resolvable(), "{:?}", plan.steps);
        let text = plan
            .into_executable("\"t\"")
            .expect_err("must refuse")
            .to_string();
        assert!(text.contains("uniqueness differs"), "{text}");
    }
}
