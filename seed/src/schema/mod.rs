//! Describe tables at runtime, for projects without the drizzle schema
//! macros.
//!
//! With the macros, pass the `#[derive(...Schema)]` struct to
//! [`SeedConfig`](crate::SeedConfig) and skip this module: the macros
//! already record every column type, key and enum, and the config methods
//! then check table and column names at compile time.
//!
//! Without them, build a [`Schema`] that describes the tables that already
//! exist in the database. It records only what seeding needs (types, keys,
//! `NOT NULL`, `UNIQUE`, defaults, enum values), and it does not create
//! tables. Configure it with the `*_by_name` methods of
//! [`SeedConfig`](crate::SeedConfig), which report an unknown name as a
//! [`SeedError`](crate::SeedError) from `try_generate`.
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "sqlite")]
//! # {
//! use drizzle_seed::schema::{Column, Schema, Table};
//! use drizzle_seed::{SeedConfig, generators};
//!
//! let schema = Schema::sqlite()
//!     .table(
//!         Table::new("users")
//!             .column(Column::new("id", "INTEGER").primary_key())
//!             .column(Column::new("email", "TEXT").not_null().unique())
//!             .column(Column::new("plan", "TEXT").not_null().enum_values(["free", "pro"]))
//!             .column(Column::new("age", "INTEGER")),
//!     )
//!     .table(
//!         Table::new("posts")
//!             .column(Column::new("id", "INTEGER").primary_key())
//!             .column(Column::new("user_id", "INTEGER").not_null().references("users", "id"))
//!             .column(Column::new("title", "TEXT").not_null()),
//!     );
//!
//! let tables = SeedConfig::sqlite(&schema)
//!     .seed(42)
//!     .count_by_name("users", 3)
//!     .relation_by_name("users", "posts", 2)
//!     .generator_by_name("users", "age", generators::int(18..=90))
//!     .try_generate_rows()
//!     .expect("valid seed plan");
//!
//! assert_eq!(tables[0].table, "users");
//! assert_eq!(tables[0].rows.len(), 3);
//! assert_eq!(tables[1].table, "posts");
//! assert_eq!(tables[1].rows.len(), 6);
//! # }
//! ```
//!
//! # Memory
//!
//! Seeding reads `'static` table metadata, so the first time a [`Schema`]
//! is seeded its metadata is copied into memory that is never freed. That
//! happens once per `Schema` value, not per call; build the schema once and
//! reuse it.

// Without a dialect feature the module still compiles (so `from_snapshot`
// can report the missing feature), but every dialect match is empty.
#![cfg_attr(
    not(any(feature = "sqlite", feature = "postgres", feature = "mysql")),
    allow(unused_imports, unused_variables, unreachable_code)
)]

use drizzle_core::error::Result as CoreResult;
use drizzle_core::{
    ColumnDialect, ColumnFlags, ColumnRef, ConstraintRef, EnumVariantRef, ForeignKeyRef,
    PrimaryKeyRef, SQLConstraintKind, SQLSchemaImpl, TableDialect, TableRef,
};
use std::sync::OnceLock;

mod check;
mod ddl;

pub use ddl::DdlEntity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dialect {
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "postgres")]
    Postgres,
    #[cfg(feature = "mysql")]
    MySql,
}

/// A set of tables described at runtime, for
/// [`SeedConfig`](crate::SeedConfig).
///
/// Create one per dialect with `Schema::sqlite()`, `Schema::postgres()` or
/// `Schema::mysql()` and add [`Table`]s. See the [module docs](self).
#[derive(Debug)]
pub struct Schema {
    dialect: Dialect,
    tables: Vec<Table>,
    refs: OnceLock<&'static [&'static TableRef]>,
}

impl Clone for Schema {
    fn clone(&self) -> Self {
        Self {
            dialect: self.dialect,
            tables: self.tables.clone(),
            // The copy builds (and leaks) its own metadata if it is seeded.
            refs: OnceLock::new(),
        }
    }
}

