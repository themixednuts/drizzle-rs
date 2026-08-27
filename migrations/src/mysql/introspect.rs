//! Driver-neutral MySQL 8 catalog introspection.
//!
//! Database adapters decode rows from the parameterized queries in [`queries`]
//! into the raw structs here. [`assemble_ddl`] is the only place that turns
//! transport data into the canonical MySQL migration model.

use std::borrow::Cow;
use std::collections::BTreeMap;

use drizzle_types::mysql::MySQLTypeCategory;
use drizzle_types::mysql::ddl::{
    CheckConstraint, Column, ForeignKey, Generated, GeneratedType, Index, IndexColumn, IndexMethod,
    InlineEnum, InlineType, PrimaryKey, ReferentialAction, Table, View, ViewAlgorithm,
    ViewCheckOption, ViewSqlSecurity,
};

use super::{MySQLCatalogDefaults, MySQLDDL, MySQLSnapshot, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntrospectError {
    #[error("MySQL connection has no selected database")]
    NoDatabase,
    #[error(
        "MySQL catalog references unsupported cross-database foreign key `{name}` from `{from}` to `{to}`"
    )]
    CrossDatabaseForeignKey {
        name: String,
        from: String,
        to: String,
    },
    #[error("invalid MySQL catalog metadata: {0}")]
    InvalidCatalog(String),
    #[error(transparent)]
    InvalidSnapshot(#[from] ValidationError),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawDatabaseInfo {
    pub name: String,
    pub default_engine: Option<String>,
    pub default_charset: Option<String>,
    pub default_collation: Option<String>,
}

