//! Schema snapshot builder from parsed schema files.
//!
//! This module converts [`ParseResult`] from the schema parser into
//! [`Snapshot`] values used for migration diffing.
//!
//! It is shared by runtime/build-time migration generation flows that do not
//! rely on the CLI.

use crate::parser::{ParseResult, ParsedField, ParsedIndex, ParsedTable};
use crate::postgres::PostgresSnapshot;
use crate::schema::Snapshot;
use crate::sqlite::SQLiteSnapshot;
use drizzle_types::postgres::{PostgreSQLType, TypeCategory as PgTypeCategory};
use drizzle_types::sqlite::{SQLiteType, TypeCategory as SQLiteTypeCategory};
use drizzle_types::{Casing, Dialect};
use heck::{ToLowerCamelCase, ToSnakeCase};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// Convert a `ParseResult` into a `Snapshot` for migration diffing
///
/// Uses the provided `dialect` from config rather than the parser-detected dialect,
/// allowing users to have multi-dialect schema files and select which to use via config.
#[must_use]
pub fn parse_result_to_snapshot(
    result: &ParseResult,
    dialect: Dialect,
    casing: Option<Casing>,
) -> Snapshot {
    match dialect {
        Dialect::SQLite => Snapshot::Sqlite(build_sqlite_snapshot(result, casing)),
        Dialect::PostgreSQL => Snapshot::Postgres(build_postgres_snapshot(result, casing)),
        Dialect::MySQL => {
            unreachable!("Unsupported dialect for snapshot generation: {dialect:?}")
        }
    }
}

fn apply_casing(name: &str, casing: Casing) -> String {
    match casing {
        Casing::SnakeCase => name.to_snake_case(),
        Casing::CamelCase => name.to_lower_camel_case(),
    }
}

fn trim_wrapping_quotes(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ParsedIndexAttrs {
    name: Option<String>,
}

impl ParsedIndexAttrs {
    fn parse(attr: &str) -> Self {
        let Some(start) = attr.find('(') else {
            return Self::default();
        };
        let Some(end) = attr.rfind(')') else {
            return Self::default();
        };

        let mut parsed = Self::default();
        let content = &attr[start + 1..end];
        for part in content.split(',') {
            let part = part.trim();
            if let Some((k, v)) = part.split_once('=')
                && k.trim() == "name"
            {
                parsed.name = Some(trim_wrapping_quotes(v));
            }
        }

        parsed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemberRef<'a> {
    table: &'a str,
    field: &'a str,
}

impl<'a> MemberRef<'a> {
    fn parse(raw: &'a str) -> Option<Self> {
        let (table, field) = raw.split_once("::")?;
        if table.is_empty() || field.is_empty() || field.contains("::") {
            return None;
        }

        Some(Self { table, field })
    }
}

fn sqlite_type_sql(ty: SQLiteType) -> String {
    ty.to_sql_type().to_ascii_lowercase()
}

fn postgres_type_sql(ty: &PostgreSQLType) -> String {
    ty.to_sql_type().to_ascii_lowercase()
}

fn resolve_table_name(table: &ParsedTable, casing: Casing) -> String {
    table.attr_value("name").map_or_else(
        || apply_casing(&table.name, casing),
        |v| trim_wrapping_quotes(&v),
    )
}

fn resolve_field_name(field: &ParsedField, casing: Casing) -> String {
    field.attr_value("name").map_or_else(
        || apply_casing(&field.name, casing),
        |v| trim_wrapping_quotes(&v),
    )
}

fn resolve_sqlite_type(field: &ParsedField) -> SQLiteType {
    explicit_sqlite_type(field).unwrap_or_else(|| infer_sqlite_type(&field.ty))
}

fn explicit_sqlite_type(field: &ParsedField) -> Option<SQLiteType> {
    for attr in &field.attrs {
        let Some(name) = attr_name(attr) else {
            continue;
        };

        if let Some(ty) = sqlite_type_marker(name) {
            return Some(ty);
        }

        if !name.eq_ignore_ascii_case("column") {
            continue;
        }

        let Some(args) = attr_args(attr) else {
            continue;
        };

        for part in split_attr_parts(args) {
            let marker = marker_key(part);
            if let Some(ty) = sqlite_type_marker(marker) {
                return Some(ty);
            }
        }
    }

    None
}

fn sqlite_type_marker(marker: &str) -> Option<SQLiteType> {
    if marker.eq_ignore_ascii_case("json") {
        Some(SQLiteType::Text)
    } else if marker.eq_ignore_ascii_case("jsonb") {
        Some(SQLiteType::Blob)
    } else {
        SQLiteType::from_attribute_name(marker)
    }
}

fn attr_name(attr: &str) -> Option<&str> {
    let rest = attr.trim().strip_prefix("#[")?;
    let end = rest.find(['(', ']'])?;
    Some(rest[..end].trim())
}

fn attr_args(attr: &str) -> Option<&str> {
    let start = attr.find('(')?;
    let end = attr.rfind(')')?;
    (start < end).then_some(&attr[start + 1..end])
}

fn marker_key(part: &str) -> &str {
    let part = part.trim();
    let end = part.find(['=', '(']).unwrap_or(part.len());
    part[..end].trim()
}

fn marker_args(part: &str) -> Option<&str> {
    let start = part.find('(')?;
    let end = part.rfind(')')?;
    (start < end).then_some(&part[start + 1..end])
}

fn marker_value(part: &str) -> Option<&str> {
    part.split_once('=')
        .map(|(_, value)| value.trim())
        .or_else(|| marker_args(part).map(str::trim))
}

fn field_marker_part<'a>(field: &'a ParsedField, marker: &str) -> Option<&'a str> {
    for attr in &field.attrs {
        let Some(name) = attr_name(attr) else {
            continue;
        };

        if name.eq_ignore_ascii_case(marker) {
            return Some(attr);
        }

        if !name.eq_ignore_ascii_case("column") {
            continue;
        }

        let Some(args) = attr_args(attr) else {
            continue;
        };

        for part in split_attr_parts(args) {
            if marker_key(part).eq_ignore_ascii_case(marker) {
                return Some(part);
            }
        }
    }

    None
}