impl Schema {
    const fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            tables: Vec::new(),
            refs: OnceLock::new(),
        }
    }

    /// Starts an empty SQLite schema.
    #[cfg(feature = "sqlite")]
    #[must_use]
    pub const fn sqlite() -> Self {
        Self::new(Dialect::Sqlite)
    }

    /// Starts an empty PostgreSQL schema.
    #[cfg(feature = "postgres")]
    #[must_use]
    pub const fn postgres() -> Self {
        Self::new(Dialect::Postgres)
    }

    /// Starts an empty MySQL schema.
    #[cfg(feature = "mysql")]
    #[must_use]
    pub const fn mysql() -> Self {
        Self::new(Dialect::MySql)
    }

    /// Adds `table`.
    ///
    /// # Panics
    ///
    /// Panics if the schema already has a table with the same name (and
    /// namespace), if two of the table's columns share a name, or if its
    /// primary key, `UNIQUE` or foreign key names a column it does not have.
    #[must_use]
    pub fn table(mut self, table: Table) -> Self {
        table.validate();
        assert!(
            !self
                .tables
                .iter()
                .any(|existing| existing.name == table.name && existing.schema == table.schema),
            "schema already has a table named `{}`",
            table.display_name()
        );
        self.tables.push(table);
        self.refs = OnceLock::new();
        self
    }

    /// The tables added so far, in order.
    #[must_use]
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }

    /// Keeps only the tables for which `keep` returns `true`: for example
    /// one `PostgreSQL` schema of an introspected database, which holds
    /// every schema the connection can read.
    ///
    /// A kept table whose foreign key points at a dropped one keeps its
    /// generated values for that key, as with
    /// [`skip`](crate::SeedConfig::skip); point them at existing rows with a
    /// generator, or keep the parent too.
    #[must_use]
    pub fn retain(mut self, mut keep: impl FnMut(&Table) -> bool) -> Self {
        self.tables.retain(|table| keep(table));
        self.refs = OnceLock::new();
        self
    }

    fn build_refs(&self) -> &'static [&'static TableRef] {
        let refs: Vec<&'static TableRef> = self
            .tables
            .iter()
            .map(|table| &*Box::leak(Box::new(table.to_ref(self.dialect))))
            .collect();
        leak_slice(refs)
    }
}

impl SQLSchemaImpl for Schema {
    fn table_refs(&self) -> &'static [&'static TableRef] {
        self.refs.get_or_init(|| self.build_refs())
    }

    /// Always empty: a runtime `Schema` describes tables that already
    /// exist.
    fn create_statements(&self) -> CoreResult<impl Iterator<Item = String>> {
        Ok(std::iter::empty())
    }
}

/// One table of a runtime [`Schema`].
#[derive(Clone, Debug)]
pub struct Table {
    schema: Option<String>,
    name: String,
    columns: Vec<Column>,
    primary_key: Option<Vec<String>>,
    unique: Vec<Vec<String>>,
    foreign_keys: Vec<ForeignKey>,
}

#[derive(Clone, Debug)]
struct ForeignKey {
    columns: Vec<String>,
    target_schema: Option<String>,
    target_table: String,
    target_columns: Vec<String>,
}

impl Table {
    /// Starts a table named `name`, with no columns.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: name.into(),
            columns: Vec::new(),
            primary_key: None,
            unique: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    /// Puts the table in a namespace: a `PostgreSQL` schema or a MySQL
    /// database. INSERTs then name it `schema.table` (except `public` on
    /// `PostgreSQL`, which is left to `search_path`).
    #[must_use]
    pub fn in_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Adds a column.
    #[must_use]
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    /// Sets a primary key over several columns. For a single column, use
    /// [`Column::primary_key`].
    #[must_use]
    pub fn primary_key<I>(mut self, columns: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.primary_key = Some(columns.into_iter().map(Into::into).collect());
        self
    }

    /// Adds a `UNIQUE` constraint over several columns. For a single
    /// column, use [`Column::unique`].
    #[must_use]
    pub fn unique<I>(mut self, columns: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.unique
            .push(columns.into_iter().map(Into::into).collect());
        self
    }

