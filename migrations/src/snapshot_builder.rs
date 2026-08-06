//! Schema snapshot builder from parsed schema files.
//!
//! This module converts [`ParseResult`] from the schema parser into
//! [`Snapshot`] values used for migration diffing.
//!
//! It is shared by runtime/build-time migration generation flows that do not
//! rely on the CLI.
//!
//! # Producer parity
//!
//! The entities built here mirror what the derive macros' runtime
//! `Schema::to_snapshot()` produces for the same source (see
//! `procmacros/src/{sqlite,postgres}/schema.rs` and `table/traits.rs`): the
//! same SQL type spellings, the same default rendering, and the same
//! constraint-name derivations (`{table}_pk` / `fk_..._fk` for `SQLite`,
//! `{table}_pkey` / `{table}_{col}_fkey` / `{table}_{col}_key` for
//! `PostgreSQL`). Foreign-key targets resolve through the referenced table's
//! metadata (explicit `name = "..."` renames included), primary keys are ONE
//! entity per table, `PostgreSQL` identity sequence options / index
//! `method`-`where`-`concurrently` / `NULLS NOT DISTINCT` are preserved, and
//! enum types carry their declared `#[postgres_enum(schema = "...")]` schema
//! — all matching the runtime producer. Remaining deliberate divergence:
//!
//! * Custom (non-enum) column types resolve `type_schema` to `public`; the
//!   runtime reads `DrizzlePostgresColumn::SCHEMA`, whose default is
//!   `public` but which a hand-written trait impl could override — the
//!   parser cannot evaluate user trait impls.

use crate::parser::{
    ColumnSpec, ParseResult, ParsedDefault, ParsedField, ParsedIndex, ParsedTable,
};
use crate::postgres::PostgresSnapshot;
use crate::schema::Snapshot;
use crate::sqlite::SQLiteSnapshot;
use drizzle_types::{Casing, Dialect};
use heck::ToSnakeCase;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

impl Snapshot {
    /// Build a snapshot from a parsed schema file, for migration diffing.
    ///
    /// Uses the provided `dialect` from config rather than the parser-detected
    /// dialect, allowing users to have multi-dialect schema files and select
    /// which to use via config.
    ///
    /// `casing` is accepted for API compatibility but is ignored for name
    /// derivation: the table macros unconditionally snake-case inferred names
    /// (`procmacros/src/common/table_pipeline.rs`), so honoring a camelCase
    /// casing here would produce names the runtime never uses. The casing
    /// config still affects introspection codegen (field naming), which is a
    /// separate flow.
    ///
    /// # Panics
    ///
    /// Panics for [`Dialect::MySQL`], which has no snapshot representation yet
    /// (same contract as [`Snapshot::empty`]). All upstream flows guard the
    /// dialect before calling (`build::run`, CLI dialect validation).
    #[must_use]
    pub fn from_parse_result(
        result: &ParseResult,
        dialect: Dialect,
        casing: Option<Casing>,
    ) -> Self {
        let _ = casing;
        match dialect {
            Dialect::SQLite => Self::Sqlite(build_sqlite_snapshot(result)),
            Dialect::PostgreSQL => Self::Postgres(build_postgres_snapshot(result)),
            Dialect::MySQL => {
                panic!("MySQL snapshot generation is not supported yet")
            }
        }
    }
}

// =============================================================================
// Shared name resolution
// =============================================================================

fn resolve_table_name(table: &ParsedTable) -> String {
    table
        .spec
        .explicit_name
        .clone()
        .unwrap_or_else(|| table.name.to_snake_case())
}

fn resolve_field_name(field: &ParsedField) -> String {
    field
        .spec
        .explicit_name
        .clone()
        .unwrap_or_else(|| field.name.to_snake_case())
}

/// Name maps built once per snapshot: struct ident -> resolved table name and
/// (struct ident, field ident) -> resolved column name.
struct NameMaps {
    tables: HashMap<String, String>,
    fields: HashMap<(String, String), String>,
}

impl NameMaps {
    fn build(tables: &[&ParsedTable]) -> Self {
        let mut table_map = HashMap::new();
        let mut field_map = HashMap::new();
        for table in tables {
            table_map.insert(table.name.clone(), resolve_table_name(table));
            for field in &table.fields {
                field_map.insert(
                    (table.name.clone(), field.name.clone()),
                    resolve_field_name(field),
                );
            }
        }
        Self {
            tables: table_map,
            fields: field_map,
        }
    }

    fn table(&self, struct_name: &str) -> String {
        self.tables
            .get(struct_name)
            .cloned()
            .unwrap_or_else(|| struct_name.to_snake_case())
    }

    fn field(&self, struct_name: &str, field_name: &str) -> String {
        self.fields
            .get(&(struct_name.to_string(), field_name.to_string()))
            .cloned()
            .unwrap_or_else(|| field_name.to_snake_case())
    }
}

/// Member-type filter when a schema struct of the matching dialect exists.
///
/// Mirrors runtime semantics: only schema members end up in
/// `Schema::to_snapshot()`. Applied to tables (and transitively to the
/// indexes/policies that target them); views and enums always pass through
/// because the introspection codegen intentionally omits them from generated
/// schema structs.
fn schema_members(result: &ParseResult, dialect: Dialect) -> Option<HashSet<&str>> {
    result
        .schema
        .as_ref()
        .filter(|schema| schema.dialect == dialect)
        .map(|schema| schema.member_types.iter().map(String::as_str).collect())
}

fn tables_for(result: &ParseResult, dialect: Dialect) -> Vec<&ParsedTable> {
    let members = schema_members(result, dialect);
    let mut tables: Vec<&ParsedTable> = result
        .tables
        .values()
        .filter(|t| t.dialect == dialect)
        .filter(|t| {
            members
                .as_ref()
                .is_none_or(|members| members.contains(t.name.as_str()))
        })
        .collect();
    tables.sort_by_key(|t| t.order);
    tables
}

fn indexes_for<'a>(
    result: &'a ParseResult,
    dialect: Dialect,
    kept_tables: &HashSet<&str>,
) -> Vec<&'a ParsedIndex> {
    let mut indexes: Vec<&ParsedIndex> = result
        .indexes
        .values()
        .filter(|i| i.dialect == dialect)
        .filter(|i| {
            i.table_name()
                .is_none_or(|table| kept_tables.contains(table))
        })
        .collect();
    indexes.sort_by_key(|i| i.order);
    indexes
}

// =============================================================================
// SQLite
// =============================================================================

/// `sqlite::field::parenthesized_sql_expression`.
fn parenthesized_sql_expression(expression: &str) -> String {
    let trimmed = expression.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed.to_string()
    } else {
        format!("({trimmed})")
    }
}

/// `sqlite::field::normalize_default_sql` — bare `CURRENT_*` keywords stay
/// verbatim, everything else gets parenthesized.
fn normalize_sqlite_default_sql(expression: &str) -> String {
    let trimmed = expression.trim();
    match trimmed.to_ascii_uppercase().as_str() {
        "CURRENT_TIME" | "CURRENT_DATE" | "CURRENT_TIMESTAMP" => trimmed.to_string(),
        _ => parenthesized_sql_expression(trimmed),
    }
}