fn field_has_marker(field: &ParsedField, marker: &str) -> bool {
    field_marker_part(field, marker).is_some()
}

fn field_marker_args<'a>(field: &'a ParsedField, marker: &str) -> Option<&'a str> {
    let part = field_marker_part(field, marker)?;
    if part.trim_start().starts_with("#[") {
        attr_args(part)
    } else {
        marker_args(part)
    }
}

fn field_marker_value(field: &ParsedField, marker: &str) -> Option<String> {
    let part = field_marker_part(field, marker)?;
    let raw_value = if part.trim_start().starts_with("#[") {
        attr_args(part)?
    } else {
        marker_value(part)?
    };
    Some(trim_wrapping_quotes(raw_value))
}

fn split_attr_parts(content: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (i, c) in content.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match c {
            '\\' if in_single || in_double => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' | '<' | '[' | '{' if !in_single && !in_double => depth += 1,
            ')' | '>' | ']' | '}' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
            }
            ',' if depth == 0 && !in_single && !in_double => {
                parts.push(content[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < content.len() {
        parts.push(content[start..].trim());
    }

    parts
}

/// Build an `SQLite` snapshot from parsed schema
fn build_sqlite_snapshot(result: &ParseResult, casing: Option<Casing>) -> SQLiteSnapshot {
    use crate::sqlite::{PrimaryKey, SqliteEntity, Table, UniqueConstraint};

    let mut snapshot = SQLiteSnapshot::new();
    let name_casing = casing.unwrap_or(Casing::SnakeCase);

    let sqlite_tables: Vec<_> = result
        .tables
        .values()
        .filter(|t| t.dialect == Dialect::SQLite)
        .collect();

    let mut table_name_map: HashMap<String, String> = HashMap::new();
    let mut field_name_map: HashMap<(String, String), String> = HashMap::new();
    for table in &sqlite_tables {
        let table_name = resolve_table_name(table, name_casing);
        table_name_map.insert(table.name.clone(), table_name);
        for field in &table.fields {
            field_name_map.insert(
                (table.name.clone(), field.name.clone()),
                resolve_field_name(field, name_casing),
            );
        }
    }

    // Process tables (only those matching SQLite dialect)
    for table in sqlite_tables {
        let table_name = table_name_map
            .get(&table.name)
            .cloned()
            .unwrap_or_else(|| resolve_table_name(table, name_casing));

        // Add table entity
        let mut sqlite_table = Table::new(table_name.clone());
        sqlite_table.strict = table.is_strict();
        sqlite_table.without_rowid = table.is_without_rowid();
        snapshot.add_entity(SqliteEntity::Table(sqlite_table));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col_name = field_name_map
                .get(&(table.name.clone(), field.name.clone()))
                .cloned()
                .unwrap_or_else(|| resolve_field_name(field, name_casing));
            let col = build_sqlite_column(&table_name, field, &col_name);
            snapshot.add_entity(SqliteEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(col_name.clone());
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                let constraint_name = format!("{table_name}_{col_name}_unique");
                snapshot.add_entity(SqliteEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        table_name.clone(),
                        constraint_name,
                        vec![col_name.clone()],
                    ),
                ));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references()
                && let Some(fk) = build_sqlite_foreign_key(
                    &table_name,
                    &col_name,
                    field,
                    &ref_target,
                    &table_name_map,
                    &field_name_map,
                    name_casing,
                )
            {
                snapshot.add_entity(SqliteEntity::ForeignKey(fk));
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            let pk_name = format!("{table_name}_pkey");
            snapshot.add_entity(SqliteEntity::PrimaryKey(PrimaryKey::from_strings(
                table_name, pk_name, pk_columns,
            )));
        }
    }

    // Process indexes (only those matching SQLite dialect)
    for index in result
        .indexes
        .values()
        .filter(|i| i.dialect == Dialect::SQLite)
    {
        let idx = build_sqlite_index(index, &table_name_map, &field_name_map, name_casing);
        snapshot.add_entity(SqliteEntity::Index(idx));
    }

    snapshot
}

/// Name/schema maps for `PostgreSQL` snapshot building, built once up front
/// and reused for column/FK/index resolution.
struct PgNameMaps {
    /// Parsed struct name -> resolved SQL table name.
    table_name_map: HashMap<String, String>,
    /// (parsed struct name, parsed field name) -> resolved SQL column name.
    field_name_map: HashMap<(String, String), String>,
    /// Parsed struct name -> `PostgreSQL` schema ("public" by default).
    table_schemas: HashMap<String, String>,
    /// All distinct schemas discovered in the schema set.
    schema_list: Vec<String>,
}

fn build_pg_name_maps(pg_tables: &[&ParsedTable], casing: Casing) -> PgNameMaps {
    let mut table_name_map: HashMap<String, String> = HashMap::new();
    let mut field_name_map: HashMap<(String, String), String> = HashMap::new();
    for table in pg_tables {
        table_name_map.insert(table.name.clone(), resolve_table_name(table, casing));
        for field in &table.fields {
            field_name_map.insert(
                (table.name.clone(), field.name.clone()),
                resolve_field_name(field, casing),
            );
        }
    }

    let mut table_schemas: HashMap<String, String> = HashMap::new();
    let mut schemas: HashSet<String> = HashSet::new();
    for table in pg_tables {
        let schema_name = table.schema_name().unwrap_or_else(|| "public".to_string());
        table_schemas.insert(table.name.clone(), schema_name.clone());
        schemas.insert(schema_name);
    }
    if schemas.is_empty() {
        schemas.insert("public".to_string());
    }

    let mut schema_list: Vec<String> = schemas.into_iter().collect();
    schema_list.sort();

    PgNameMaps {
        table_name_map,
        field_name_map,
        table_schemas,
        schema_list,
    }
}

/// Add table, column, unique, primary-key and FK entities for a single
/// parsed `PostgreSQL` table.
fn add_postgres_table_entities(
    snapshot: &mut PostgresSnapshot,
    table: &ParsedTable,
    maps: &PgNameMaps,
    casing: Casing,
) {
    use crate::postgres::{PostgresEntity, PrimaryKey, Table, UniqueConstraint};

    let table_name = maps
        .table_name_map
        .get(&table.name)
        .cloned()
        .unwrap_or_else(|| resolve_table_name(table, casing));
    let schema_name = table.schema_name().unwrap_or_else(|| "public".to_string());

    snapshot.add_entity(PostgresEntity::Table(Table {
        schema: schema_name.clone().into(),
        name: table_name.clone().into(),
        is_unlogged: None,
        is_temporary: None,
        inherits: None,
        tablespace: None,
        is_rls_enabled: None,
        comment: None,
    }));

    let mut pk_columns = Vec::new();

    for field in &table.fields {
        let col_name = maps
            .field_name_map
            .get(&(table.name.clone(), field.name.clone()))
            .cloned()
            .unwrap_or_else(|| resolve_field_name(field, casing));
        let col = build_postgres_column(&schema_name, &table_name, field, &col_name);
        snapshot.add_entity(PostgresEntity::Column(col));

        if field.is_primary_key() {
            pk_columns.push(col_name.clone());
        }

        if field.is_unique() && !field.is_primary_key() {
            snapshot.add_entity(PostgresEntity::UniqueConstraint(
                UniqueConstraint::from_strings(
                    schema_name.clone(),
                    table_name.clone(),
                    format!("{table_name}_{col_name}_key"),
                    vec![col_name.clone()],
                ),
            ));
        }

        if let Some(ref_target) = field.references()
            && let Some(fk) = build_postgres_foreign_key(
                &schema_name,
                &table_name,
                &col_name,
                field,
                &ref_target,
                &maps.table_name_map,
                &maps.field_name_map,
                &maps.table_schemas,
                casing,
            )
        {
            snapshot.add_entity(PostgresEntity::ForeignKey(fk));
        }
    }

    if !pk_columns.is_empty() {
        snapshot.add_entity(PostgresEntity::PrimaryKey(PrimaryKey::from_strings(
            schema_name,
            table_name.clone(),
            format!("{table_name}_pkey"),
            pk_columns,
        )));
    }
}

/// Build a `PostgreSQL` snapshot from parsed schema
fn build_postgres_snapshot(result: &ParseResult, casing: Option<Casing>) -> PostgresSnapshot {
    use crate::postgres::{PostgresEntity, Schema as PgSchema};

    let mut snapshot = PostgresSnapshot::new();
    let name_casing = casing.unwrap_or(Casing::SnakeCase);

    let pg_tables: Vec<_> = result
        .tables
        .values()
        .filter(|t| t.dialect == Dialect::PostgreSQL)
        .collect();

    let maps = build_pg_name_maps(&pg_tables, name_casing);

    for schema in &maps.schema_list {
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new(schema.clone())));
    }

    for table in pg_tables {
        add_postgres_table_entities(&mut snapshot, table, &maps, name_casing);
    }

    // Process indexes (only those matching PostgreSQL dialect)
    for index in result
        .indexes
        .values()
        .filter(|i| i.dialect == Dialect::PostgreSQL)
    {
        let idx = build_postgres_index(
            index,
            &maps.table_name_map,
            &maps.field_name_map,
            &maps.table_schemas,
            name_casing,
        );
        snapshot.add_entity(PostgresEntity::Index(idx));
    }

    snapshot
}

/// Build an `SQLite` column from a parsed field
fn build_sqlite_column(
    table_name: &str,
    field: &ParsedField,
    col_name: &str,
) -> crate::sqlite::Column {
    use crate::sqlite::Column;

    let col_type = resolve_sqlite_type(field);

    let mut col = Column::new(
        table_name.to_string(),
        col_name.to_string(),
        sqlite_type_sql(col_type),
    );

    if !field.is_nullable() {
        col = col.not_null();
    }

    if field.is_autoincrement() {
        col = col.autoincrement();
    }

    if let Some(default) = field.default_value() {
        col = col.default_value(default);
    }

    col
}

fn resolve_postgres_type(field: &ParsedField) -> PostgreSQLType {
    if field_has_marker(field, "smallserial") {
        PostgreSQLType::Smallserial
    } else if field_has_marker(field, "bigserial") {
        PostgreSQLType::Bigserial
    } else if field_has_marker(field, "serial") {
        PostgreSQLType::Serial
    } else if field_has_marker(field, "json") {
        PostgreSQLType::Json
    } else if field_has_marker(field, "jsonb") {
        PostgreSQLType::Jsonb
    } else {
        infer_postgres_type(&field.ty)
    }
}

fn postgres_identity(
    schema_name: &str,
    table_name: &str,
    col_name: &str,
    field: &ParsedField,
) -> Option<crate::postgres::Identity> {
    use crate::postgres::Identity;
    use crate::postgres::ddl::IdentityType;

    if !field_has_marker(field, "identity") {
        return None;
    }

    let type_ = match field_marker_args(field, "identity")
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("by_default") => IdentityType::ByDefault,
        Some("always") | None => IdentityType::Always,
        Some(_) => IdentityType::Always,
    };

    Some(Identity {
        name: format!("{table_name}_{col_name}_seq").into(),
        schema: Some(schema_name.to_string().into()),
        type_,
        increment: None,
        min_value: None,
        max_value: None,
        start_with: None,
        cache: None,
        cycle: None,
    })
}

fn postgres_generated(field: &ParsedField) -> Option<crate::postgres::Generated> {
    use crate::postgres::Generated;
    use crate::postgres::ddl::GeneratedType;

    let args = field_marker_args(field, "generated")?;
    let parts = split_attr_parts(args);
    let [kind, expression] = parts.as_slice() else {
        return None;
    };

    if !kind.trim().eq_ignore_ascii_case("stored") {
        return None;
    }

    Some(Generated {
        expression: trim_wrapping_quotes(expression).into(),
        gen_type: GeneratedType::Stored,
    })
}

/// Build a `PostgreSQL` column from a parsed field
fn build_postgres_column(
    schema_name: &str,
    table_name: &str,
    field: &ParsedField,
    col_name: &str,
) -> crate::postgres::Column {
    use crate::postgres::Column;

    let col_type = resolve_postgres_type(field);
    let generated = postgres_generated(field);
    let identity = postgres_identity(schema_name, table_name, col_name, field);
    let default = if matches!(
        &col_type,
        PostgreSQLType::Smallserial | PostgreSQLType::Serial | PostgreSQLType::Bigserial
    ) || generated.is_some()
        || identity.is_some()
    {
        None
    } else {
        field.default_value().map(Cow::Owned)
    };

    Column {
        schema: schema_name.to_string().into(),
        table: table_name.to_string().into(),
        name: col_name.to_string().into(),
        sql_type: postgres_type_sql(&col_type).into(),
        type_schema: None,
        not_null: !field.is_nullable(),
        default,
        generated,
        identity,
        dimensions: None,
        comment: None,
        collate: field_marker_value(field, "collate").map(Cow::Owned),
        ordinal_position: None,
    }
}

/// Build an `SQLite` foreign key from a parsed field
fn build_sqlite_foreign_key(
    table_name: &str,
    col_name: &str,
    field: &ParsedField,
    ref_target: &str,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    casing: Casing,
) -> Option<crate::sqlite::ForeignKey> {
    use crate::sqlite::ForeignKey;

    let target = MemberRef::parse(ref_target)?;

    let ref_table = table_name_map
        .get(target.table)
        .cloned()
        .unwrap_or_else(|| apply_casing(target.table, casing));
    let ref_column = field_name_map
        .get(&(target.table.to_string(), target.field.to_string()))
        .cloned()
        .unwrap_or_else(|| apply_casing(target.field, casing));
    let fk_name = format!("{table_name}_{col_name}_{ref_table}_{ref_column}_fk");

    let mut fk = ForeignKey::from_strings(
        table_name.to_string(),
        fk_name,
        vec![col_name.to_string()],
        ref_table,
        vec![ref_column],
    );

    fk.on_delete = field.on_delete().map(Cow::Owned);
    fk.on_update = field.on_update().map(Cow::Owned);

    Some(fk)
}

/// Build a `PostgreSQL` foreign key from a parsed field
#[allow(clippy::too_many_arguments)]
fn build_postgres_foreign_key(
    schema_name: &str,
    table_name: &str,
    col_name: &str,
    field: &ParsedField,
    ref_target: &str,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    table_schemas: &HashMap<String, String>,
    casing: Casing,
) -> Option<crate::postgres::ForeignKey> {
    use crate::postgres::ForeignKey;

    let target = MemberRef::parse(ref_target)?;
    let ref_table_struct = target.table;
    let ref_table = table_name_map
        .get(ref_table_struct)
        .cloned()
        .unwrap_or_else(|| apply_casing(ref_table_struct, casing));
    let ref_column = field_name_map
        .get(&(ref_table_struct.to_string(), target.field.to_string()))
        .cloned()
        .unwrap_or_else(|| apply_casing(target.field, casing));
    let ref_schema = table_schemas
        .get(ref_table_struct)
        .cloned()
        .unwrap_or_else(|| "public".to_string());
    let fk_name = format!("{table_name}_{col_name}_{ref_table}_{ref_column}_fk");

    Some(ForeignKey {
        schema: schema_name.to_string().into(),
        table: table_name.to_string().into(),
        name: fk_name.into(),
        name_explicit: false,
        columns: Cow::Owned(vec![Cow::Owned(col_name.to_string())]),
        schema_to: ref_schema.into(),
        table_to: ref_table.into(),
        columns_to: Cow::Owned(vec![Cow::Owned(ref_column)]),
        on_update: field.on_update().map(Cow::Owned),
        on_delete: field.on_delete().map(Cow::Owned),
        deferrable: false,
        initially_deferred: false,
    })
}

/// Build an `SQLite` index from a parsed index
fn build_sqlite_index(
    index: &ParsedIndex,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    casing: Casing,
) -> crate::sqlite::Index {
    use crate::sqlite::{Index, IndexColumn, IndexOrigin};

    let table_struct = index.table_name().unwrap_or_default();
    let table_name = table_name_map
        .get(table_struct)
        .cloned()
        .unwrap_or_else(|| apply_casing(table_struct, casing));
    let index_attrs = ParsedIndexAttrs::parse(&index.attr);
    let index_name = index_attrs
        .name
        .unwrap_or_else(|| apply_casing(&index.name, casing));

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            let target = MemberRef::parse(c)?;
            let col_name = field_name_map
                .get(&(target.table.to_string(), target.field.to_string()))
                .cloned()
                .unwrap_or_else(|| apply_casing(target.field, casing));
            Some(IndexColumn::new(col_name))
        })
        .collect();

    Index {
        table: table_name.into(),
        name: index_name.into(),
        columns,
        is_unique: index.is_unique(),
        where_clause: None,
        origin: IndexOrigin::Manual,
    }
}