    /// Adds a foreign key from `columns` to `target_columns` of
    /// `target_table`. For a single column, use [`Column::references`].
    ///
    /// `target_table` may be qualified as `schema.table`; otherwise it is
    /// looked up in this table's namespace.
    #[must_use]
    pub fn foreign_key<I, J>(
        mut self,
        columns: I,
        target_table: impl Into<String>,
        target_columns: J,
    ) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
        J: IntoIterator,
        J::Item: Into<String>,
    {
        let (target_schema, target_table) = split_qualified(target_table.into());
        self.foreign_keys.push(ForeignKey {
            columns: columns.into_iter().map(Into::into).collect(),
            target_schema,
            target_table,
            target_columns: target_columns.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// The table name, without its namespace.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The table's namespace (`PostgreSQL` schema or MySQL database), if
    /// it has one.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// The columns, in order.
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    fn display_name(&self) -> String {
        match &self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        }
    }

    fn validate(&self) {
        for (index, column) in self.columns.iter().enumerate() {
            assert!(
                !self.columns[..index]
                    .iter()
                    .any(|earlier| earlier.name == column.name),
                "table `{}` has two columns named `{}`",
                self.display_name(),
                column.name
            );
        }
        let has = |name: &String| self.columns.iter().any(|column| &column.name == name);
        let constraint_columns = self
            .primary_key
            .iter()
            .flatten()
            .chain(self.unique.iter().flatten())
            .chain(self.foreign_keys.iter().flat_map(|fk| &fk.columns));
        for name in constraint_columns {
            assert!(
                has(name),
                "a key of table `{}` names column `{name}`, which the table does not have",
                self.display_name()
            );
        }
        for fk in &self.foreign_keys {
            assert!(
                !fk.columns.is_empty() && fk.columns.len() == fk.target_columns.len(),
                "a foreign key of table `{}` needs as many target columns as columns",
                self.display_name()
            );
        }
    }

    fn primary_key_columns(&self) -> Vec<&str> {
        self.primary_key.as_ref().map_or_else(
            || {
                self.columns
                    .iter()
                    .filter(|column| column.primary_key)
                    .map(|column| column.name.as_str())
                    .collect()
            },
            |columns| columns.iter().map(String::as_str).collect(),
        )
    }

    fn to_ref(&self, dialect: Dialect) -> TableRef {
        let name = leak_str(&self.name);
        let schema = self.schema.as_deref().map(leak_str);
        let primary_key = self.primary_key_columns();

        let columns: Vec<ColumnRef> = self
            .columns
            .iter()
            .map(|column| column.to_ref(name, primary_key.contains(&column.name.as_str()), dialect))
            .collect();
        let column_names: Vec<&'static str> = columns.iter().map(|column| column.name).collect();

        let mut foreign_keys: Vec<ForeignKey> = self
            .columns
            .iter()
            .filter_map(|column| {
                column.references.as_ref().map(|(table, target)| {
                    let (target_schema, target_table) = split_qualified(table.clone());
                    ForeignKey {
                        columns: vec![column.name.clone()],
                        target_schema,
                        target_table,
                        target_columns: vec![target.clone()],
                    }
                })
            })
            .collect();
        foreign_keys.extend(self.foreign_keys.iter().cloned());

        let mut dependency_names: Vec<&'static str> = Vec::new();
        let foreign_keys: Vec<ForeignKeyRef> = foreign_keys
            .iter()
            .map(|fk| {
                let target_table = leak_str(&fk.target_table);
                if target_table != name && !dependency_names.contains(&target_table) {
                    dependency_names.push(target_table);
                }
                ForeignKeyRef {
                    name: leak_str(&format!("{}_{}_fk", self.name, fk.columns.join("_"))),
                    name_explicit: false,
                    target_table,
                    target_schema: fk.target_schema.as_deref().map_or("", leak_str),
                    source_columns: leak_strs(&fk.columns),
                    target_columns: leak_strs(&fk.target_columns),
                    on_delete: None,
                    on_update: None,
                    deferrable: false,
                    initially_deferred: false,
                }
            })
            .collect();

        let constraints: Vec<ConstraintRef> = self
            .unique
            .iter()
            .map(|columns| ConstraintRef {
                name: None,
                name_explicit: false,
                kind: SQLConstraintKind::Unique,
                columns: leak_strs(columns),
                check_expression: None,
                deferrable: false,
                initially_deferred: false,
            })
            .collect();

        TableRef {
            name,
            column_names: leak_slice(column_names),
            schema,
            qualified_name: schema.map_or(name, |schema| leak_str(&format!("{schema}.{name}"))),
            columns: leak_slice(columns),
            primary_key: (!primary_key.is_empty()).then(|| PrimaryKeyRef {
                columns: leak_slice(primary_key.into_iter().map(leak_str).collect()),
            }),
            foreign_keys: leak_slice(foreign_keys),
            constraints: leak_slice(constraints),
            dependency_names: leak_slice(dependency_names),
            dialect: table_dialect(dialect),
        }
    }
}

/// One column of a runtime [`Table`].
///
/// Columns are nullable unless marked [`not_null`](Self::not_null) or
/// [`primary_key`](Self::primary_key), as in SQL.
#[derive(Clone, Debug)]
pub struct Column {
    name: String,
    sql_type: String,
    not_null: bool,
    primary_key: bool,
    unique: bool,
    has_default: bool,
    auto_increment: bool,
    identity_always: bool,
    generated: Option<String>,
    references: Option<(String, String)>,
    enum_values: Option<Vec<String>>,
    /// A `PostgreSQL` column whose default draws from a sequence
    /// (`nextval(...)`), as introspection reports a `SERIAL`.
    sequence_default: bool,
}

impl Column {
    /// Starts a nullable column named `name` with the SQL type `sql_type`,
    /// written as in `CREATE TABLE`: `"INTEGER"`, `"VARCHAR(32)"`,
    /// `"jsonb"`, `"text[]"`, `"ENUM('a','b')"`, ...
    #[must_use]
    pub fn new(name: impl Into<String>, sql_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql_type: sql_type.into(),
            not_null: false,
            primary_key: false,
            unique: false,
            has_default: false,
            auto_increment: false,
            identity_always: false,
            generated: None,
            references: None,
            enum_values: None,
            sequence_default: false,
        }
    }

    /// Marks the column `NOT NULL`.
    #[must_use]
    pub const fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Makes the column the primary key (which implies `NOT NULL`). For a
    /// key over several columns, use [`Table::primary_key`].
    #[must_use]
    pub const fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.not_null = true;
        self
    }