/// SQL default string for a `SQLite` column, matching
/// `sqlite::table::traits::sqlite_default_sql` (string defaults become
/// SQL-quoted `'...'` with `''` doubling; booleans become `1`/`0`).
fn sqlite_default(spec: &ColumnSpec) -> Option<String> {
    if spec.generated.is_some() {
        // The macro rejects generated + default combinations at compile time.
        return None;
    }
    if let Some(default_sql) = &spec.default_sql {
        return Some(normalize_sqlite_default_sql(default_sql));
    }
    match spec.default.as_ref()? {
        ParsedDefault::Int(token) | ParsedDefault::Float(token) => Some(token.clone()),
        ParsedDefault::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
        ParsedDefault::Str(s) => Some(format!("'{}'", s.replace('\'', "''"))),
        // Non-literal defaults are dropped by the macros.
        ParsedDefault::Unsupported(_) => None,
    }
}

/// Build an `SQLite` snapshot from parsed schema
#[allow(clippy::too_many_lines)]
fn build_sqlite_snapshot(result: &ParseResult) -> SQLiteSnapshot {
    use crate::sqlite::{
        CheckConstraint, ForeignKey, Generated, GeneratedType, Index, IndexColumn, IndexOrigin,
        PrimaryKey, SqliteEntity, Table, UniqueConstraint, View,
    };

    let mut snapshot = SQLiteSnapshot::new();

    let tables = tables_for(result, Dialect::SQLite);
    let kept: HashSet<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    let maps = NameMaps::build(&tables);

    for table in &tables {
        let table_name = maps.table(&table.name);

        let mut sqlite_table = Table::new(table_name.clone());
        sqlite_table.strict = table.spec.strict;
        sqlite_table.without_rowid = table.spec.without_rowid;
        snapshot.add_entity(SqliteEntity::Table(sqlite_table));

        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col_name = maps.field(&table.name, &field.name);
            let spec = &field.spec;

            let mut col = crate::sqlite::Column::new(
                table_name.clone(),
                col_name.clone(),
                spec.sqlite_type.clone().unwrap_or_default(),
            );
            if !spec.nullable {
                col = col.not_null();
            }
            if spec.autoincrement {
                col = col.autoincrement();
            }
            if let Some(default) = sqlite_default(spec) {
                col = col.default_value(default);
            }
            if let Some(generated) = &spec.generated {
                col.generated = Some(Generated {
                    expression: Cow::Owned(parenthesized_sql_expression(&generated.expression)),
                    gen_type: if generated.stored {
                        GeneratedType::Stored
                    } else {
                        GeneratedType::Virtual
                    },
                });
            }
            if let Some(collate) = &spec.collate {
                col.collate = Some(Cow::Owned(collate.clone()));
            }
            snapshot.add_entity(SqliteEntity::Column(col));

            if spec.primary {
                pk_columns.push(col_name.clone());
            }

            // Column-level UNIQUE (macro: only when not also primary).
            if spec.unique && !spec.primary {
                snapshot.add_entity(SqliteEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        table_name.clone(),
                        format!("{table_name}_{col_name}_unique"),
                        vec![col_name.clone()],
                    ),
                ));
            }

            // Column-level CHECK, named like the macro's constraint refs.
            if let Some(check) = &spec.check {
                snapshot.add_entity(SqliteEntity::CheckConstraint(CheckConstraint::new(
                    table_name.clone(),
                    format!("{table_name}_{col_name}_check"),
                    check.clone(),
                )));
            }

            // Column-level foreign key. Name, target table, and target
            // column all resolve through the referenced table's actual
            // names (the macro resolves them via the target's `NAME`
            // consts, honoring explicit `name = "..."` renames).
            if let Some(reference) = &spec.references {
                let target_table = maps.table(&reference.table);
                let target_column = maps.field(&reference.table, &reference.column);
                let mut fk = ForeignKey::from_strings(
                    table_name.clone(),
                    format!("fk_{table_name}_{col_name}_{target_table}_{target_column}_fk"),
                    vec![col_name.clone()],
                    target_table,
                    vec![target_column],
                );
                fk.on_delete = spec.on_delete.clone().map(Cow::Owned);
                fk.on_update = spec.on_update.clone().map(Cow::Owned);
                snapshot.add_entity(SqliteEntity::ForeignKey(fk));
            }
        }

        // ONE PrimaryKey entity, canonical `{table}_pk` name.
        if !pk_columns.is_empty() {
            snapshot.add_entity(SqliteEntity::PrimaryKey(PrimaryKey::from_strings(
                table_name.clone(),
                format!("{table_name}_pk"),
                pk_columns,
            )));
        }

        // Composite FOREIGN_KEY(...) attributes. Source and target columns
        // resolve through the respective tables' field metadata (matching
        // the macro, which honors explicit `name = "..."` renames on both
        // sides); actions pass through verbatim.
        for cfk in &table.spec.composite_fks {
            let source_columns: Vec<String> = cfk
                .source_columns
                .iter()
                .map(|field| maps.field(&table.name, field))
                .collect();
            let target_table = maps.table(&cfk.target_table);
            let target_columns: Vec<String> = cfk
                .target_columns
                .iter()
                .map(|c| maps.field(&cfk.target_table, c))
                .collect();
            let fk_name = format!(
                "fk_{table_name}_{}_{}_{}_fk",
                source_columns.join("_"),
                target_table,
                target_columns.join("_")
            );
            let mut fk = ForeignKey::from_strings(
                table_name.clone(),
                fk_name,
                source_columns,
                target_table,
                target_columns,
            );
            fk.on_delete = cfk.on_delete.clone().map(Cow::Owned);
            fk.on_update = cfk.on_update.clone().map(Cow::Owned);
            snapshot.add_entity(SqliteEntity::ForeignKey(fk));
        }

        // Table-level UNIQUE(...) attributes (columns resolve through field
        // names, like the macro's constraint refs).
        for unique in &table.spec.unique_constraints {
            let columns: Vec<String> = unique
                .columns
                .iter()
                .map(|field| maps.field(&table.name, field))
                .collect();
            let name = unique
                .name
                .clone()
                .unwrap_or_else(|| format!("{table_name}_{}_unique", columns.join("_")));
            let mut constraint = UniqueConstraint::from_strings(table_name.clone(), name, columns);
            constraint.name_explicit = unique.name.is_some();
            snapshot.add_entity(SqliteEntity::UniqueConstraint(constraint));
        }

        // Table-level CHECK(...) attributes with the macro's naming: a single
        // check is `{table}_check`, several are `{table}_check{N}`.
        let check_count = table.spec.check_constraints.len();
        for (idx, check) in table.spec.check_constraints.iter().enumerate() {
            let name = check.name.clone().unwrap_or_else(|| {
                if check_count == 1 {
                    format!("{table_name}_check")
                } else {
                    format!("{table_name}_check{}", idx + 1)
                }
            });
            snapshot.add_entity(SqliteEntity::CheckConstraint(CheckConstraint::new(
                table_name.clone(),
                name,
                check.expr.clone(),
            )));
        }
    }

    // Indexes targeting kept tables. Index names derive from the struct
    // ident with the macro's fold; an explicit `name = "..."` (parser
    // extension) wins.
    for index in indexes_for(result, Dialect::SQLite, &kept) {
        let table_struct = index.table_name().unwrap_or_default();
        let index_name = index
            .spec
            .explicit_name
            .clone()
            .unwrap_or_else(|| crate::parser::sqlite_index_name(&index.name));
        let columns: Vec<IndexColumn> = index
            .spec
            .column_refs
            .iter()
            .map(|(table, field)| IndexColumn::new(maps.field(table, field)))
            .collect();

        snapshot.add_entity(SqliteEntity::Index(Index {
            table: maps.table(table_struct).into(),
            name: index_name.into(),
            columns,
            is_unique: index.spec.unique,
            where_clause: None,
            origin: IndexOrigin::Manual,
        }));
    }

    // Views (never scoped by schema membership; see `schema_members`).
    let mut views: Vec<_> = result
        .views
        .values()
        .filter(|v| v.dialect == Dialect::SQLite)
        .collect();
    views.sort_by_key(|v| v.order);
    for parsed in views {
        let name = parsed
            .explicit_name
            .clone()
            .unwrap_or_else(|| parsed.name.to_snake_case());
        let mut view = View::new(name);
        view.definition = parsed.definition.clone().map(Cow::Owned);
        view.is_existing = parsed.existing;
        snapshot.add_entity(SqliteEntity::View(view));
    }

    snapshot
}