impl RawDatabaseInfo {
    #[must_use]
    pub fn catalog_defaults(&self) -> MySQLCatalogDefaults {
        MySQLCatalogDefaults {
            engine: self.default_engine.clone(),
            charset: self.default_charset.clone(),
            collation: self.default_collation.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTableInfo {
    pub database: String,
    pub name: String,
    pub engine: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawColumnInfo {
    pub database: String,
    pub table: String,
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub extra: String,
    pub generation_expression: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub comment: Option<String>,
    pub ordinal_position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawIndexPart {
    pub database: String,
    pub table: String,
    pub name: String,
    pub non_unique: bool,
    pub sequence: u32,
    pub column_name: Option<String>,
    pub expression: Option<String>,
    pub prefix_length: Option<u32>,
    pub collation: Option<String>,
    pub index_type: Option<String>,
    pub comment: Option<String>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPrimaryKeyPart {
    pub database: String,
    pub table: String,
    pub constraint_name: String,
    pub column: String,
    pub ordinal_position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawForeignKeyPart {
    pub database: String,
    pub table: String,
    pub name: String,
    pub column: String,
    pub ordinal_position: u32,
    pub foreign_database: String,
    pub foreign_table: String,
    pub foreign_column: String,
    pub on_update: String,
    pub on_delete: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCheckInfo {
    pub database: String,
    pub table: String,
    pub name: String,
    pub expression: String,
    pub enforced: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawViewInfo {
    pub database: String,
    pub name: String,
    pub definition: String,
    pub algorithm: Option<String>,
    pub definer: Option<String>,
    pub sql_security: Option<String>,
    pub check_option: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawIntrospection {
    pub database: RawDatabaseInfo,
    pub tables: Vec<RawTableInfo>,
    pub columns: Vec<RawColumnInfo>,
    pub indexes: Vec<RawIndexPart>,
    pub primary_keys: Vec<RawPrimaryKeyPart>,
    pub foreign_keys: Vec<RawForeignKeyPart>,
    pub checks: Vec<RawCheckInfo>,
    pub views: Vec<RawViewInfo>,
}

#[derive(Debug, Clone)]
pub struct IntrospectionResult {
    pub ddl: MySQLDDL,
}

impl IntrospectionResult {
    #[must_use]
    pub fn to_snapshot(&self) -> MySQLSnapshot {
        let mut snapshot = MySQLSnapshot::new();
        snapshot.ddl = self.ddl.to_entities();
        snapshot
    }
}

/// Convert decoded catalog rows into a validated, deterministic MySQL DDL.
pub fn assemble_ddl(mut raw: RawIntrospection) -> Result<MySQLDDL, IntrospectError> {
    let database = raw.database.name.trim();
    if database.is_empty() {
        return Err(IntrospectError::NoDatabase);
    }

    raw.tables.sort_by(|left, right| left.name.cmp(&right.name));
    raw.columns.sort_by(|left, right| {
        (&left.table, left.ordinal_position, &left.name).cmp(&(
            &right.table,
            right.ordinal_position,
            &right.name,
        ))
    });

    let table_defaults = raw
        .tables
        .iter()
        .map(|table| {
            (
                table.name.clone(),
                (
                    table
                        .charset
                        .clone()
                        .or_else(|| raw.database.default_charset.clone()),
                    table
                        .collation
                        .clone()
                        .or_else(|| raw.database.default_collation.clone()),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut ddl = MySQLDDL::new();
    for table in raw.tables {
        ensure_database(database, &table.database)?;
        let mut entity = Table::new(table.name);
        entity.database = Some(Cow::Owned(database.to_string()));
        entity.engine = different(table.engine, raw.database.default_engine.as_deref());
        entity.charset = different(table.charset, raw.database.default_charset.as_deref());
        entity.collation = different(table.collation, raw.database.default_collation.as_deref());
        entity.comment = nonempty(table.comment);
        ddl.tables.push(entity);
    }

    for raw_column in raw.columns {
        ensure_database(database, &raw_column.database)?;
        if contains_word(&raw_column.extra, "INVISIBLE") {
            return Err(IntrospectError::InvalidCatalog(format!(
                "column `{}.{}` is invisible, which the MySQL migration model cannot preserve",
                raw_column.table, raw_column.name
            )));
        }
        let mut column = Column::new(
            raw_column.table.clone(),
            raw_column.name.clone(),
            raw_column.column_type.clone(),
        );
        column.database = Some(Cow::Owned(database.to_string()));
        column.not_null = !raw_column.nullable;
        column.autoincrement = contains_word(&raw_column.extra, "auto_increment");
        column.default = normalize_default(&raw_column);
        column.on_update = parse_on_update(&raw_column.extra).map(Cow::Owned);
        column.generated = generated_column(&raw_column)?;
        column.inline_type = parse_inline_type(&raw_column.column_type);
        let (table_charset, table_collation) = table_defaults
            .get(&raw_column.table)
            .map(|(charset, collation)| (charset.as_deref(), collation.as_deref()))
            .unwrap_or((
                raw.database.default_charset.as_deref(),
                raw.database.default_collation.as_deref(),
            ));
        column.charset = different(raw_column.charset, table_charset);
        column.collation = different(raw_column.collation, table_collation);
        column.comment = nonempty(raw_column.comment);
        ddl.columns.push(column);
    }

    for primary_key in group_primary_keys(raw.primary_keys, database)? {
        for primary_column in &primary_key.columns {
            let column = ddl
                .columns
                .list_mut()
                .iter_mut()
                .find(|column| {
                    column.database == primary_key.database
                        && column.table == primary_key.table
                        && column.name == *primary_column
                })
                .ok_or_else(|| {
                    IntrospectError::InvalidCatalog(format!(
                        "primary key `{}.{}` references missing column `{primary_column}`",
                        primary_key.table,
                        primary_key.name.as_deref().unwrap_or("PRIMARY")
                    ))
                })?;
            // MySQL's catalog exposes PRIMARY as a table constraint while the
            // public MySQLTable macro also records the same fact on each
            // column. Canonicalize both representations so pull -> parse and
            // push comparisons do not emit a spurious MODIFY COLUMN.
            column.primary_key = true;
        }
        ddl.pks.push(primary_key);
    }
    for index in group_indexes(raw.indexes, database)? {
        ddl.indexes.push(index);
    }
    for foreign_key in group_foreign_keys(raw.foreign_keys, database)? {
        ddl.fks.push(foreign_key);
    }
    raw.checks
        .sort_by(|left, right| (&left.table, &left.name).cmp(&(&right.table, &right.name)));
    for check in raw.checks {
        ensure_database(database, &check.database)?;
        let mut entity = CheckConstraint::new(check.table, check.name, check.expression);
        entity.database = Some(Cow::Owned(database.to_string()));
        entity.enforced = check.enforced;
        ddl.checks.push(entity);
    }

    raw.views.sort_by(|left, right| left.name.cmp(&right.name));
    for view in raw.views {
        ensure_database(database, &view.database)?;
        let mut entity = View::new(view.name, view.definition);
        entity.database = Some(Cow::Owned(database.to_string()));
        entity.algorithm = parse_view_algorithm(view.algorithm.as_deref())?;
        // DEFINER names the account whose privileges MySQL uses at runtime.
        // It is server-owned catalog metadata, not part of the MySQLView
        // source model, and retaining it would recreate every pulled view.
        entity.definer = None;
        entity.sql_security = parse_view_security(view.sql_security.as_deref())?;
        entity.check_option = parse_view_check(view.check_option.as_deref())?;
        entity.charset = nonempty(view.charset);
        entity.collation = nonempty(view.collation);
        ddl.views.push(entity);
    }

    let entities = ddl.to_entities();
    let ddl = MySQLDDL::try_from_entities(entities).map_err(IntrospectError::from)?;
    Ok(normalize_selected_database_scope(ddl))
}

/// Remove the connection-selected database from a validated catalog snapshot.
///
/// The catalog queries and validation above prove that every object belongs to
/// `DATABASE()`. That selected database is the execution context for a MySQL
/// push or pull, rather than an explicit schema declaration. Keeping the
/// snapshot unqualified makes it agree with ordinary `#[MySQLTable]` schemas
/// while explicit `DATABASE` attributes remain available for user-authored
/// qualified DDL.
fn normalize_selected_database_scope(mut ddl: MySQLDDL) -> MySQLDDL {
    for table in ddl.tables.list_mut() {
        table.database = None;
    }
    for column in ddl.columns.list_mut() {
        column.database = None;
    }
    for index in ddl.indexes.list_mut() {
        index.database = None;
    }
    for primary_key in ddl.pks.list_mut() {
        primary_key.database = None;
    }
    for unique in ddl.uniques.list_mut() {
        unique.database = None;
    }
    for foreign_key in ddl.fks.list_mut() {
        foreign_key.database = None;
        foreign_key.foreign_database = None;
    }
    for check in ddl.checks.list_mut() {
        check.database = None;
    }
    for view in ddl.views.list_mut() {
        view.database = None;
    }
    ddl
}

fn ensure_database(expected: &str, actual: &str) -> Result<(), IntrospectError> {
    if expected == actual {
        Ok(())
    } else {
        Err(IntrospectError::InvalidCatalog(format!(
            "expected database `{expected}`, got `{actual}`"
        )))
    }
}

fn different(value: Option<String>, inherited: Option<&str>) -> Option<Cow<'static, str>> {
    value
        .filter(|value| !value.is_empty() && Some(value.as_str()) != inherited)
        .map(Cow::Owned)
}

fn nonempty(value: Option<String>) -> Option<Cow<'static, str>> {
    value.filter(|value| !value.is_empty()).map(Cow::Owned)
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split_ascii_whitespace()
        .any(|word| word.eq_ignore_ascii_case(needle))
}

fn generated_column(column: &RawColumnInfo) -> Result<Option<Generated>, IntrospectError> {
    let expression = column
        .generation_expression
        .as_deref()
        .map(str::trim)
        .filter(|expression| !expression.is_empty());
    let extra = column.extra.to_ascii_lowercase();
    let kind = match (
        extra.contains("virtual generated"),
        extra.contains("stored generated"),
    ) {
        (true, false) => Some(GeneratedType::Virtual),
        (false, true) => Some(GeneratedType::Stored),
        (false, false) => None,
        (true, true) => {
            return Err(IntrospectError::InvalidCatalog(format!(
                "generated column `{}.{}` has conflicting storage metadata `{}`",
                column.table, column.name, column.extra
            )));
        }
    };
    match (expression, kind) {
        (None, None) => Ok(None),
        (Some(expression), Some(generation_type)) => Ok(Some(Generated {
            expression: Cow::Owned(expression.to_string()),
            generation_type,
        })),
        _ => Err(IntrospectError::InvalidCatalog(format!(
            "generated column `{}.{}` has inconsistent expression and EXTRA metadata",
            column.table, column.name
        ))),
    }
}

fn parse_on_update(extra: &str) -> Option<String> {
    let lower = extra.to_ascii_lowercase();
    let start = lower.find("on update")? + "on update".len();
    let value = extra[start..].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn normalize_default(column: &RawColumnInfo) -> Option<Cow<'static, str>> {
    if column.autogenerated_or_generated() {
        return None;
    }
    let value = column.default_value.as_deref()?.trim();
    let category = MySQLTypeCategory::from_sql_type(&column.column_type);
    let expression = column
        .extra
        .to_ascii_lowercase()
        .contains("default_generated")
        || looks_like_sql_default(value)
        || matches!(
            category,
            MySQLTypeCategory::TinyInt
                | MySQLTypeCategory::TinyIntUnsigned
                | MySQLTypeCategory::SmallInt
                | MySQLTypeCategory::SmallIntUnsigned
                | MySQLTypeCategory::MediumInt
                | MySQLTypeCategory::MediumIntUnsigned
                | MySQLTypeCategory::Int
                | MySQLTypeCategory::IntUnsigned
                | MySQLTypeCategory::BigInt
                | MySQLTypeCategory::BigIntUnsigned
                | MySQLTypeCategory::Decimal
                | MySQLTypeCategory::Float
                | MySQLTypeCategory::Double
                | MySQLTypeCategory::Boolean
                | MySQLTypeCategory::Bit
                | MySQLTypeCategory::Year
        );
    Some(Cow::Owned(if expression {
        value.to_string()
    } else {
        quote_string(value)
    }))
}

impl RawColumnInfo {
    fn autogenerated_or_generated(&self) -> bool {
        contains_word(&self.extra, "auto_increment")
            || self
                .generation_expression
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

fn looks_like_sql_default(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower == "null"
        || lower.starts_with("current_timestamp")
        || lower.starts_with("current_date")
        || lower.starts_with("current_time")
        || lower.starts_with("localtimestamp")
        || lower.starts_with("utc_timestamp")
        || lower.starts_with("now(")
        || lower.starts_with("b'")
        || lower.starts_with("x'")
        || lower.starts_with("0x")
        || (value.starts_with('(') && value.ends_with(')'))
}

fn quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
}

fn parse_inline_type(sql_type: &str) -> Option<InlineType> {
    if starts_type(sql_type, "enum") {
        parse_inline_values(sql_type)
            .map(InlineEnum::new)
            .map(InlineType::Enum)
    } else if starts_type(sql_type, "set") {
        parse_inline_values(sql_type)
            .map(InlineEnum::new)
            .map(InlineType::Set)
    } else {
        None
    }
}

fn starts_type(sql_type: &str, expected: &str) -> bool {
    sql_type
        .trim_start()
        .get(..expected.len())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn parse_inline_values(sql_type: &str) -> Option<Vec<String>> {
    let inner = sql_type.get(sql_type.find('(')? + 1..sql_type.rfind(')')?)?;
    let mut values = Vec::new();
    let mut chars = inner.chars().peekable();
    loop {
        while chars
            .peek()
            .is_some_and(|ch| ch.is_ascii_whitespace() || *ch == ',')
        {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        if chars.next()? != '\'' {
            return None;
        }
        let mut value = String::new();
        loop {
            match chars.next()? {
                '\\' => value.push(chars.next()?),
                '\'' if chars.peek() == Some(&'\'') => {
                    chars.next();
                    value.push('\'');
                }
                '\'' => break,
                ch => value.push(ch),
            }
        }
        values.push(value);
    }
    Some(values)
}

fn group_primary_keys(
    rows: Vec<RawPrimaryKeyPart>,
    database: &str,
) -> Result<Vec<PrimaryKey>, IntrospectError> {
    let mut groups = BTreeMap::<(String, String), Vec<RawPrimaryKeyPart>>::new();
    for row in rows {
        ensure_database(database, &row.database)?;
        groups
            .entry((row.table.clone(), row.constraint_name.clone()))
            .or_default()
            .push(row);
    }
    Ok(groups
        .into_iter()
        .map(|((table, _name), mut rows)| {
            rows.sort_by_key(|row| row.ordinal_position);
            let mut primary = PrimaryKey::new(table, rows.into_iter().map(|row| row.column));
            primary.database = Some(Cow::Owned(database.to_string()));
            // MySQL always exposes the fixed catalog name `PRIMARY`. The
            // public macro snapshot intentionally omits it because users
            // cannot choose another name, so keep the canonical form unnamed.
            primary.name = None;
            primary
        })
        .collect())
}

fn group_indexes(rows: Vec<RawIndexPart>, database: &str) -> Result<Vec<Index>, IntrospectError> {
    let mut groups = BTreeMap::<(String, String), Vec<RawIndexPart>>::new();
    for row in rows {
        ensure_database(database, &row.database)?;
        if row.name.eq_ignore_ascii_case("PRIMARY") {
            continue;
        }
        groups
            .entry((row.table.clone(), row.name.clone()))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|((table, name), mut rows)| {
            rows.sort_by_key(|row| row.sequence);
            let first = rows.first().ok_or_else(|| {
                IntrospectError::InvalidCatalog(format!("index `{table}.{name}` has no parts"))
            })?;
            let columns = rows
                .iter()
                .map(|row| {
                        let mut part = match (&row.column_name, &row.expression) {
                            (Some(column), None) => IndexColumn::column(column.clone()),
                            (None, Some(expression)) => IndexColumn::expression(expression.clone()),
                            _ => {
                                return Err(IntrospectError::InvalidCatalog(
                                    format!(
                                        "index `{table}.{name}` part {} must have exactly one of column name or expression",
                                        row.sequence
                                    ),
                                ));
                            }
                        };
                        part.length = row.prefix_length;
                        part.ascending = match row.collation.as_deref() {
                            None | Some("") => None,
                            Some(value) if value.eq_ignore_ascii_case("A") => Some(true),
                            Some(value) if value.eq_ignore_ascii_case("D") => Some(false),
                            Some(value) => {
                                return Err(IntrospectError::InvalidCatalog(format!(
                                    "index `{table}.{name}` part {} has unsupported collation `{value}`",
                                    row.sequence
                                )));
                            }
                        };
                        Ok(part)
                    })
                .collect::<Result<Vec<_>, _>>()?;
            let mut index = Index::new(table, name, columns);
            index.database = Some(Cow::Owned(database.to_string()));
            index.unique = !first.non_unique;
            index.using = match first.index_type.as_deref() {
                None | Some("") => None,
                Some(value) if value.eq_ignore_ascii_case("BTREE") => Some(IndexMethod::Btree),
                Some(value) if value.eq_ignore_ascii_case("HASH") => Some(IndexMethod::Hash),
                Some(value) => {
                    return Err(IntrospectError::InvalidCatalog(format!(
                        "index `{}` uses unsupported MySQL index type `{value}`",
                        index.name
                    )));
                }
            };
            index.comment = nonempty(first.comment.clone());
            index.visible = first.visible;
            Ok(index)
        })
        .collect()
}

fn group_foreign_keys(
    rows: Vec<RawForeignKeyPart>,
    database: &str,
) -> Result<Vec<ForeignKey>, IntrospectError> {
    let mut groups = BTreeMap::<(String, String), Vec<RawForeignKeyPart>>::new();
    for row in rows {
        ensure_database(database, &row.database)?;
        if row.foreign_database != database {
            return Err(IntrospectError::CrossDatabaseForeignKey {
                name: row.name,
                from: database.to_string(),
                to: row.foreign_database,
            });
        }
        groups
            .entry((row.table.clone(), row.name.clone()))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|((table, name), mut rows)| {
            rows.sort_by_key(|row| row.ordinal_position);
            let first = rows.first().ok_or_else(|| {
                IntrospectError::InvalidCatalog(format!("foreign key `{table}.{name}` is empty"))
            })?;
            let mut foreign_key = ForeignKey::new(
                table,
                name,
                rows.iter().map(|row| row.column.clone()),
                first.foreign_table.clone(),
                rows.iter().map(|row| row.foreign_column.clone()),
            );
            foreign_key.database = Some(Cow::Owned(database.to_string()));
            foreign_key.foreign_database = Some(Cow::Owned(database.to_string()));
            foreign_key.on_update = referential_action(&first.on_update)?;
            foreign_key.on_delete = referential_action(&first.on_delete)?;
            Ok(foreign_key)
        })
        .collect()
}

fn referential_action(value: &str) -> Result<Option<ReferentialAction>, IntrospectError> {
    let action = match value.trim().to_ascii_uppercase().as_str() {
        "CASCADE" => Some(ReferentialAction::Cascade),
        "SET NULL" => Some(ReferentialAction::SetNull),
        "RESTRICT" => Some(ReferentialAction::Restrict),
        "NO ACTION" => Some(ReferentialAction::NoAction),
        "" => None,
        other => {
            return Err(IntrospectError::InvalidCatalog(format!(
                "unsupported referential action `{other}`"
            )));
        }
    };
    Ok(action)
}

fn parse_view_algorithm(value: Option<&str>) -> Result<Option<ViewAlgorithm>, IntrospectError> {
    parse_enum(
        value,
        |value| match value {
            "UNDEFINED" => Some(ViewAlgorithm::Undefined),
            "MERGE" => Some(ViewAlgorithm::Merge),
            "TEMPTABLE" => Some(ViewAlgorithm::Temptable),
            _ => None,
        },
        "view algorithm",
    )
}

fn parse_view_security(value: Option<&str>) -> Result<Option<ViewSqlSecurity>, IntrospectError> {
    parse_enum(
        value,
        |value| match value {
            "DEFINER" => Some(ViewSqlSecurity::Definer),
            "INVOKER" => Some(ViewSqlSecurity::Invoker),
            _ => None,
        },
        "view SQL security",
    )
}

fn parse_view_check(value: Option<&str>) -> Result<Option<ViewCheckOption>, IntrospectError> {
    parse_enum(
        value.filter(|value| !value.eq_ignore_ascii_case("NONE")),
        |value| match value {
            "CASCADED" => Some(ViewCheckOption::Cascaded),
            "LOCAL" => Some(ViewCheckOption::Local),
            _ => None,
        },
        "view check option",
    )
}

fn parse_enum<T>(
    value: Option<&str>,
    parse: impl FnOnce(&str) -> Option<T>,
    kind: &str,
) -> Result<Option<T>, IntrospectError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_ascii_uppercase();
    parse(&normalized)
        .map(Some)
        .ok_or_else(|| IntrospectError::InvalidCatalog(format!("unsupported {kind} `{value}`")))
}

/// Enrich a view row with metadata only exposed by `SHOW CREATE VIEW`.
///
/// `information_schema.VIEWS` is still the source of the normalized query
/// definition. This parser deliberately extracts only the stable header
/// options so formatting inside the view query cannot confuse it.
pub fn apply_show_create_view(view: &mut RawViewInfo, create_sql: &str) {
    let header_end = find_ascii_case_insensitive(create_sql, " VIEW ").unwrap_or(create_sql.len());
    let header = &create_sql[..header_end];
    view.algorithm = header_value(header, "ALGORITHM=").map(str::to_string);
    view.definer = header_value(header, "DEFINER=").map(str::to_string);
    if let Some(position) = find_ascii_case_insensitive(header, "SQL SECURITY ") {
        let value = header[position + "SQL SECURITY ".len()..]
            .split_ascii_whitespace()
            .next();
        view.sql_security = value.map(str::to_string);
    }
}

fn header_value<'a>(header: &'a str, key: &str) -> Option<&'a str> {
    let position = find_ascii_case_insensitive(header, key)? + key.len();
    let rest = header[position..].trim_start();
    let mut characters = rest.char_indices().peekable();
    let mut quote = None;
    let mut end = rest.len();
    while let Some((index, character)) = characters.next() {
        if let Some(terminator) = quote {
            if character == terminator {
                if characters
                    .peek()
                    .is_some_and(|(_, next)| *next == terminator)
                {
                    characters.next();
                } else {
                    quote = None;
                }
            }
        } else if matches!(character, '`' | '\'' | '"') {
            quote = Some(character);
        } else if character.is_ascii_whitespace() {
            end = index;
            break;
        }
    }
    Some(&rest[..end])
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Parameterized catalog queries. Every `?` is the one selected database;
/// adapters must bind it rather than interpolate user input.
pub mod queries {
    pub const DATABASE: &str = r#"
SELECT s.SCHEMA_NAME, @@default_storage_engine,
       s.DEFAULT_CHARACTER_SET_NAME, s.DEFAULT_COLLATION_NAME
FROM information_schema.SCHEMATA s
WHERE s.SCHEMA_NAME = DATABASE()"#;

    pub const TABLES: &str = r#"
SELECT t.TABLE_SCHEMA, t.TABLE_NAME, t.ENGINE,
       ccsa.CHARACTER_SET_NAME, t.TABLE_COLLATION, t.TABLE_COMMENT
FROM information_schema.TABLES t
LEFT JOIN information_schema.COLLATION_CHARACTER_SET_APPLICABILITY ccsa
  ON ccsa.COLLATION_NAME = t.TABLE_COLLATION
WHERE t.TABLE_SCHEMA = ? AND t.TABLE_TYPE = 'BASE TABLE'
ORDER BY t.TABLE_NAME"#;

    pub const COLUMNS: &str = r#"
SELECT c.TABLE_SCHEMA, c.TABLE_NAME, c.COLUMN_NAME, c.COLUMN_TYPE,
       c.IS_NULLABLE, c.COLUMN_DEFAULT, c.EXTRA, c.GENERATION_EXPRESSION,
       c.CHARACTER_SET_NAME, c.COLLATION_NAME, c.COLUMN_COMMENT,
       c.ORDINAL_POSITION
FROM information_schema.COLUMNS c
WHERE c.TABLE_SCHEMA = ?
ORDER BY c.TABLE_NAME, c.ORDINAL_POSITION"#;

    pub const INDEXES: &str = r#"
SELECT s.TABLE_SCHEMA, s.TABLE_NAME, s.INDEX_NAME, s.NON_UNIQUE,
       s.SEQ_IN_INDEX, s.COLUMN_NAME, s.EXPRESSION, s.SUB_PART,
       s.COLLATION, s.INDEX_TYPE, s.INDEX_COMMENT, s.IS_VISIBLE
FROM information_schema.STATISTICS s
WHERE s.TABLE_SCHEMA = ?
ORDER BY s.TABLE_NAME, s.INDEX_NAME, s.SEQ_IN_INDEX"#;

    pub const PRIMARY_KEYS: &str = r#"
SELECT k.TABLE_SCHEMA, k.TABLE_NAME, k.CONSTRAINT_NAME,
       k.COLUMN_NAME, k.ORDINAL_POSITION
FROM information_schema.TABLE_CONSTRAINTS tc
JOIN information_schema.KEY_COLUMN_USAGE k
  ON k.CONSTRAINT_SCHEMA = tc.CONSTRAINT_SCHEMA
 AND k.TABLE_SCHEMA = tc.TABLE_SCHEMA
 AND k.TABLE_NAME = tc.TABLE_NAME
 AND k.CONSTRAINT_NAME = tc.CONSTRAINT_NAME
WHERE tc.TABLE_SCHEMA = ? AND tc.CONSTRAINT_TYPE = 'PRIMARY KEY'
ORDER BY k.TABLE_NAME, k.ORDINAL_POSITION"#;

    pub const FOREIGN_KEYS: &str = r#"
SELECT k.TABLE_SCHEMA, k.TABLE_NAME, k.CONSTRAINT_NAME, k.COLUMN_NAME,
       k.ORDINAL_POSITION, k.REFERENCED_TABLE_SCHEMA,
       k.REFERENCED_TABLE_NAME, k.REFERENCED_COLUMN_NAME,
       r.UPDATE_RULE, r.DELETE_RULE
FROM information_schema.KEY_COLUMN_USAGE k
JOIN information_schema.REFERENTIAL_CONSTRAINTS r
  ON r.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA
 AND r.TABLE_NAME = k.TABLE_NAME
 AND r.CONSTRAINT_NAME = k.CONSTRAINT_NAME
WHERE k.TABLE_SCHEMA = ? AND k.REFERENCED_TABLE_NAME IS NOT NULL
ORDER BY k.TABLE_NAME, k.CONSTRAINT_NAME, k.ORDINAL_POSITION"#;

    pub const CHECKS: &str = r#"
SELECT tc.TABLE_SCHEMA, tc.TABLE_NAME, tc.CONSTRAINT_NAME,
       cc.CHECK_CLAUSE, tc.ENFORCED
FROM information_schema.TABLE_CONSTRAINTS tc
JOIN information_schema.CHECK_CONSTRAINTS cc
  ON cc.CONSTRAINT_SCHEMA = tc.CONSTRAINT_SCHEMA
 AND cc.CONSTRAINT_NAME = tc.CONSTRAINT_NAME
WHERE tc.TABLE_SCHEMA = ? AND tc.CONSTRAINT_TYPE = 'CHECK'
ORDER BY tc.TABLE_NAME, tc.CONSTRAINT_NAME"#;

    pub const VIEWS: &str = r#"
SELECT v.TABLE_SCHEMA, v.TABLE_NAME, v.VIEW_DEFINITION, v.DEFINER,
       v.SECURITY_TYPE, v.CHECK_OPTION, v.CHARACTER_SET_CLIENT,
       v.COLLATION_CONNECTION
FROM information_schema.VIEWS v
WHERE v.TABLE_SCHEMA = ?
ORDER BY v.TABLE_NAME"#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mysql::{DiffOptions, compute_migration, compute_migration_with};
    use crate::{parser::SchemaParser, schema::Snapshot};
    use drizzle_types::{Dialect, mysql::ddl::MySQLEntity};

    fn raw() -> RawIntrospection {
        RawIntrospection {
            database: RawDatabaseInfo {
                name: "app".to_string(),
                default_engine: Some("InnoDB".to_string()),
                default_charset: Some("utf8mb4".to_string()),
                default_collation: Some("utf8mb4_0900_ai_ci".to_string()),
            },
            tables: vec![RawTableInfo {
                database: "app".to_string(),
                name: "users".to_string(),
                engine: Some("InnoDB".to_string()),
                charset: Some("utf8mb4".to_string()),
                collation: Some("utf8mb4_0900_ai_ci".to_string()),
                comment: None,
            }],
            columns: vec![
                RawColumnInfo {
                    database: "app".to_string(),
                    table: "users".to_string(),
                    name: "id".to_string(),
                    column_type: "int unsigned".to_string(),
                    nullable: false,
                    default_value: None,
                    extra: "auto_increment".to_string(),
                    generation_expression: None,
                    charset: None,
                    collation: None,
                    comment: None,
                    ordinal_position: 1,
                },
                RawColumnInfo {
                    database: "app".to_string(),
                    table: "users".to_string(),
                    name: "status".to_string(),
                    column_type: "enum('new','it''s done')".to_string(),
                    nullable: false,
                    default_value: Some("new".to_string()),
                    extra: String::new(),
                    generation_expression: None,
                    charset: Some("utf8mb4".to_string()),
                    collation: Some("utf8mb4_0900_ai_ci".to_string()),
                    comment: None,
                    ordinal_position: 2,
                },
            ],
            primary_keys: vec![RawPrimaryKeyPart {
                database: "app".to_string(),
                table: "users".to_string(),
                constraint_name: "PRIMARY".to_string(),
                column: "id".to_string(),
                ordinal_position: 1,
            }],
            ..RawIntrospection::default()
        }
    }

    #[test]
    fn catalog_assembly_omits_inherited_options_and_preserves_inline_types() {
        let ddl = assemble_ddl(raw()).expect("valid catalog");
        let table = ddl.tables.list().first().expect("table");
        assert_eq!(table.engine, None);
        assert_eq!(table.charset, None);
        let id = ddl
            .columns
            .list()
            .iter()
            .find(|column| column.name == "id")
            .expect("id");
        assert!(
            id.primary_key,
            "catalog PRIMARY must normalize column metadata"
        );
        let status = ddl
            .columns
            .list()
            .iter()
            .find(|column| column.name == "status")
            .expect("status");
        assert_eq!(status.default.as_deref(), Some("'new'"));
        let Some(InlineType::Enum(values)) = &status.inline_type else {
            panic!("expected inline enum")
        };
        assert_eq!(
            values.values.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["new", "it's done"]
        );
        assert!(matches!(
            IntrospectionResult { ddl }.to_snapshot().ddl[0],
            MySQLEntity::Table(_)
        ));
    }

    #[test]
    fn selected_database_scope_matches_unqualified_schema() {
        let current = assemble_ddl(RawIntrospection {
            database: RawDatabaseInfo {
                name: "app".to_string(),
                ..RawDatabaseInfo::default()
            },
            tables: vec![RawTableInfo {
                database: "app".to_string(),
                name: "users".to_string(),
                engine: None,
                charset: None,
                collation: None,
                comment: None,
            }],
            columns: vec![RawColumnInfo {
                database: "app".to_string(),
                table: "users".to_string(),
                name: "id".to_string(),
                column_type: "int".to_string(),
                nullable: false,
                default_value: None,
                extra: String::new(),
                generation_expression: None,
                charset: None,
                collation: None,
                comment: None,
                ordinal_position: 1,
            }],
            ..RawIntrospection::default()
        })
        .expect("valid selected-database catalog");

        let parsed = SchemaParser::parse(
            r#"
#[MySQLTable(NAME = "users")]
pub struct Users {
    pub id: i32,
}
"#,
        );
        assert!(parsed.errors.is_empty(), "{:#?}", parsed.errors);
        let Snapshot::MySQL(snapshot) = Snapshot::from_parse_result(&parsed, Dialect::MySQL, None)
        else {
            panic!("expected MySQL snapshot")
        };
        let desired =
            MySQLDDL::try_from_entities(snapshot.ddl).expect("ordinary MySQLTable schema is valid");

        assert_eq!(current.database_scope().expect("one scope"), None);
        let migration = compute_migration(&current, &desired)
            .expect("selected database and an unqualified schema must agree");
        assert!(migration.statements.is_empty());
    }

    #[test]
    fn collapsed_catalog_defaults_compare_cleanly_with_explicit_schema_options() {
        let raw = raw();
        let defaults = raw.database.catalog_defaults();
        let current = assemble_ddl(raw).expect("valid catalog");
        let mut desired = current.clone();
        desired.tables.list_mut()[0].engine = Some("InnoDB".into());
        desired.tables.list_mut()[0].charset = Some("utf8mb4".into());
        desired.tables.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());
        let status = desired
            .columns
            .list_mut()
            .iter_mut()
            .find(|column| column.name == "status")
            .expect("status");
        status.charset = Some("utf8mb4".into());
        status.collation = Some("utf8mb4_0900_ai_ci".into());
        let options = DiffOptions {
            catalog_defaults: Some(defaults),
            ..DiffOptions::default()
        };

        let migration = compute_migration_with(&current, &desired, &options).expect("valid diff");

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn cross_database_foreign_keys_are_rejected() {
        let mut raw = raw();
        raw.foreign_keys.push(RawForeignKeyPart {
            database: "app".to_string(),
            table: "users".to_string(),
            name: "users_org_fk".to_string(),
            column: "id".to_string(),
            ordinal_position: 1,
            foreign_database: "other".to_string(),
            foreign_table: "orgs".to_string(),
            foreign_column: "id".to_string(),
            on_update: "NO ACTION".to_string(),
            on_delete: "RESTRICT".to_string(),
        });
        assert!(matches!(
            assemble_ddl(raw),
            Err(IntrospectError::CrossDatabaseForeignKey { .. })
        ));
    }

    #[test]
    fn invisible_columns_are_rejected_instead_of_becoming_visible() {
        let mut raw = raw();
        raw.columns[0].extra = "auto_increment INVISIBLE".to_string();

        let error = assemble_ddl(raw).expect_err("invisible column is unsupported");
        assert!(
            error.to_string().contains("column `users.id` is invisible"),
            "{error}"
        );
    }

    #[test]
    fn generated_columns_require_matching_catalog_metadata() {
        let mut raw = raw();
        raw.columns[1].generation_expression = Some("(`id` + 1)".to_string());

        let error = assemble_ddl(raw).expect_err("generated kind is required");
        assert!(
            error.to_string().contains("inconsistent expression"),
            "{error}"
        );
    }

    #[test]
    fn index_parts_reject_unknown_catalog_order() {
        let mut raw = raw();
        raw.indexes.push(RawIndexPart {
            database: "app".to_string(),
            table: "users".to_string(),
            name: "users_status_idx".to_string(),
            non_unique: true,
            sequence: 1,
            column_name: Some("status".to_string()),
            expression: None,
            prefix_length: None,
            collation: Some("SIDEWAYS".to_string()),
            index_type: Some("BTREE".to_string()),
            comment: None,
            visible: Some(true),
        });

        let error = assemble_ddl(raw).expect_err("unknown index order is invalid");
        assert!(
            error.to_string().contains("unsupported collation"),
            "{error}"
        );
    }

    #[test]
    fn show_create_view_supplies_persistent_header_metadata() {
        let mut view = RawViewInfo {
            database: "app".to_string(),
            name: "active_users".to_string(),
            definition: "select `users`.`id` AS `id` from `users`".to_string(),
            algorithm: None,
            definer: None,
            sql_security: None,
            check_option: None,
            charset: None,
            collation: None,
        };
        apply_show_create_view(
            &mut view,
            "CREATE ALGORITHM=MERGE DEFINER=`app user`@`%` SQL SECURITY INVOKER VIEW `active_users` AS select 1",
        );
        assert_eq!(view.algorithm.as_deref(), Some("MERGE"));
        assert_eq!(view.definer.as_deref(), Some("`app user`@`%`"));
        assert_eq!(view.sql_security.as_deref(), Some("INVOKER"));
    }

    #[test]
    fn catalog_view_definer_is_not_part_of_the_migration_snapshot() {
        let mut raw = raw();
        raw.views.push(RawViewInfo {
            database: "app".to_string(),
            name: "active_users".to_string(),
            definition: "select `users`.`id` AS `id` from `users`".to_string(),
            algorithm: Some("MERGE".to_string()),
            definer: Some("app_user@%".to_string()),
            sql_security: Some("DEFINER".to_string()),
            check_option: Some("NONE".to_string()),
            charset: Some("utf8mb4".to_string()),
            collation: Some("utf8mb4_0900_ai_ci".to_string()),
        });

        let ddl = assemble_ddl(raw).expect("valid catalog");
        let view = ddl.views.list().first().expect("view");

        assert_eq!(view.definer, None);
        assert_eq!(view.sql_security, Some(ViewSqlSecurity::Definer));
    }
}