    /// Marks the column `UNIQUE`: seeded values are distinct.
    #[must_use]
    pub const fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Records that the column has a `DEFAULT`. Seeding writes `DEFAULT`
    /// for it (unless it is the primary key, or has its own generator), so
    /// the database fills it in.
    #[must_use]
    pub const fn has_default(mut self) -> Self {
        self.has_default = true;
        self
    }

    /// Records an auto-numbered column: `AUTOINCREMENT` on SQLite,
    /// `AUTO_INCREMENT` on MySQL, `GENERATED BY DEFAULT AS IDENTITY` on
    /// `PostgreSQL`. (`SERIAL` types are recognized from the type name.)
    ///
    /// A primary key still gets explicit ids 1, 2, 3, ... so children can
    /// reference them; on `PostgreSQL` the sequence is then moved past
    /// them. Any other such column is left to the database.
    #[must_use]
    pub const fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }

    /// Records a `PostgreSQL` `GENERATED ALWAYS AS IDENTITY` column; INSERTs
    /// that give it a value say `OVERRIDING SYSTEM VALUE`. On other
    /// dialects, the same as [`auto_increment`](Self::auto_increment).
    #[must_use]
    pub const fn identity_always(mut self) -> Self {
        self.auto_increment = true;
        self.identity_always = true;
        self
    }

    /// Records a generated (computed) column, `GENERATED ALWAYS AS
    /// (expression)`. Seeding leaves it out of INSERTs.
    #[must_use]
    pub fn generated(mut self, expression: impl Into<String>) -> Self {
        self.generated = Some(expression.into());
        self
    }

    /// Adds a foreign key from this column to `column` of `table`. Seeded
    /// values then point at seeded rows of `table`.
    ///
    /// `table` may be qualified as `schema.table`; otherwise it is looked up
    /// in this table's namespace.
    #[must_use]
    pub fn references(mut self, table: impl Into<String>, column: impl Into<String>) -> Self {
        self.references = Some((table.into(), column.into()));
        self
    }

    /// Restricts the column to `values`, as for an enum: seeding picks one
    /// of them. On an integer column, the value's position (0, 1, 2, ...)
    /// is stored instead.
    ///
    /// A MySQL `ENUM('a','b')` or `SET(...)` type already declares its
    /// values and needs no call. A PostgreSQL enum type is named as in
    /// `CREATE TABLE`, so a mixed-case one is quoted:
    /// `Column::new("mood", "\"Mood\"")`, as the derive and migrations
    /// create it.
    #[must_use]
    pub fn enum_values<I>(mut self, values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.enum_values = Some(values.into_iter().map(Into::into).collect());
        self
    }

    /// The column name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn to_ref(&self, table: &'static str, in_primary_key: bool, dialect: Dialect) -> ColumnRef {
        let mut flags = ColumnFlags::empty();
        if self.not_null || in_primary_key {
            flags |= ColumnFlags::NOT_NULL;
        }
        if in_primary_key {
            flags |= ColumnFlags::PRIMARY_KEY;
        }
        if self.unique {
            flags |= ColumnFlags::UNIQUE;
        }
        if self.has_default
            || self.sequence_default
            || (dialect_is_postgres(dialect) && is_serial_type(&self.sql_type))
        {
            flags |= ColumnFlags::HAS_DEFAULT;
        }

        let generated = self.generated.as_deref().map(leak_str);
        let enum_variants: Option<&'static [EnumVariantRef]> =
            self.enum_values.as_ref().map(|values| {
                leak_slice(
                    values
                        .iter()
                        .zip(0..)
                        .map(|(label, discriminant)| EnumVariantRef {
                            label: leak_str(label),
                            discriminant,
                        })
                        .collect(),
                )
            });

        let (sql_type, dialect) = match dialect {
            #[cfg(feature = "sqlite")]
            Dialect::Sqlite => (
                leak_str(&self.sql_type),
                ColumnDialect::SQLite {
                    autoincrement: self.auto_increment,
                    default: None,
                    generated_expression: generated,
                    generated_stored: false,
                    collate: None,
                    enum_variants,
                },
            ),
            #[cfg(feature = "postgres")]
            Dialect::Postgres => {
                // `text[]` is stored as its element type plus a depth, as
                // the table macros do.
                let mut element = self.sql_type.trim();
                let mut dimensions = 0;
                while let Some(inner) = element.strip_suffix("[]") {
                    element = inner.trim_end();
                    dimensions += 1;
                }
                let upper = element.to_ascii_uppercase();
                let element = leak_str(element);
                (
                    element,
                    ColumnDialect::PostgreSQL {
                        postgres_type: element,
                        dimensions: (dimensions > 0).then_some(dimensions),
                        is_serial: (is_serial_type(&upper) && upper != "BIGSERIAL")
                            || self.sequence_default,
                        is_bigserial: upper == "BIGSERIAL",
                        is_generated_identity: self.auto_increment,
                        is_identity_always: self.identity_always,
                        default: None,
                        generated_expression: generated,
                        generated_stored: generated.is_some(),
                        collate: None,
                        comment: None,
                        enum_variants,
                    },
                )
            }
            #[cfg(feature = "mysql")]
            Dialect::MySql => {
                // MySQL declares enum values in the type itself; write them
                // there so the MySQL rules pick and check them.
                let sql_type = match &self.enum_values {
                    Some(values) if !is_mysql_enum_type(&self.sql_type) => {
                        let labels: Vec<String> = values
                            .iter()
                            .map(|value| format!("'{}'", value.replace('\'', "''")))
                            .collect();
                        format!("ENUM({})", labels.join(","))
                    }
                    _ => self.sql_type.clone(),
                };
                let _ = enum_variants;
                (
                    leak_str(&sql_type),
                    ColumnDialect::MySQL {
                        auto_increment: self.auto_increment,
                        default: None,
                        generated_expression: generated,
                        generated_stored: false,
                        charset: None,
                        collate: None,
                        on_update: None,
                    },
                )
            }
        };

        ColumnRef {
            table,
            name: leak_str(&self.name),
            sql_type,
            flags,
            dialect,
        }
    }
}