// =============================================================================
// PostgreSQL
// =============================================================================

/// SQL default string for a `PostgreSQL` column, matching
/// `postgres::field::parse_column_attribute` / `default_to_string` (string
/// defaults become `'...'` with `''` doubling; booleans stay `true`/`false`;
/// `default_sql` passes through verbatim; serial/identity/generated columns
/// have no default).
fn postgres_default(spec: &ColumnSpec) -> Option<String> {
    if spec.serial.is_some() || spec.identity.is_some() || spec.generated.is_some() {
        return None;
    }
    if let Some(default_sql) = &spec.default_sql {
        return Some(default_sql.clone());
    }
    match spec.default.as_ref()? {
        ParsedDefault::Int(token) | ParsedDefault::Float(token) => Some(token.clone()),
        ParsedDefault::Bool(b) => Some(b.to_string()),
        ParsedDefault::Str(s) => Some(format!("'{}'", s.replace('\'', "''"))),
        ParsedDefault::Unsupported(_) => None,
    }
}

/// Build a `PostgreSQL` snapshot from parsed schema
#[allow(clippy::too_many_lines)]
fn build_postgres_snapshot(result: &ParseResult) -> PostgresSnapshot {
    use crate::postgres::ddl::{GeneratedType, IdentityType};
    use crate::postgres::{
        CheckConstraint, Column, Enum as PgEnum, ForeignKey, Generated, Identity, Index,
        IndexColumn, Policy, PostgresEntity, PrimaryKey, Schema as PgSchema, Table,
        UniqueConstraint, View,
    };

    let mut snapshot = PostgresSnapshot::new();

    let tables = tables_for(result, Dialect::PostgreSQL);
    let kept: HashSet<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    let maps = NameMaps::build(&tables);

    let mut table_schemas: HashMap<String, String> = HashMap::new();
    for table in &tables {
        let schema = table
            .spec
            .schema
            .clone()
            .unwrap_or_else(|| "public".to_string());
        table_schemas.insert(table.name.clone(), schema);
    }
    let schema_of = |struct_name: &str| -> String {
        table_schemas
            .get(struct_name)
            .cloned()
            .unwrap_or_else(|| "public".to_string())
    };

    // Enum ident -> declared schema (`#[postgres_enum(schema = "...")]`),
    // used to resolve `type_schema` on custom/enum columns the way the
    // macros do (`DrizzlePostgresColumn::SCHEMA`, default `public`).
    let enum_schemas: HashMap<&str, &str> = result
        .enums
        .values()
        .filter(|e| e.dialect == Dialect::PostgreSQL)
        .map(|e| (e.name.as_str(), e.schema.as_deref().unwrap_or("public")))
        .collect();
    let type_schema_of = |sql_type: &str| -> String {
        enum_schemas
            .get(sql_type)
            .map_or("public", |schema| *schema)
            .to_string()
    };

    // Schema entities appear when the first table, policy target, enum, or
    // view in that schema appears, mirroring the runtime producer — an enum
    // or view may be a schema's only occupant, and its CREATE needs the
    // CREATE SCHEMA first.
    let mut seen_schemas: HashSet<String> = HashSet::new();
    let mut schema_entities: Vec<String> = Vec::new();
    for table in &tables {
        let schema = schema_of(&table.name);
        if seen_schemas.insert(schema.clone()) {
            schema_entities.push(schema);
        }
    }
    let mut policies: Vec<_> = result
        .policies
        .values()
        .filter(|p| p.dialect == Dialect::PostgreSQL)
        .filter(|p| kept.contains(p.table.as_str()) || !table_schemas.contains_key(&p.table))
        .collect();
    policies.sort_by_key(|p| p.order);
    for policy in &policies {
        let schema = schema_of(&policy.table);
        if seen_schemas.insert(schema.clone()) {
            schema_entities.push(schema);
        }
    }
    let mut enum_decls: Vec<_> = result
        .enums
        .values()
        .filter(|e| e.dialect == Dialect::PostgreSQL)
        .collect();
    enum_decls.sort_by_key(|e| e.order);
    for parsed in enum_decls {
        let schema = parsed
            .schema
            .clone()
            .unwrap_or_else(|| "public".to_string());
        if seen_schemas.insert(schema.clone()) {
            schema_entities.push(schema);
        }
    }
    let mut view_decls: Vec<_> = result
        .views
        .values()
        .filter(|v| v.dialect == Dialect::PostgreSQL)
        .collect();
    view_decls.sort_by_key(|v| v.order);
    for parsed in view_decls {
        let schema = parsed
            .schema
            .clone()
            .unwrap_or_else(|| "public".to_string());
        if seen_schemas.insert(schema.clone()) {
            schema_entities.push(schema);
        }
    }
    for schema in schema_entities {
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new(schema)));
    }

    for table in &tables {
        let table_name = maps.table(&table.name);
        let schema_name = schema_of(&table.name);

        let mut pg_table = Table::new(schema_name.clone(), table_name.clone());
        // Mirror the builder interplay in the DDL type: TEMPORARY and
        // UNLOGGED are mutually exclusive; temporary wins when both are set.
        if table.spec.temporary {
            pg_table = pg_table.temporary();
        } else if table.spec.unlogged {
            pg_table = pg_table.unlogged();
        }
        if let Some(inherits) = &table.spec.inherits {
            pg_table = pg_table.inherits(inherits.clone());
        }
        if let Some(tablespace) = &table.spec.tablespace {
            pg_table = pg_table.tablespace(tablespace.clone());
        }
        if table.spec.rls {
            pg_table = pg_table.rls_enabled();
        }
        if let Some(comment) = &table.spec.comment {
            pg_table = pg_table.comment(comment.clone());
        }
        snapshot.add_entity(PostgresEntity::Table(pg_table));

        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col_name = maps.field(&table.name, &field.name);
            let spec = &field.spec;
            let sql_type = spec.pg_type.clone().unwrap_or_default();

            let identity = if spec.serial.is_none() {
                spec.identity.as_ref().map(|parsed| Identity {
                    name: Cow::Owned(format!("{table_name}_{col_name}_seq")),
                    schema: Some(Cow::Owned(schema_name.clone())),
                    type_: if parsed.always {
                        IdentityType::Always
                    } else {
                        IdentityType::ByDefault
                    },
                    increment: parsed.increment.clone().map(Cow::Owned),
                    min_value: parsed.min_value.clone().map(Cow::Owned),
                    max_value: parsed.max_value.clone().map(Cow::Owned),
                    start_with: parsed.start.clone().map(Cow::Owned),
                    cache: parsed.cache,
                    cycle: if parsed.cycle { Some(true) } else { None },
                })
            } else {
                None
            };

            let generated = spec.generated.as_ref().map(|parsed| Generated {
                expression: Cow::Owned(parsed.expression.clone()),
                gen_type: if parsed.stored {
                    GeneratedType::Stored
                } else {
                    GeneratedType::Virtual
                },
            });

            // Enum/custom types carry the TYPE's own schema: for enums the
            // declared `#[postgres_enum(schema = "...")]`, otherwise the
            // `DrizzlePostgresColumn::SCHEMA` default of `public` (the
            // parser cannot evaluate user trait impls, so custom non-enum
            // types resolve to `public` like the trait default).
            let type_schema = if matches!(
                crate::postgres::grammar::PgTypeCategory::from_sql_type(&sql_type),
                crate::postgres::grammar::PgTypeCategory::Custom
            ) {
                Some(Cow::Owned(type_schema_of(&sql_type)))
            } else {
                None
            };

            let column = Column {
                schema: schema_name.clone().into(),
                table: table_name.clone().into(),
                name: col_name.clone().into(),
                sql_type: sql_type.into(),
                type_schema,
                not_null: !spec.nullable,
                default: postgres_default(spec).map(Cow::Owned),
                generated,
                identity,
                dimensions: spec.pg_dimensions,
                comment: spec.comment.clone().map(Cow::Owned),
                collate: spec.collate.clone().map(Cow::Owned),
                ordinal_position: None,
            };
            snapshot.add_entity(PostgresEntity::Column(column));

            if spec.primary {
                pk_columns.push(col_name.clone());
            }

            if spec.unique && !spec.primary {
                snapshot.add_entity(PostgresEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        schema_name.clone(),
                        table_name.clone(),
                        format!("{table_name}_{col_name}_key"),
                        vec![col_name.clone()],
                    ),
                ));
            }

            if let Some(check) = &spec.check {
                snapshot.add_entity(PostgresEntity::CheckConstraint(CheckConstraint::new(
                    schema_name.clone(),
                    table_name.clone(),
                    format!("{table_name}_{col_name}_check"),
                    check.clone(),
                )));
            }

            // Column-level foreign key: `{table}_{col}_fkey` naming; target
            // schema/table/column resolve through the referenced table's
            // metadata (the macro resolves the target column via its `NAME`
            // const, honoring explicit `name = "..."` renames).
            if let Some(reference) = &spec.references {
                let mut fk = ForeignKey::from_strings(
                    schema_name.clone(),
                    table_name.clone(),
                    format!("{table_name}_{col_name}_fkey"),
                    vec![col_name.clone()],
                    schema_of(&reference.table),
                    maps.table(&reference.table),
                    vec![maps.field(&reference.table, &reference.column)],
                );
                if let Some(on_delete) = &spec.on_delete {
                    fk = fk.on_delete(on_delete.clone());
                }
                if let Some(on_update) = &spec.on_update {
                    fk = fk.on_update(on_update.clone());
                }
                if spec.deferrable {
                    fk = fk.deferrable();
                }
                if spec.initially_deferred {
                    fk = fk.initially_deferred();
                }
                snapshot.add_entity(PostgresEntity::ForeignKey(fk));
            }
        }

        // ONE PrimaryKey entity covering all PK columns. (The runtime
        // producer emits one per column for composite PKs; single entity is
        // the spec-correct shape and matches the macro's compile-time
        // metadata.)
        if !pk_columns.is_empty() {
            snapshot.add_entity(PostgresEntity::PrimaryKey(PrimaryKey::from_strings(
                schema_name.clone(),
                table_name.clone(),
                format!("{table_name}_pkey"),
                pk_columns,
            )));
        }

        // Composite FOREIGN_KEY(...) attributes: `{table}_{first_col}_fkey`
        // naming with source columns resolved through field names (Postgres
        // macro behavior); actions pass through verbatim.
        for cfk in &table.spec.composite_fks {
            let source_columns: Vec<String> = cfk
                .source_columns
                .iter()
                .map(|field| maps.field(&table.name, field))
                .collect();
            let fk_name = format!(
                "{table_name}_{}_fkey",
                source_columns.first().cloned().unwrap_or_default()
            );
            let mut fk = ForeignKey::from_strings(
                schema_name.clone(),
                table_name.clone(),
                fk_name,
                source_columns,
                schema_of(&cfk.target_table),
                maps.table(&cfk.target_table),
                cfk.target_columns
                    .iter()
                    .map(|c| maps.field(&cfk.target_table, c))
                    .collect(),
            );
            if let Some(on_delete) = &cfk.on_delete {
                fk = fk.on_delete(on_delete.clone());
            }
            if let Some(on_update) = &cfk.on_update {
                fk = fk.on_update(on_update.clone());
            }
            if cfk.deferrable {
                fk = fk.deferrable();
            }
            if cfk.initially_deferred {
                fk = fk.initially_deferred();
            }
            snapshot.add_entity(PostgresEntity::ForeignKey(fk));
        }

        // Table-level UNIQUE(...) attributes: `{table}_{cols}_key` naming.
        for unique in &table.spec.unique_constraints {
            let columns: Vec<String> = unique
                .columns
                .iter()
                .map(|field| maps.field(&table.name, field))
                .collect();
            let name = unique
                .name
                .clone()
                .unwrap_or_else(|| format!("{table_name}_{}_key", columns.join("_")));
            let mut constraint = UniqueConstraint::from_strings(
                schema_name.clone(),
                table_name.clone(),
                name,
                columns,
            );
            constraint.name_explicit = unique.name.is_some();
            constraint.nulls_not_distinct = unique.nulls_not_distinct;
            constraint.deferrable = unique.deferrable;
            constraint.initially_deferred = unique.initially_deferred;
            snapshot.add_entity(PostgresEntity::UniqueConstraint(constraint));
        }

        // Table-level CHECK(...) attributes, macro naming (see SQLite twin).
        let check_count = table.spec.check_constraints.len();
        for (idx, check) in table.spec.check_constraints.iter().enumerate() {
            let name = check.name.clone().unwrap_or_else(|| {
                if check_count == 1 {
                    format!("{table_name}_check")
                } else {
                    format!("{table_name}_check{}", idx + 1)
                }
            });
            snapshot.add_entity(PostgresEntity::CheckConstraint(CheckConstraint::new(
                schema_name.clone(),
                table_name.clone(),
                name,
                check.expr.clone(),
            )));
        }
    }

    // Indexes targeting kept tables.
    for index in indexes_for(result, Dialect::PostgreSQL, &kept) {
        let table_struct = index.table_name().unwrap_or_default();
        let index_name = index
            .spec
            .explicit_name
            .clone()
            .unwrap_or_else(|| crate::parser::postgres_index_name(&index.name));
        let columns: Vec<IndexColumn> = index
            .spec
            .column_refs
            .iter()
            .map(|(table, field)| IndexColumn::new(maps.field(table, field)))
            .collect();

        snapshot.add_entity(PostgresEntity::Index(Index {
            schema: schema_of(table_struct).into(),
            table: maps.table(table_struct).into(),
            name: index_name.into(),
            name_explicit: false,
            columns,
            is_unique: index.spec.unique,
            where_clause: index.spec.where_clause.clone().map(Cow::Owned),
            method: index.spec.method.clone().map(Cow::Owned),
            with: None,
            concurrently: index.spec.concurrent,
        }));
    }

    // Enums: created in the schema from `#[postgres_enum(schema = "...")]`
    // (default `public`), matching the derive's `ENUM_SCHEMA` const.
    let mut enums: Vec<_> = result
        .enums
        .values()
        .filter(|e| e.dialect == Dialect::PostgreSQL)
        .collect();
    enums.sort_by_key(|e| e.order);
    for parsed in enums {
        snapshot.add_entity(PostgresEntity::Enum(PgEnum::from_strings(
            parsed
                .schema
                .clone()
                .unwrap_or_else(|| "public".to_string()),
            parsed.name.clone(),
            parsed.variants.clone(),
        )));
    }

    // Views.
    let mut views: Vec<_> = result
        .views
        .values()
        .filter(|v| v.dialect == Dialect::PostgreSQL)
        .collect();
    views.sort_by_key(|v| v.order);
    for parsed in views {
        let schema = parsed
            .schema
            .clone()
            .unwrap_or_else(|| "public".to_string());
        let name = parsed
            .explicit_name
            .clone()
            .unwrap_or_else(|| parsed.name.to_snake_case());
        let mut view = View::new(schema, name);
        view.definition = parsed.definition.clone().map(Cow::Owned);
        view.materialized = parsed.materialized;
        view.is_existing = parsed.existing;
        view.with_no_data = if parsed.with_no_data {
            Some(true)
        } else {
            None
        };
        view.using = parsed.using.clone().map(Cow::Owned);
        view.tablespace = parsed.tablespace.clone().map(Cow::Owned);
        snapshot.add_entity(PostgresEntity::View(view));
    }

    // Policies (RLS) targeting kept tables.
    for parsed in policies {
        let schema = schema_of(&parsed.table);
        let table = maps.table(&parsed.table);
        let name = parsed
            .explicit_name
            .clone()
            .unwrap_or_else(|| heck::AsSnakeCase(parsed.name.as_str()).to_string());
        let mut policy = Policy::new(schema, table, name);
        policy.as_clause = parsed.as_clause.clone().map(Cow::Owned);
        policy.for_clause = parsed.for_clause.clone().map(Cow::Owned);
        if !parsed.to.is_empty() {
            policy.to = Some(parsed.to.iter().cloned().map(Cow::Owned).collect());
        }
        policy.using = parsed.using.clone().map(Cow::Owned);
        policy.with_check = parsed.with_check.clone().map(Cow::Owned);
        snapshot.add_entity(PostgresEntity::Policy(policy));
    }

    snapshot
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SchemaParser;

    fn sqlite_snapshot(code: &str) -> SQLiteSnapshot {
        let result = SchemaParser::parse(code);
        assert!(
            result.errors.is_empty(),
            "parse errors: {:?}",
            result.errors
        );
        match Snapshot::from_parse_result(&result, Dialect::SQLite, None) {
            Snapshot::Sqlite(s) => s,
            Snapshot::Postgres(_) => panic!("expected SQLite snapshot"),
        }
    }

    fn postgres_snapshot(code: &str) -> PostgresSnapshot {
        let result = SchemaParser::parse(code);
        assert!(
            result.errors.is_empty(),
            "parse errors: {:?}",
            result.errors
        );
        match Snapshot::from_parse_result(&result, Dialect::PostgreSQL, None) {
            Snapshot::Postgres(s) => s,
            Snapshot::Sqlite(_) => panic!("expected Postgres snapshot"),
        }
    }

    #[test]
    fn test_postgres_snapshot_preserves_column_markers() {
        use crate::postgres::ddl::{GeneratedType, IdentityType, PostgresEntity};

        let code = r#"
#[PostgresTable(schema = "app")]
pub struct PgMarkers {
    #[column(serial, primary)]
    pub id: i32,
    #[column(smallserial)]
    pub small_id: i16,
    #[column(bigserial)]
    pub big_id: i64,
    #[column(json)]
    pub json_doc: AppDoc,
    #[column(jsonb)]
    pub jsonb_doc: AppDoc,
    #[column(identity(by_default))]
    pub identity_id: i32,
    #[column(generated(stored, "first_name || ' ' || last_name"))]
    pub full_name: String,
    #[column(collate = "C")]
    pub sortable: String,
}
"#;

        let snap = postgres_snapshot(code);
        let column = |name: &str| {
            snap.ddl
                .iter()
                .find_map(|entity| {
                    if let PostgresEntity::Column(column) = entity
                        && column.name.as_ref() == name
                    {
                        Some(column)
                    } else {
                        None
                    }
                })
                .expect("expected column")
        };

        // SQL types use the macro's spelling (uppercase `to_sql_type`).
        assert_eq!(column("id").sql_type.as_ref(), "SERIAL");
        assert!(column("id").identity.is_none());
        assert!(column("id").default.is_none());
        assert_eq!(column("small_id").sql_type.as_ref(), "SMALLSERIAL");
        assert_eq!(column("big_id").sql_type.as_ref(), "BIGSERIAL");
        assert_eq!(column("json_doc").sql_type.as_ref(), "JSON");
        assert_eq!(column("jsonb_doc").sql_type.as_ref(), "JSONB");

        let identity = column("identity_id")
            .identity
            .as_ref()
            .expect("expected identity");
        assert_eq!(identity.type_, IdentityType::ByDefault);
        assert_eq!(identity.name.as_ref(), "pg_markers_identity_id_seq");
        assert_eq!(identity.schema.as_deref(), Some("app"));

        let generated = column("full_name")
            .generated
            .as_ref()
            .expect("expected generated column");
        assert_eq!(generated.gen_type, GeneratedType::Stored);
        assert_eq!(
            generated.expression.as_ref(),
            "first_name || ' ' || last_name"
        );
        assert!(column("full_name").identity.is_none());
        assert!(column("full_name").default.is_none());

        assert_eq!(column("sortable").collate.as_deref(), Some("C"));
    }

    #[test]
    fn test_sqlite_uuid_snapshot_storage_respects_column_type() {
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct UuidStorage {
    #[column(primary)]
    pub id: i64,
    pub blob_uuid: uuid::Uuid,
    #[column(text)]
    pub text_uuid: uuid::Uuid,
    #[blob]
    pub legacy_blob_uuid: uuid::Uuid,
    #[text]
    pub legacy_text_uuid: uuid::Uuid,
}
"#;

        let snap = sqlite_snapshot(code);
        let column_type = |name: &str| {
            snap.ddl
                .iter()
                .find_map(|entity| {
                    if let SqliteEntity::Column(column) = entity
                        && column.name.as_ref() == name
                    {
                        Some(column.sql_type.as_ref())
                    } else {
                        None
                    }
                })
                .expect("expected column")
        };

        assert_eq!(column_type("blob_uuid"), "BLOB");
        assert_eq!(column_type("text_uuid"), "TEXT");
        assert_eq!(column_type("legacy_blob_uuid"), "BLOB");
        assert_eq!(column_type("legacy_text_uuid"), "TEXT");
    }

    #[test]
    fn test_sqlite_string_defaults_are_sql_quoted() {
        // P4: string defaults become SQL-quoted with '' doubling, matching
        // the macro's rendering; P9: default_sql passes through with the
        // macro's normalization.
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct Defaults {
    #[column(default = "hello")]
    pub greeting: String,
    #[column(default = "it's")]
    pub quoted: String,
    #[column(default_sql = "CURRENT_TIMESTAMP")]
    pub created_at: String,
    #[column(default_sql = "strftime('%s','now')")]
    pub epoch: i64,
    #[column(default = true)]
    pub active: bool,
}
"#;

        let snap = sqlite_snapshot(code);
        let default_of = |name: &str| {
            snap.ddl
                .iter()
                .find_map(|entity| {
                    if let SqliteEntity::Column(column) = entity
                        && column.name.as_ref() == name
                    {
                        Some(column.default.as_deref().map(str::to_string))
                    } else {
                        None
                    }
                })
                .expect("expected column")
        };

        assert_eq!(default_of("greeting").as_deref(), Some("'hello'"));
        assert_eq!(default_of("quoted").as_deref(), Some("'it''s'"));
        assert_eq!(
            default_of("created_at").as_deref(),
            Some("CURRENT_TIMESTAMP")
        );
        assert_eq!(
            default_of("epoch").as_deref(),
            Some("(strftime('%s','now'))")
        );
        assert_eq!(default_of("active").as_deref(), Some("1"));
    }

    #[test]
    fn test_sqlite_fk_actions_are_normalized_and_names_canonical() {
        // P3: SET_NULL renders as `SET NULL`; P14: canonical `fk_..._fk` /
        // `{table}_pk` names.
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
    #[column(references = Users::id, on_delete = SET_NULL, on_update = Cascade)]
    pub author_id: i64,
}
"#;

        let snap = sqlite_snapshot(code);
        let fk = snap
            .ddl
            .iter()
            .find_map(|e| {
                if let SqliteEntity::ForeignKey(fk) = e {
                    Some(fk)
                } else {
                    None
                }
            })
            .expect("expected foreign key");
        assert_eq!(fk.name.as_ref(), "fk_posts_author_id_users_id_fk");
        assert_eq!(fk.on_delete.as_deref(), Some("SET NULL"));
        assert_eq!(fk.on_update.as_deref(), Some("CASCADE"));

        let pk_names: Vec<&str> = snap
            .ddl
            .iter()
            .filter_map(|e| {
                if let SqliteEntity::PrimaryKey(pk) = e {
                    Some(pk.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert!(pk_names.contains(&"users_pk"));
        assert!(pk_names.contains(&"posts_pk"));
    }

    #[test]
    fn test_sqlite_generated_collate_check_table_constraints() {
        // P10 + P8: generated/collate/check plus table-level
        // FOREIGN_KEY/UNIQUE/CHECK all survive into the snapshot.
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct Parent {
    #[column(primary)]
    pub id_a: i64,
    #[column(primary)]
    pub id_b: i64,
}

#[SQLiteTable(
    FOREIGN_KEY(columns(pa, pb), references(Parent, id_a, id_b), on_delete = "CASCADE"),
    UNIQUE(columns(first, last), name = "people_name_unique"),
    CHECK(expr = "score >= 0")
)]
pub struct People {
    #[column(primary)]
    pub id: i64,
    pub pa: i64,
    pub pb: i64,
    pub first: String,
    #[column(collate = NOCASE)]
    pub last: String,
    #[column(check = "score <= 100")]
    pub score: i64,
    #[column(generated(stored, "first || ' ' || last"))]
    pub full_name: String,
    #[column(generated(virtual, "score * 2"))]
    pub double_score: i64,
}
"#;

        let snap = sqlite_snapshot(code);

        // Composite PK keeps declaration order in one entity.
        let parent_pk = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                SqliteEntity::PrimaryKey(pk) if pk.table.as_ref() == "parent" => Some(pk),
                _ => None,
            })
            .expect("parent pk");
        assert_eq!(parent_pk.name.as_ref(), "parent_pk");
        assert_eq!(
            parent_pk
                .columns
                .iter()
                .map(std::convert::AsRef::as_ref)
                .collect::<Vec<_>>(),
            vec!["id_a", "id_b"]
        );

        let fk = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                SqliteEntity::ForeignKey(fk) => Some(fk),
                _ => None,
            })
            .expect("composite fk");
        assert_eq!(fk.name.as_ref(), "fk_people_pa_pb_parent_id_a_id_b_fk");
        assert_eq!(fk.on_delete.as_deref(), Some("CASCADE"));

        let unique = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                SqliteEntity::UniqueConstraint(u) => Some(u),
                _ => None,
            })
            .expect("table unique");
        assert_eq!(unique.name.as_ref(), "people_name_unique");
        assert!(unique.name_explicit);

        let checks: Vec<&crate::sqlite::CheckConstraint> = snap
            .ddl
            .iter()
            .filter_map(|e| match e {
                SqliteEntity::CheckConstraint(c) => Some(c),
                _ => None,
            })
            .collect();
        assert!(
            checks
                .iter()
                .any(|c| c.name.as_ref() == "people_score_check")
        );
        assert!(checks.iter().any(|c| c.name.as_ref() == "people_check"));

        let column = |name: &str| {
            snap.ddl
                .iter()
                .find_map(|e| match e {
                    SqliteEntity::Column(c) if c.name.as_ref() == name => Some(c),
                    _ => None,
                })
                .expect("column")
        };
        assert_eq!(column("last").collate.as_deref(), Some("NOCASE"));
        let full_name = column("full_name").generated.as_ref().expect("generated");
        assert_eq!(full_name.expression.as_ref(), "(first || ' ' || last)");
        assert_eq!(full_name.gen_type, crate::sqlite::GeneratedType::Stored);
        assert_eq!(
            column("double_score")
                .generated
                .as_ref()
                .expect("generated")
                .gen_type,
            crate::sqlite::GeneratedType::Virtual
        );
    }

    #[test]
    fn test_postgres_schema_and_index_options_are_preserved() {
        use crate::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable(schema = "auth")]
