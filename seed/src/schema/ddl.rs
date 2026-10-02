//! Building a [`Schema`] from migration-snapshot DDL: the entities a
//! snapshot file stores, and the ones a driver's `introspect()` returns.

use super::check::enum_check;
use super::{Column, Schema, Table};
use std::collections::HashMap;

/// A DDL entity type a [`Schema`] can be built from with
/// [`Schema::from_ddl`]: `drizzle_types::sqlite::ddl::SqliteEntity`,
/// `drizzle_types::postgres::ddl::PostgresEntity` or
/// `drizzle_types::mysql::ddl::MySQLEntity` (each behind its dialect
/// feature).
pub trait DdlEntity: sealed::Sealed + Sized {
    #[doc(hidden)]
    fn build_schema<'a>(entities: impl IntoIterator<Item = &'a Self>) -> Schema
    where
        Self: 'a;
}

mod sealed {
    pub trait Sealed {}
}

impl Schema {
    /// Builds a schema from snapshot entities, such as the `ddl` of a
    /// `drizzle_migrations` snapshot. With the `migrations` feature,
    /// [`from_snapshot`](Self::from_snapshot) takes the snapshot itself.
    ///
    /// Read from the entities: tables and their namespaces, column types,
    /// `NOT NULL`, defaults, generated columns, primary keys, foreign
    /// keys, `UNIQUE` constraints and unique indexes, `PostgreSQL` enums,
    /// identity and `SERIAL` columns (a `nextval(...)` default), arrays, and
    /// `CHECK (col IN ('a', 'b'))` constraints, whose values the column then
    /// picks from. Views, and keys that name a column the table does not
    /// have, are ignored.
    ///
    /// Other `CHECK` constraints are not read: give such a column a
    /// generator whose values pass it.
    #[must_use]
    pub fn from_ddl<'a, E: DdlEntity + 'a>(entities: impl IntoIterator<Item = &'a E>) -> Self {
        E::build_schema(entities)
    }
}

/// A table being assembled from entities that refer to it by name.
#[derive(Default)]
struct TableParts {
    schema: Option<String>,
    name: String,
    /// `(ordinal position, column)`; sorted by position when every column
    /// has one, otherwise kept in entity order.
    columns: Vec<(Option<i32>, Column)>,
    primary_key: Option<Vec<String>>,
    unique: Vec<Vec<String>>,
    foreign_keys: Vec<ForeignKeyParts>,
    checks: Vec<String>,
}

struct ForeignKeyParts {
    columns: Vec<String>,
    target_schema: Option<String>,
    target_table: String,
    target_columns: Vec<String>,
}

type TableKey = (Option<String>, String);

/// Collects the parts of every table, in the order the tables appear.
#[derive(Default)]
struct Parts {
    tables: Vec<TableParts>,
    index: HashMap<TableKey, usize>,
}

impl Parts {
    fn add_table(&mut self, schema: Option<&str>, name: &str) {
        let key = (schema.map(str::to_owned), name.to_owned());
        if self.index.contains_key(&key) {
            return;
        }
        self.index.insert(key, self.tables.len());
        self.tables.push(TableParts {
            schema: schema.map(str::to_owned),
            name: name.to_owned(),
            ..TableParts::default()
        });
    }

    /// The table `name` in `schema`, if a table entity declared it. Columns
    /// and keys of views or unknown tables are skipped.
    fn table(&mut self, schema: Option<&str>, name: &str) -> Option<&mut TableParts> {
        let index = *self
            .index
            .get(&(schema.map(str::to_owned), name.to_owned()))?;
        self.tables.get_mut(index)
    }

    /// Fills in each table's primary key from its columns' own key flags
    /// when no separate key entity declared one.
    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    fn primary_keys_from_column_flags(&mut self) {
        for table in &mut self.tables {
            if table.primary_key.is_some() {
                continue;
            }
            let flagged: Vec<String> = table
                .columns
                .iter()
                .filter(|(_, column)| column.primary_key)
                .map(|(_, column)| column.name.clone())
                .collect();
            if !flagged.is_empty() {
                table.primary_key = Some(flagged);
            }
        }
    }

    fn into_schema(self, mut schema: Schema) -> Schema {
        for parts in self.tables {
            schema.tables.push(parts.into_table());
        }
        schema
    }
}

impl TableParts {
    fn add_unique(&mut self, columns: Vec<String>) {
        if !self.unique.contains(&columns) {
            self.unique.push(columns);
        }
    }