/// `SERIAL`, `SMALLSERIAL` or `BIGSERIAL` (and their `SERIAL4`/`SERIAL2`/
/// `SERIAL8` spellings): an integer with a sequence default.
fn is_serial_type(sql_type: &str) -> bool {
    matches!(
        sql_type.trim().to_ascii_uppercase().as_str(),
        "SERIAL" | "SMALLSERIAL" | "BIGSERIAL" | "SERIAL2" | "SERIAL4" | "SERIAL8"
    )
}

const fn dialect_is_postgres(dialect: Dialect) -> bool {
    #[cfg(feature = "postgres")]
    {
        matches!(dialect, Dialect::Postgres)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = dialect;
        false
    }
}

#[cfg(feature = "mysql")]
fn is_mysql_enum_type(sql_type: &str) -> bool {
    let upper = sql_type.trim_start().to_ascii_uppercase();
    upper.starts_with("ENUM") || upper.starts_with("SET")
}

const fn table_dialect(dialect: Dialect) -> TableDialect {
    match dialect {
        #[cfg(feature = "sqlite")]
        Dialect::Sqlite => TableDialect::SQLite {
            without_rowid: false,
            strict: false,
        },
        #[cfg(feature = "postgres")]
        Dialect::Postgres => TableDialect::PostgreSQL {
            is_unlogged: false,
            is_temporary: false,
            inherits: None,
            tablespace: None,
            is_rls_enabled: false,
            comment: None,
        },
        #[cfg(feature = "mysql")]
        Dialect::MySql => TableDialect::MySQL {
            is_temporary: false,
            engine: None,
            charset: None,
            collate: None,
            comment: None,
        },
    }
}