pub struct Users {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(schema = "app")]
pub struct Sessions {
    #[column(primary)]
    pub id: i32,
    #[column(references = Users::id)]
    pub user_id: i32,
}

#[PostgresIndex(concurrent, method = "gin", where = "user_id > 0")]
pub struct SessionsUserIdx(Sessions::user_id);
"#;

        let snap = postgres_snapshot(code);

        let has_auth_schema = snap
            .ddl
            .iter()
            .any(|e| matches!(e, PostgresEntity::Schema(s) if s.name.as_ref() == "auth"));
        let has_app_schema = snap
            .ddl
            .iter()
            .any(|e| matches!(e, PostgresEntity::Schema(s) if s.name.as_ref() == "app"));
        assert!(has_auth_schema, "missing auth schema entity");
        assert!(has_app_schema, "missing app schema entity");

        let fk = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::ForeignKey(fk) = e {
                Some(fk)
            } else {
                None
            }
        });
        let fk = fk.expect("expected foreign key");
        assert_eq!(fk.schema.as_ref(), "app");
        assert_eq!(fk.schema_to.as_ref(), "auth");
        assert_eq!(fk.name.as_ref(), "sessions_user_id_fkey");

        let idx = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let idx = idx.expect("expected index");
        assert!(idx.concurrently);
        assert_eq!(idx.method.as_deref(), Some("gin"));
        assert_eq!(idx.where_clause.as_deref(), Some("user_id > 0"));
        assert_eq!(idx.schema.as_ref(), "app");
        assert_eq!(idx.name.as_ref(), "sessions_user_idx");
    }

    #[test]
    fn test_postgres_deferrable_arrays_and_key_names() {
        // P13: deferrable FKs, Vec<T> arrays; P14: `{t}_pkey` / `{t}_{c}_key`.
        use crate::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable]