    fn into_table(mut self) -> Table {
        if self.columns.iter().all(|(position, _)| position.is_some()) {
            self.columns.sort_by_key(|(position, _)| *position);
        }
        for check in &self.checks {
            let Some((column, values)) = enum_check(check) else {
                continue;
            };
            if let Some((_, column)) = self
                .columns
                .iter_mut()
                .find(|(_, candidate)| candidate.name == column)
                && column.enum_values.is_none()
            {
                column.enum_values = Some(values);
            }
        }

        let mut table = Table::new(self.name);
        table.schema = self.schema;
        let names: Vec<String> = self
            .columns
            .iter()
            .map(|(_, column)| column.name.clone())
            .collect();
        let known = |columns: &[String]| {
            !columns.is_empty() && columns.iter().all(|name| names.contains(name))
        };
        table.columns = self.columns.into_iter().map(|(_, column)| column).collect();
        table.primary_key = self.primary_key.filter(|columns| known(columns));
        table.unique = self
            .unique
            .into_iter()
            .filter(|columns| known(columns))
            .collect();
        table.foreign_keys = self
            .foreign_keys
            .into_iter()
            .filter(|fk| known(&fk.columns) && fk.columns.len() == fk.target_columns.len())
            .map(|fk| super::ForeignKey {
                columns: fk.columns,
                target_schema: fk.target_schema,
                target_table: fk.target_table,
                target_columns: fk.target_columns,
            })
            .collect();
        table
    }
}

fn strings<S: AsRef<str>>(values: &[S]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect()
}

/// The column names of a unique index, or `None` when one of its keys is an
/// expression (its uniqueness then does not map onto single columns).
fn index_columns<'c>(columns: impl IntoIterator<Item = (&'c str, bool)>) -> Option<Vec<String>> {
    columns
        .into_iter()
        .map(|(value, is_expression)| (!is_expression).then(|| value.to_owned()))
        .collect()
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::{DdlEntity, Parts, Schema, index_columns, sealed, strings};
    use crate::schema::Column;
    use drizzle_types::sqlite::ddl::SqliteEntity;

    impl sealed::Sealed for SqliteEntity {}

    impl DdlEntity for SqliteEntity {
        fn build_schema<'a>(entities: impl IntoIterator<Item = &'a Self>) -> Schema {
            let entities: Vec<&Self> = entities.into_iter().collect();
            let mut parts = Parts::default();
            for entity in &entities {
                if let SqliteEntity::Table(table) = entity {
                    parts.add_table(None, &table.name);
                }
            }
            for entity in &entities {
                match entity {
                    SqliteEntity::Column(column) => {
                        let Some(table) = parts.table(None, &column.table) else {
                            continue;
                        };
                        let mut built = Column::new(&*column.name, &*column.sql_type);
                        built.not_null = column.not_null;
                        built.unique = column.unique == Some(true);
                        built.has_default = column.default.is_some();
                        built.auto_increment = column.autoincrement == Some(true);
                        built.generated = column
                            .generated
                            .as_ref()
                            .map(|generated| generated.expression.to_string());
                        if column.primary_key == Some(true) {
                            built.primary_key = true;
                            built.not_null = true;
                        }
                        table.columns.push((column.ordinal_position, built));
                    }
                    SqliteEntity::PrimaryKey(pk) => {
                        if let Some(table) = parts.table(None, &pk.table) {
                            table.primary_key = Some(strings(&pk.columns));
                        }
                    }
                    SqliteEntity::UniqueConstraint(unique) => {
                        if let Some(table) = parts.table(None, &unique.table) {
                            table.add_unique(strings(&unique.columns));
                        }
                    }
                    SqliteEntity::Index(index) if index.is_unique => {
                        if let Some(table) = parts.table(None, &index.table)
                            && let Some(columns) = index_columns(
                                index
                                    .columns
                                    .iter()
                                    .map(|column| (&*column.value, column.is_expression)),
                            )
                        {
                            table.add_unique(columns);
                        }
                    }
                    SqliteEntity::ForeignKey(fk) => {
                        if let Some(table) = parts.table(None, &fk.table) {
                            table.foreign_keys.push(super::ForeignKeyParts {
                                columns: strings(&fk.columns),
                                target_schema: None,
                                target_table: fk.table_to.to_string(),
                                target_columns: strings(&fk.columns_to),
                            });
                        }
                    }
                    SqliteEntity::CheckConstraint(check) => {
                        if let Some(table) = parts.table(None, &check.table) {
                            table.checks.push(check.value.to_string());
                        }
                    }
                    _ => {}
                }
            }
            parts.primary_keys_from_column_flags();
            parts.into_schema(Schema::sqlite())
        }
    }
}