/// Build a `PostgreSQL` index from a parsed index
fn build_postgres_index(
    index: &ParsedIndex,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    table_schemas: &HashMap<String, String>,
    casing: Casing,
) -> crate::postgres::Index {
    use crate::postgres::{Index, IndexColumn};

    let table_struct = index.table_name().unwrap_or_default();
    let table_name = table_name_map
        .get(table_struct)
        .cloned()
        .unwrap_or_else(|| apply_casing(table_struct, casing));
    let schema_name = table_schemas
        .get(table_struct)
        .cloned()
        .unwrap_or_else(|| "public".to_string());
    let index_attrs = ParsedIndexAttrs::parse(&index.attr);
    let index_name = index_attrs
        .name
        .unwrap_or_else(|| apply_casing(&index.name, casing));

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            let target = MemberRef::parse(c)?;
            let col_name = field_name_map
                .get(&(target.table.to_string(), target.field.to_string()))
                .cloned()
                .unwrap_or_else(|| apply_casing(target.field, casing));
            Some(IndexColumn::new(col_name))
        })
        .collect();

    Index {
        schema: schema_name.into(),
        table: table_name.into(),
        name: index_name.into(),
        name_explicit: false,
        columns,
        is_unique: index.is_unique(),
        where_clause: index.where_clause().map(Cow::Owned),
        method: index.method().map(Cow::Owned),
        with: None,
        concurrently: index.is_concurrent(),
    }
}