pub struct Users {
    #[column(primary)]
    pub id: i32,
    #[column(unique)]
    pub email: String,
    pub tags: Vec<String>,
}

#[PostgresTable]
pub struct Sessions {
    #[column(primary)]
    pub id: i32,
    #[column(references = Users::id, deferrable, initially_deferred, on_delete = SET_NULL)]
    pub user_id: i32,
}
"#;

        let snap = postgres_snapshot(code);

        let tags = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Column(c) if c.name.as_ref() == "tags" => Some(c),
                _ => None,
            })
            .expect("tags column");
        assert_eq!(tags.sql_type.as_ref(), "TEXT");
        assert_eq!(tags.dimensions, Some(1));

        let fk = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::ForeignKey(fk) => Some(fk),
                _ => None,
            })
            .expect("fk");
        assert!(fk.deferrable);
        assert!(fk.initially_deferred);
        assert_eq!(fk.on_delete.as_deref(), Some("SET NULL"));

        let unique = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::UniqueConstraint(u) => Some(u),
                _ => None,
            })
            .expect("unique");
        assert_eq!(unique.name.as_ref(), "users_email_key");

        let pk = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::PrimaryKey(pk) if pk.table.as_ref() == "users" => Some(pk),
                _ => None,
            })
            .expect("pk");
        assert_eq!(pk.name.as_ref(), "users_pkey");
    }

    #[test]
    fn test_postgres_enum_view_policy_entities() {
        // P8: enums, views, and policies materialize as snapshot entities.
        use crate::postgres::ddl::PostgresEntity;

        let code = r#"
#[derive(PostgresEnum, Default, Clone)]
pub enum OrderStatus {
    #[default]
    Pending,
    Shipped,
}

#[PostgresTable(RLS)]
pub struct Orders {
    #[column(primary)]
    pub id: i32,
    #[column(enum)]
    pub status: OrderStatus,
}

#[PostgresView(definition = "SELECT id FROM orders WHERE status = 'Pending'")]
pub struct PendingOrders {
    pub id: i32,
}

#[PostgresPolicy(AS = "PERMISSIVE", FOR = "SELECT", TO(authenticated), USING = "true")]
pub struct OrdersPolicy(Orders);
"#;

        let snap = postgres_snapshot(code);

        let pg_enum = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Enum(en) => Some(en),
                _ => None,
            })
            .expect("enum entity");
        assert_eq!(pg_enum.name.as_ref(), "OrderStatus");
        assert_eq!(pg_enum.schema.as_ref(), "public");
        assert_eq!(
            pg_enum
                .values
                .iter()
                .map(std::convert::AsRef::as_ref)
                .collect::<Vec<_>>(),
            vec!["Pending", "Shipped"]
        );

        let status = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Column(c) if c.name.as_ref() == "status" => Some(c),
                _ => None,
            })
            .expect("status column");
        assert_eq!(status.sql_type.as_ref(), "OrderStatus");
        assert_eq!(status.type_schema.as_deref(), Some("public"));

        let orders = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Table(t) => Some(t),
                _ => None,
            })
            .expect("orders table");
        assert_eq!(orders.is_rls_enabled, Some(true));

        let view = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::View(v) => Some(v),
                _ => None,
            })
            .expect("view entity");
        assert_eq!(view.name.as_ref(), "pending_orders");
        assert_eq!(
            view.definition.as_deref(),
            Some("SELECT id FROM orders WHERE status = 'Pending'")
        );

        let policy = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Policy(p) => Some(p),
                _ => None,
            })
            .expect("policy entity");
        assert_eq!(policy.name.as_ref(), "orders_policy");
        assert_eq!(policy.table.as_ref(), "orders");
        assert_eq!(policy.as_clause.as_deref(), Some("PERMISSIVE"));
        assert_eq!(policy.using.as_deref(), Some("true"));
    }

    #[test]
    fn test_sqlite_view_entity() {
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteView(definition = "SELECT id FROM users")]
pub struct AllUsers {
    pub id: i64,
}
"#;

        let snap = sqlite_snapshot(code);
        let view = snap
            .ddl
            .iter()
            .find_map(|e| match e {
                SqliteEntity::View(v) => Some(v),
                _ => None,
            })
            .expect("view entity");
        assert_eq!(view.name.as_ref(), "all_users");
        assert_eq!(view.definition.as_deref(), Some("SELECT id FROM users"));
        assert!(!view.is_existing);
    }

    #[test]
    fn test_casing_config_is_ignored_for_names() {
        // P15: the macros always snake-case inferred names; a camelCase
        // casing config must not change parser output.
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct UserAccounts {
    #[column(primary)]
    pub id: i64,
    pub displayName: String,
}
"#;

        let result = SchemaParser::parse(code);
        let snapshot =
            Snapshot::from_parse_result(&result, Dialect::SQLite, Some(Casing::CamelCase));
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            Snapshot::Postgres(_) => panic!("expected SQLite snapshot"),
        };

        assert!(
            snap.ddl
                .iter()
                .any(|e| matches!(e, SqliteEntity::Table(t) if t.name.as_ref() == "user_accounts"))
        );
        assert!(
            snap.ddl
                .iter()
                .any(|e| matches!(e, SqliteEntity::Column(c) if c.name.as_ref() == "display_name"))
        );
    }

    #[test]
    fn test_schema_struct_scopes_tables() {
        // Membership scoping matches runtime semantics: with a schema struct
        // present, only member tables (and their indexes) are emitted.
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteTable]
pub struct Orphan {
    #[column(primary)]
    pub id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
}
"#;

        let snap = sqlite_snapshot(code);
        let table_names: Vec<&str> = snap
            .ddl
            .iter()
            .filter_map(|e| match e {
                SqliteEntity::Table(t) => Some(t.name.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(table_names, vec!["users"]);
    }

    #[test]
    fn test_no_schema_struct_keeps_all_tables() {
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
}
"#;

        let snap = sqlite_snapshot(code);
        let table_count = snap
            .ddl
            .iter()
            .filter(|e| matches!(e, SqliteEntity::Table(_)))
            .count();
        assert_eq!(table_count, 2);
    }

    #[test]
    fn test_sqlite_table_options_and_pk_name_are_preserved() {
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(strict, without_rowid)]
pub struct Accounts {
    #[column(primary)]
    pub id: i64,
}
"#;

        let snap = sqlite_snapshot(code);
        let table = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected sqlite table");
        assert!(table.strict, "strict should be preserved");
        assert!(table.without_rowid, "without_rowid should be preserved");

        let pk = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::PrimaryKey(pk) = e {
                Some(pk)
            } else {
                None
            }
        });
        let pk = pk.expect("expected sqlite primary key");
        // Canonical macro/introspection name (`types::sqlite::ddl::name_for_pk`).
        assert_eq!(pk.name.as_ref(), "accounts_pk");
    }

    #[test]
    fn test_sqlite_casing_preserves_explicit_names() {
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(name = "users_tbl")]
pub struct UsersTable {
    #[column(name = "user_id", primary)]
    pub userId: i64,
    pub emailAddress: String,
}