#[cfg(feature = "postgres")]
mod postgres {
    use super::{DdlEntity, Parts, Schema, index_columns, sealed, strings};
    use crate::schema::Column;
    use drizzle_types::postgres::ddl::{IdentityType, PostgresEntity};
    use std::collections::HashMap;

    impl sealed::Sealed for PostgresEntity {}

    /// How to name a type in a cast: quoted when PostgreSQL would otherwise
    /// fold it to lowercase, and qualified outside `public`.
    fn type_name(schema: &str, name: &str) -> String {
        let quote = |identifier: &str| {
            let plain = identifier.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
                && identifier
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '$');
            if plain {
                identifier.to_owned()
            } else {
                format!("\"{}\"", identifier.replace('"', "\"\""))
            }
        };
        if schema == "public" || schema.is_empty() {
            quote(name)
        } else {
            format!("{}.{}", quote(schema), quote(name))
        }
    }

    impl DdlEntity for PostgresEntity {
        fn build_schema<'a>(entities: impl IntoIterator<Item = &'a Self>) -> Schema {
            let entities: Vec<&Self> = entities.into_iter().collect();
            let mut parts = Parts::default();
            let mut enums: HashMap<(&str, &str), &[std::borrow::Cow<'static, str>]> =
                HashMap::new();
            for entity in &entities {
                match entity {
                    PostgresEntity::Table(table) => {
                        parts.add_table(Some(&table.schema), &table.name);
                    }
                    PostgresEntity::Enum(enum_type) => {
                        enums.insert((&enum_type.schema, &enum_type.name), &enum_type.values);
                    }
                    _ => {}
                }
            }
            for entity in &entities {
                match entity {
                    PostgresEntity::Column(column) => {
                        let Some(table) = parts.table(Some(&column.schema), &column.table) else {
                            continue;
                        };
                        let type_schema = column.type_schema.as_deref().unwrap_or("public");
                        let enum_values = enums.get(&(type_schema, &*column.sql_type));
                        let mut sql_type = if enum_values.is_some() {
                            type_name(type_schema, &column.sql_type)
                        } else {
                            column.sql_type.to_string()
                        };
                        for _ in 0..column.dimensions.unwrap_or(0) {
                            sql_type.push_str("[]");
                        }

                        let mut built = Column::new(&*column.name, sql_type);
                        built.not_null = column.not_null;
                        built.enum_values = enum_values.map(|values| strings(values));
                        built.generated = column
                            .generated
                            .as_ref()
                            .map(|generated| generated.expression.to_string());
                        match column.identity.as_ref().map(|identity| identity.type_) {
                            Some(IdentityType::Always) => {
                                built.auto_increment = true;
                                built.identity_always = true;
                            }
                            Some(IdentityType::ByDefault) => built.auto_increment = true,
                            None => {}
                        }
                        match column.default.as_deref() {
                            Some(default) if default.trim_start().starts_with("nextval(") => {
                                built.sequence_default = true;
                            }
                            Some(_) => built.has_default = true,
                            None => {}
                        }
                        table.columns.push((column.ordinal_position, built));
                    }
                    PostgresEntity::PrimaryKey(pk) => {
                        if let Some(table) = parts.table(Some(&pk.schema), &pk.table) {
                            table.primary_key = Some(strings(&pk.columns));
                        }
                    }
                    PostgresEntity::UniqueConstraint(unique) => {
                        if let Some(table) = parts.table(Some(&unique.schema), &unique.table) {
                            table.add_unique(strings(&unique.columns));
                        }
                    }
                    PostgresEntity::Index(index) if index.is_unique => {
                        if let Some(table) = parts.table(Some(&index.schema), &index.table)
                            && let Some(columns) = index_columns(
                                index
                                    .columns
                                    .iter()
                                    .map(|column| (&*column.value, column.is_expression)),
                            )
                        {
                            table.add_unique(columns);
                        }
                    }
                    PostgresEntity::ForeignKey(fk) => {
                        if let Some(table) = parts.table(Some(&fk.schema), &fk.table) {
                            table.foreign_keys.push(super::ForeignKeyParts {
                                columns: strings(&fk.columns),
                                target_schema: Some(fk.schema_to.to_string()),
                                target_table: fk.table_to.to_string(),
                                target_columns: strings(&fk.columns_to),
                            });
                        }
                    }
                    PostgresEntity::CheckConstraint(check) => {
                        if let Some(table) = parts.table(Some(&check.schema), &check.table) {
                            table.checks.push(check.value.to_string());
                        }
                    }
                    _ => {}
                }
            }
            parts.into_schema(Schema::postgres())
        }
    }
}