/// Infer `SQLite` type from Rust type string
fn infer_sqlite_type(rust_type: &str) -> SQLiteType {
    match SQLiteTypeCategory::from_type_string(rust_type) {
        SQLiteTypeCategory::Unknown => SQLiteType::Any,
        category => category.to_sqlite_type().unwrap_or(SQLiteType::Any),
    }
}

/// Infer `PostgreSQL` type from Rust type string
fn infer_postgres_type(rust_type: &str) -> PostgreSQLType {
    let base_type = rust_type
        .trim()
        .strip_prefix("Option<")
        .and_then(|s| s.strip_suffix(">"))
        .unwrap_or(rust_type)
        .trim();

    match base_type {
        "u8" | "u16" | "u32" => PostgreSQLType::Integer,
        "u64" => PostgreSQLType::Bigint,
        "&str" | "str" => PostgreSQLType::Text,
        "[u8]" => PostgreSQLType::Bytea,
        _ if base_type.contains("Decimal") => PostgreSQLType::Numeric,
        _ => PgTypeCategory::from_type_string(rust_type)
            .to_postgres_type()
            .unwrap_or(PostgreSQLType::Text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_sqlite_type() {
        assert_eq!(infer_sqlite_type("i32"), SQLiteType::Integer);
        assert_eq!(infer_sqlite_type("i64"), SQLiteType::Integer);
        assert_eq!(infer_sqlite_type("f64"), SQLiteType::Real);
        assert_eq!(infer_sqlite_type("String"), SQLiteType::Text);
        assert_eq!(
            infer_sqlite_type("compact_str::CompactString"),
            SQLiteType::Text
        );
        assert_eq!(infer_sqlite_type("bytes::Bytes"), SQLiteType::Blob);
        assert_eq!(
            infer_sqlite_type("smallvec::SmallVec<[u8; 16]>"),
            SQLiteType::Blob
        );
        assert_eq!(infer_sqlite_type("Option<String>"), SQLiteType::Text);
        assert_eq!(infer_sqlite_type("Vec<u8>"), SQLiteType::Blob);
        assert_eq!(infer_sqlite_type("Uuid"), SQLiteType::Blob);
        assert_eq!(infer_sqlite_type("uuid::Uuid"), SQLiteType::Blob);
        assert_eq!(infer_sqlite_type("Option<uuid::Uuid>"), SQLiteType::Blob);
    }

    #[test]
    fn test_infer_postgres_type() {
        assert_eq!(infer_postgres_type("i32"), PostgreSQLType::Integer);
        assert_eq!(infer_postgres_type("i64"), PostgreSQLType::Bigint);
        assert_eq!(infer_postgres_type("bool"), PostgreSQLType::Boolean);
        assert_eq!(infer_postgres_type("String"), PostgreSQLType::Text);
        assert_eq!(
            infer_postgres_type("compact_str::CompactString"),
            PostgreSQLType::Varchar
        );
        assert_eq!(
            infer_postgres_type("arrayvec::ArrayString<32>"),
            PostgreSQLType::Varchar
        );
        assert_eq!(infer_postgres_type("bytes::Bytes"), PostgreSQLType::Bytea);
        assert_eq!(
            infer_postgres_type("smallvec::SmallVec<[u8; 16]>"),
            PostgreSQLType::Bytea
        );
        assert_eq!(infer_postgres_type("Vec<u8>"), PostgreSQLType::Bytea);
        assert_eq!(infer_postgres_type("Uuid"), PostgreSQLType::Uuid);
        assert_eq!(
            infer_postgres_type("serde_json::Value"),
            PostgreSQLType::Jsonb
        );
    }

    #[test]
    fn test_postgres_snapshot_preserves_column_markers() {
        use crate::parser::SchemaParser;
        use crate::postgres::ddl::{GeneratedType, IdentityType, PostgresEntity};

        let code = r#"
#[PostgresTable(schema = "app")]
pub struct PgMarkers {
    #[column(serial, primary, default = 1)]
    pub id: i32,
    #[column(smallserial)]
    pub small_id: i16,
    #[column(bigserial)]
    pub big_id: i64,
    #[column(json)]
    pub json_doc: AppDoc,
    #[column(jsonb)]
    pub jsonb_doc: AppDoc,
    #[column(identity(by_default), default = 2)]
    pub identity_id: i32,
    #[column(generated(stored, "first_name || ' ' || last_name"), default = "'ignored'")]
    pub full_name: String,
    #[column(collate = "C")]
    pub sortable: String,
}
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::PostgreSQL, None);
        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
        };

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

        assert_eq!(column("id").sql_type.as_ref(), "serial");
        assert!(column("id").identity.is_none());
        assert!(column("id").default.is_none());
        assert_eq!(column("small_id").sql_type.as_ref(), "smallserial");
        assert_eq!(column("big_id").sql_type.as_ref(), "bigserial");
        assert_eq!(column("json_doc").sql_type.as_ref(), "json");
        assert_eq!(column("jsonb_doc").sql_type.as_ref(), "jsonb");

        let identity = column("identity_id")
            .identity
            .as_ref()
            .expect("expected identity");
        assert_eq!(identity.type_, IdentityType::ByDefault);
        assert!(column("identity_id").default.is_none());

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
        use crate::parser::SchemaParser;
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

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, None);
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
        };

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

        assert_eq!(column_type("blob_uuid"), "blob");
        assert_eq!(column_type("text_uuid"), "text");
        assert_eq!(column_type("legacy_blob_uuid"), "blob");
        assert_eq!(column_type("legacy_text_uuid"), "text");
    }

    /// Test that changing a column from Option<String> to String generates table recreation
    #[test]
    fn test_nullable_to_not_null_generates_migration() {
        use crate::parser::SchemaParser;
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

        let prev_result = SchemaParser::parse(prev_code);
        let cur_result = SchemaParser::parse(cur_code);

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite, None);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite, None);

        // Extract DDL from snapshots
        let (prev_ddl, cur_ddl) = match (&prev_snapshot, &cur_snapshot) {
            (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => (
                SQLiteDDL::from_entities(p.ddl.clone()),
                SQLiteDDL::from_entities(c.ddl.clone()),
            ),
            _ => panic!("Expected SQLite snapshots"),
        };

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
        use crate::parser::SchemaParser;
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

        let prev_result = SchemaParser::parse(prev_code);
        let cur_result = SchemaParser::parse(cur_code);

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite, None);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite, None);

        // Extract DDL from snapshots
        let (prev_ddl, cur_ddl) = match (&prev_snapshot, &cur_snapshot) {
            (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => (
                SQLiteDDL::from_entities(p.ddl.clone()),
                SQLiteDDL::from_entities(c.ddl.clone()),
            ),
            _ => panic!("Expected SQLite snapshots"),
        };

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

    #[test]
    fn test_postgres_schema_and_index_options_are_preserved() {
        use crate::parser::SchemaParser;
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

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::PostgreSQL, None);

        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
        };

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
    }

    #[test]
    fn test_sqlite_table_options_and_pk_name_are_preserved() {
        use crate::parser::SchemaParser;
        use crate::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(strict, without_rowid)]
pub struct Accounts {
    #[column(primary)]
    pub id: i64,
}
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, None);
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
        };

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
        assert_eq!(pk.name.as_ref(), "accounts_pkey");
    }

    #[test]
    fn test_sqlite_casing_preserves_explicit_names() {
        use crate::parser::SchemaParser;
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
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
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
    }

    #[test]
    fn test_postgres_casing_preserves_explicit_names() {
        use crate::parser::SchemaParser;
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
            parse_result_to_snapshot(&result, Dialect::PostgreSQL, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
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
}