#[SQLiteIndex(name = "users_tbl_email_idx")]
pub struct UsersEmailIdx(UsersTable::emailAddress);
"#;

        let result = SchemaParser::parse(code);
        let snapshot =
            Snapshot::from_parse_result(&result, Dialect::SQLite, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            Snapshot::Postgres(_) => panic!("Expected SQLite snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected sqlite table");
        assert_eq!(table.name.as_ref(), "users_tbl");

        let user_id = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Column(c) = e
                && c.name.as_ref() == "user_id"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(user_id.is_some(), "expected explicit column name user_id");

        let email_col = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Column(c) = e
                && c.name.as_ref() == "email_address"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            email_col.is_some(),
            "expected inferred snake_case column name"
        );

        let index = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let index = index.expect("expected sqlite index");
        assert_eq!(index.name.as_ref(), "users_tbl_email_idx");
        assert_eq!(index.columns[0].value.as_ref(), "email_address");
    }

    #[test]
    fn test_postgres_casing_preserves_explicit_names() {
        use crate::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable(schema = "auth", name = "users_tbl")]
pub struct UsersTable {
    #[column(name = "user_id", primary)]
    pub userId: i32,
    pub createdAt: String,
}

#[PostgresIndex(name = "users_tbl_created_idx")]
pub struct UsersCreatedIdx(UsersTable::createdAt);
"#;

        let result = SchemaParser::parse(code);
        let snapshot =
            Snapshot::from_parse_result(&result, Dialect::PostgreSQL, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            Snapshot::Sqlite(_) => panic!("Expected Postgres snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected postgres table");
        assert_eq!(table.schema.as_ref(), "auth");
        assert_eq!(table.name.as_ref(), "users_tbl");

        let user_id = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Column(c) = e
                && c.name.as_ref() == "user_id"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(user_id.is_some(), "expected explicit column name user_id");

        let created_at = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Column(c) = e
                && c.name.as_ref() == "created_at"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            created_at.is_some(),
            "expected inferred snake_case column name created_at"
        );

        let index = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let index = index.expect("expected postgres index");
        assert_eq!(index.name.as_ref(), "users_tbl_created_idx");
        assert_eq!(index.schema.as_ref(), "auth");
    }

    /// Test that changing a column from Option<String> to String generates table recreation
    #[test]
    fn test_nullable_to_not_null_generates_migration() {
        use crate::sqlite::collection::SQLiteDDL;
        use crate::sqlite::diff::compute_migration;

        // Previous schema: email is nullable (Option<String>)
        let prev_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
}
"#;

        // Current schema: email is NOT nullable (String)
        let cur_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub email: String,
}
"#;

        let prev_snap = sqlite_snapshot(prev_code);
        let cur_snap = sqlite_snapshot(cur_code);

        let prev_ddl = SQLiteDDL::from_entities(prev_snap.ddl.clone());
        let cur_ddl = SQLiteDDL::from_entities(cur_snap.ddl.clone());

        // Check that previous email column is nullable and current is not
        let prev_email = prev_ddl
            .columns
            .one("user", "email")
            .expect("email column in prev");
        let cur_email = cur_ddl
            .columns
            .one("user", "email")
            .expect("email column in cur");
        assert!(!prev_email.not_null, "Previous email should be nullable");
        assert!(cur_email.not_null, "Current email should be NOT NULL");

        // Compute migration
        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have SQL statements for table recreation
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate migration SQL for nullable change"
        );

        // Verify individual SQL statements for table recreation pattern
        assert_eq!(migration.sql_statements[0], "PRAGMA foreign_keys=OFF;");
        assert!(
            migration.sql_statements[1].starts_with("CREATE TABLE `__new_user`"),
            "Expected CREATE TABLE `__new_user`, got: {}",
            migration.sql_statements[1]
        );
        assert!(
            migration.sql_statements[1].contains("`email` TEXT NOT NULL"),
            "New table should have NOT NULL on email: {}",
            migration.sql_statements[1]
        );
        assert_eq!(
            migration.sql_statements[2],
            "INSERT INTO `__new_user`(`id`, `name`, `email`) SELECT `id`, `name`, `email` FROM `user`;"
        );
        assert_eq!(migration.sql_statements[3], "DROP TABLE `user`;");
        assert_eq!(
            migration.sql_statements[4],
            "ALTER TABLE `__new_user` RENAME TO `user`;"
        );
        assert_eq!(migration.sql_statements[5], "PRAGMA foreign_keys=ON;");
    }

    /// Test that changing a column from String to Option<String> generates table recreation
    #[test]
    fn test_not_null_to_nullable_generates_migration() {
        use crate::sqlite::collection::SQLiteDDL;
        use crate::sqlite::diff::compute_migration;

        // Previous schema: email is NOT nullable (String)
        let prev_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub email: String,
}
"#;

        // Current schema: email is nullable (Option<String>)
        let cur_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub email: Option<String>,
}
"#;

        let prev_snap = sqlite_snapshot(prev_code);
        let cur_snap = sqlite_snapshot(cur_code);

        let prev_ddl = SQLiteDDL::from_entities(prev_snap.ddl.clone());
        let cur_ddl = SQLiteDDL::from_entities(cur_snap.ddl.clone());

        // Compute migration
        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have SQL statements for table recreation
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate migration SQL for nullable change"
        );

        // Verify individual SQL statements for table recreation pattern
        assert_eq!(migration.sql_statements[0], "PRAGMA foreign_keys=OFF;");
        assert!(
            migration.sql_statements[1].starts_with("CREATE TABLE `__new_user`"),
            "Expected CREATE TABLE `__new_user`, got: {}",
            migration.sql_statements[1]
        );
        assert_eq!(migration.sql_statements[3], "DROP TABLE `user`;");
        assert_eq!(
            migration.sql_statements[4],
            "ALTER TABLE `__new_user` RENAME TO `user`;"
        );
        assert_eq!(migration.sql_statements[5], "PRAGMA foreign_keys=ON;");
    }
}