#[cfg(feature = "mysql")]
mod mysql {
    use super::{DdlEntity, Parts, Schema, index_columns, sealed, strings};
    use crate::schema::Column;
    use drizzle_types::mysql::ddl::MySQLEntity;

    impl sealed::Sealed for MySQLEntity {}

    impl DdlEntity for MySQLEntity {
        fn build_schema<'a>(entities: impl IntoIterator<Item = &'a Self>) -> Schema {
            let entities: Vec<&Self> = entities.into_iter().collect();
            let mut parts = Parts::default();
            for entity in &entities {
                if let MySQLEntity::Table(table) = entity {
                    parts.add_table(table.database.as_deref(), &table.name);
                }
            }
            for entity in &entities {
                match entity {
                    MySQLEntity::Column(column) => {
                        let Some(table) = parts.table(column.database.as_deref(), &column.table)
                        else {
                            continue;
                        };
                        let mut built = Column::new(&*column.name, &*column.sql_type);
                        built.not_null = column.not_null;
                        built.unique = column.unique;
                        built.has_default = column.default.is_some();
                        built.auto_increment = column.autoincrement;
                        built.generated = column
                            .generated
                            .as_ref()
                            .map(|generated| generated.expression.to_string());
                        if column.primary_key {
                            built.primary_key = true;
                            built.not_null = true;
                        }
                        table.columns.push((None, built));
                    }
                    MySQLEntity::PrimaryKey(pk) => {
                        if let Some(table) = parts.table(pk.database.as_deref(), &pk.table) {
                            table.primary_key = Some(strings(&pk.columns));
                        }
                    }
                    MySQLEntity::UniqueConstraint(unique) => {
                        if let Some(table) = parts.table(unique.database.as_deref(), &unique.table)
                        {
                            table.add_unique(strings(&unique.columns));
                        }
                    }
                    MySQLEntity::Index(index) if index.unique => {
                        if let Some(table) = parts.table(index.database.as_deref(), &index.table)
                            && let Some(columns) =
                                index_columns(index.columns.iter().map(|column| {
                                    // A prefix key (`name(10)`) is unique on
                                    // the prefix only, which the whole value
                                    // does not track.
                                    (
                                        &*column.expression,
                                        column.is_expression || column.length.is_some(),
                                    )
                                }))
                        {
                            table.add_unique(columns);
                        }
                    }
                    MySQLEntity::ForeignKey(fk) => {
                        if let Some(table) = parts.table(fk.database.as_deref(), &fk.table) {
                            table.foreign_keys.push(super::ForeignKeyParts {
                                columns: strings(&fk.columns),
                                target_schema: fk
                                    .foreign_database
                                    .as_deref()
                                    .or(fk.database.as_deref())
                                    .map(str::to_owned),
                                target_table: fk.foreign_table.to_string(),
                                target_columns: strings(&fk.foreign_columns),
                            });
                        }
                    }
                    MySQLEntity::CheckConstraint(check) => {
                        if let Some(table) = parts.table(check.database.as_deref(), &check.table) {
                            table.checks.push(check.expression.to_string());
                        }
                    }
                    _ => {}
                }
            }
            parts.primary_keys_from_column_flags();
            parts.into_schema(Schema::mysql())
        }
    }
}

#[cfg(feature = "migrations")]
impl Schema {
    /// Builds a schema from a `drizzle_migrations` snapshot: one loaded from
    /// a migration folder's `snapshot.json`, or the one a drizzle driver's
    /// `introspect()` reads from a live database. See
    /// [`from_ddl`](Self::from_ddl) for what is read.
    ///
    /// # Errors
    ///
    /// Returns [`SeedError::DialectNotEnabled`](crate::SeedError::DialectNotEnabled)
    /// when this crate's feature for the snapshot's dialect is off.
    pub fn from_snapshot(
        snapshot: &drizzle_migrations::Snapshot,
    ) -> Result<Self, crate::SeedError> {
        use drizzle_migrations::Snapshot;
        match snapshot {
            #[cfg(feature = "sqlite")]
            Snapshot::Sqlite(snapshot) => Ok(Self::from_ddl(&snapshot.ddl)),
            #[cfg(feature = "postgres")]
            Snapshot::Postgres(snapshot) => Ok(Self::from_ddl(&snapshot.ddl)),
            #[cfg(feature = "mysql")]
            Snapshot::MySQL(snapshot) => Ok(Self::from_ddl(&snapshot.ddl)),
            #[allow(unreachable_patterns)]
            other => Err(crate::SeedError::DialectNotEnabled {
                dialect: format!("{:?}", other.dialect()),
            }),
        }
    }
}