/// `"auth.users"` → `(Some("auth"), "users")`; `"users"` → `(None, "users")`.
fn split_qualified(name: String) -> (Option<String>, String) {
    match name.split_once('.') {
        Some((schema, table)) => (Some(schema.to_string()), table.to_string()),
        None => (None, name),
    }
}

fn leak_str(value: &str) -> &'static str {
    Box::leak(value.to_owned().into_boxed_str())
}

fn leak_strs(values: &[String]) -> &'static [&'static str] {
    leak_slice(values.iter().map(|value| leak_str(value)).collect())
}

fn leak_slice<T>(values: Vec<T>) -> &'static [T] {
    Box::leak(values.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "postgres")]
    #[test]
    fn postgres_types_record_arrays_serials_and_identities() {
        let schema = Schema::postgres().table(
            Table::new("items")
                .in_schema("app")
                .column(Column::new("id", "BIGSERIAL").primary_key())
                .column(Column::new("seq", "serial"))
                .column(Column::new("code", "INTEGER").identity_always())
                .column(Column::new("tags", "text[][]")),
        );
        let [table] = schema.table_refs() else {
            panic!("one table expected");
        };
        assert_eq!(table.qualified_name, "app.items");
        let column = |name: &str| table.columns.iter().find(|c| c.name == name).unwrap();

        assert!(matches!(
            column("id").dialect,
            ColumnDialect::PostgreSQL {
                is_bigserial: true,
                ..
            }
        ));
        assert!(column("seq").has_default() && !column("seq").not_null());
        assert!(matches!(
            column("code").dialect,
            ColumnDialect::PostgreSQL {
                is_generated_identity: true,
                is_identity_always: true,
                ..
            }
        ));
        assert_eq!(column("tags").sql_type, "text");
        assert!(matches!(
            column("tags").dialect,
            ColumnDialect::PostgreSQL {
                dimensions: Some(2),
                ..
            }
        ));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn keys_and_references_become_table_metadata() {
        let schema = Schema::sqlite()
            .table(Table::new("users").column(Column::new("id", "INTEGER").primary_key()))
            .table(
                Table::new("follows")
                    .column(Column::new("follower", "INTEGER").references("users", "id"))
                    .column(Column::new("followed", "INTEGER"))
                    .primary_key(["follower", "followed"])
                    .foreign_key(["followed"], "users", ["id"])
                    .unique(["followed", "follower"]),
            );
        let follows = schema.table_refs()[1];
        assert_eq!(
            follows.primary_key.map(|pk| pk.columns),
            Some(&["follower", "followed"][..])
        );
        assert!(
            follows
                .columns
                .iter()
                .all(|c| c.primary_key() && c.not_null())
        );
        assert_eq!(follows.foreign_keys.len(), 2);
        assert_eq!(follows.dependency_names, ["users"]);
        assert_eq!(follows.constraints[0].columns, ["followed", "follower"]);

        // Metadata is built once per schema value.
        assert!(std::ptr::eq(schema.table_refs(), schema.table_refs()));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    #[should_panic(expected = "names column `missing`")]
    fn keys_must_name_existing_columns() {
        let _ = Schema::sqlite().table(
            Table::new("t")
                .column(Column::new("id", "INTEGER"))
                .primary_key(["id", "missing"]),
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    #[should_panic(expected = "already has a table named `t`")]
    fn table_names_are_unique() {
        let _ = Schema::sqlite()
            .table(Table::new("t").column(Column::new("id", "INTEGER")))
            .table(Table::new("t").column(Column::new("id", "INTEGER")));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_enum_values_are_written_into_the_type() {
        let schema = Schema::mysql().table(
            Table::new("t")
                .column(Column::new("status", "VARCHAR(8)").enum_values(["on", "it's"]))
                .column(Column::new("kind", "ENUM('a','b')").enum_values(["x"])),
        );
        let columns = schema.table_refs()[0].columns;
        assert_eq!(columns[0].sql_type, "ENUM('on','it''s')");
        assert_eq!(columns[1].sql_type, "ENUM('a','b')");
    }
}
