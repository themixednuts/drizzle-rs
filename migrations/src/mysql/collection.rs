//! Typed MySQL DDL collection and single-database-scope validation.

use super::ddl::{
    CheckConstraint, Column, ForeignKey, Index, MySQLEntity, PrimaryKey, Table, UniqueConstraint,
    View, ViewAlgorithm, ViewSqlSecurity,
};
use crate::collection::EntityCollection;
use std::borrow::Cow;
use std::collections::HashSet;

impl EntityCollection<Table> {
    #[must_use]
    pub fn one(&self, database: Option<&str>, name: &str) -> Option<&Table> {
        self.entities
            .iter()
            .find(|entity| entity.database.as_deref() == database && entity.name == name)
    }
}

macro_rules! table_children {
    ($ty:ty) => {
        impl EntityCollection<$ty> {
            #[must_use]
            pub fn one(&self, database: Option<&str>, table: &str, name: &str) -> Option<&$ty> {
                self.entities.iter().find(|entity| {
                    entity.database.as_deref() == database
                        && entity.table == table
                        && entity.name == name
                })
            }

            #[must_use]
            pub fn for_table(&self, database: Option<&str>, table: &str) -> Vec<&$ty> {
                self.entities
                    .iter()
                    .filter(|entity| {
                        entity.database.as_deref() == database && entity.table == table
                    })
                    .collect()
            }
        }
    };
}

table_children!(Column);
table_children!(Index);
table_children!(UniqueConstraint);
table_children!(ForeignKey);
table_children!(CheckConstraint);

fn validate_children<'a, T>(
    entities: &'a EntityCollection<T>,
    kind: &'static str,
    table_keys: &HashSet<(Option<&'a str>, &'a str)>,
    fields: impl Fn(&'a T) -> (Option<&'a str>, &'a str, &'a str),
) -> Result<(), ValidationError> {
    let mut keys = HashSet::new();
    for entity in entities.list() {
        let (database, table, name) = fields(entity);
        if !table_keys.contains(&(database, table)) {
            return Err(ValidationError::MissingTable {
                kind,
                name: name.to_string(),
                table: table.to_string(),
            });
        }
        if !keys.insert((database, table, name)) {
            return Err(ValidationError::Duplicate {
                kind,
                name: name.to_string(),
            });
        }
    }
    Ok(())
}

impl EntityCollection<PrimaryKey> {
    #[must_use]
    pub fn for_table(&self, database: Option<&str>, table: &str) -> Option<&PrimaryKey> {
        self.entities
            .iter()
            .find(|entity| entity.database.as_deref() == database && entity.table == table)
    }
}

impl EntityCollection<View> {
    #[must_use]
    pub fn one(&self, database: Option<&str>, name: &str) -> Option<&View> {
        self.entities
            .iter()
            .find(|entity| entity.database.as_deref() == database && entity.name == name)
    }
}

/// Structured MySQL entities in deterministic category order.
#[derive(Clone, Debug, Default)]
pub struct MySQLDDL {
    pub tables: EntityCollection<Table>,
    pub columns: EntityCollection<Column>,
    pub indexes: EntityCollection<Index>,
    pub pks: EntityCollection<PrimaryKey>,
    pub uniques: EntityCollection<UniqueConstraint>,
    pub fks: EntityCollection<ForeignKey>,
    pub checks: EntityCollection<CheckConstraint>,
    pub views: EntityCollection<View>,
}

/// Invalid MySQL snapshot structure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("MySQL migration snapshot contains multiple database scopes: `{first}` and `{second}`")]
    MultipleDatabases { first: String, second: String },
    #[error(
        "MySQL migration snapshot mixes qualified and unqualified entities in selected database `{database}`"
    )]
    MixedDatabaseQualification { database: String },
    #[error("duplicate MySQL {kind} entity `{name}`")]
    Duplicate { kind: &'static str, name: String },
    #[error("MySQL {kind} `{name}` belongs to missing table `{table}`")]
    MissingTable {
        kind: &'static str,
        name: String,
        table: String,
    },
    #[error("MySQL foreign key `{name}` references another database")]
    CrossDatabaseForeignKey { name: String },
    #[error("MySQL foreign key `{name}` references missing table `{table}`")]
    MissingReferencedTable { name: String, table: String },
    #[error("MySQL {kind} `{name}` must reference at least one column")]
    EmptyColumns { kind: &'static str, name: String },
    #[error("MySQL table `{table}` must contain at least one column")]
    EmptyTable { table: String },
    #[error("MySQL {field} for `{name}` cannot be empty")]
    EmptySql { field: &'static str, name: String },
    #[error(
        "MySQL foreign key `{name}` has {local} local columns but {foreign} referenced columns"
    )]
    MismatchedForeignKeyColumns {
        name: String,
        local: usize,
        foreign: usize,
    },
    #[error("MySQL {kind} `{name}` references missing column `{table}.{column}`")]
    MissingColumn {
        kind: &'static str,
        name: String,
        table: String,
        column: String,
    },
}

impl MySQLDDL {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_entities(entities: Vec<MySQLEntity>) -> Self {
        let mut ddl = Self::new();
        for entity in entities {
            ddl.push_entity(entity);
        }
        ddl
    }

    pub(crate) fn set_database(&mut self, database: Option<Cow<'static, str>>) {
        for table in self.tables.list_mut() {
            table.database = database.clone();
        }
        for column in self.columns.list_mut() {
            column.database = database.clone();
        }
        for index in self.indexes.list_mut() {
            index.database = database.clone();
        }
        for primary_key in self.pks.list_mut() {
            primary_key.database = database.clone();
        }
        for unique in self.uniques.list_mut() {
            unique.database = database.clone();
        }
        for foreign_key in self.fks.list_mut() {
            foreign_key.database = database.clone();
            foreign_key.foreign_database = database.clone();
        }
        for check in self.checks.list_mut() {
            check.database = database.clone();
        }
        for view in self.views.list_mut() {
            view.database = database.clone();
        }
    }

    /// Build and validate a complete MySQL entity graph.
    pub fn try_from_entities(entities: Vec<MySQLEntity>) -> Result<Self, ValidationError> {
        let mut ddl = Self::from_entities(entities);
        ddl.resolve_database_scope()?;
        ddl.validate()?;
        Ok(ddl)
    }

    /// Resolve unqualified entities into the one explicitly selected database.
    ///
    /// A schema may omit the database on some declarations for ergonomic
    /// parity with `MySQLTable`; once any declaration selects a database, the
    /// complete snapshot is qualified to that same scope. This prevents a
    /// migration from accidentally mixing `app.table` with a connection's
    /// unrelated default database.
    fn resolve_database_scope(&mut self) -> Result<(), ValidationError> {
        for view in self.views.list_mut() {
            view.algorithm.get_or_insert(ViewAlgorithm::Undefined);
            view.sql_security.get_or_insert(ViewSqlSecurity::Definer);
        }
        let Some(database) = self.database_scope()? else {
            return Ok(());
        };
        let database: Cow<'static, str> = Cow::Owned(database);
        for table in self.tables.list_mut() {
            table.database.get_or_insert_with(|| database.clone());
        }
        for column in self.columns.list_mut() {
            column.database.get_or_insert_with(|| database.clone());
        }
        for index in self.indexes.list_mut() {
            index.database.get_or_insert_with(|| database.clone());
        }
        for primary_key in self.pks.list_mut() {
            primary_key.database.get_or_insert_with(|| database.clone());
        }
        for unique in self.uniques.list_mut() {
            unique.database.get_or_insert_with(|| database.clone());
        }
        for foreign_key in self.fks.list_mut() {
            foreign_key.database.get_or_insert_with(|| database.clone());
            foreign_key
                .foreign_database
                .get_or_insert_with(|| database.clone());
        }
        for check in self.checks.list_mut() {
            check.database.get_or_insert_with(|| database.clone());
        }
        for view in self.views.list_mut() {
            view.database.get_or_insert_with(|| database.clone());
        }
        Ok(())
    }

    pub fn push_entity(&mut self, entity: MySQLEntity) {
        match entity {
            MySQLEntity::Table(entity) => self.tables.push(entity),
            MySQLEntity::Column(entity) => self.columns.push(entity),
            MySQLEntity::Index(entity) => self.indexes.push(entity),
            MySQLEntity::PrimaryKey(entity) => self.pks.push(entity),
            MySQLEntity::UniqueConstraint(entity) => self.uniques.push(entity),
            MySQLEntity::ForeignKey(entity) => self.fks.push(entity),
            MySQLEntity::CheckConstraint(entity) => self.checks.push(entity),
            MySQLEntity::View(entity) => self.views.push(entity),
        }
    }

    /// Serialize entities in a stable dependency-oriented category order.
    #[must_use]
    pub fn to_entities(&self) -> Vec<MySQLEntity> {
        self.tables
            .list()
            .iter()
            .cloned()
            .map(MySQLEntity::Table)
            .chain(self.columns.list().iter().cloned().map(MySQLEntity::Column))
            .chain(self.pks.list().iter().cloned().map(MySQLEntity::PrimaryKey))
            .chain(
                self.uniques
                    .list()
                    .iter()
                    .cloned()
                    .map(MySQLEntity::UniqueConstraint),
            )
            .chain(self.indexes.list().iter().cloned().map(MySQLEntity::Index))
            .chain(self.fks.list().iter().cloned().map(MySQLEntity::ForeignKey))
            .chain(
                self.checks
                    .list()
                    .iter()
                    .cloned()
                    .map(MySQLEntity::CheckConstraint),
            )
            .chain(self.views.list().iter().cloned().map(MySQLEntity::View))
            .collect()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tables.is_empty()
            && self.columns.is_empty()
            && self.indexes.is_empty()
            && self.pks.is_empty()
            && self.uniques.is_empty()
            && self.fks.is_empty()
            && self.checks.is_empty()
            && self.views.is_empty()
    }

    /// The one explicit database named by this snapshot, if any.
    pub fn database_scope(&self) -> Result<Option<String>, ValidationError> {
        let mut scope: Option<String> = None;
        let mut observe = |database: Option<&str>| -> Result<(), ValidationError> {
            let Some(database) = database else {
                return Ok(());
            };
            match scope.as_deref() {
                None => scope = Some(database.to_string()),
                Some(existing) if existing == database => {}
                Some(existing) => {
                    return Err(ValidationError::MultipleDatabases {
                        first: existing.to_string(),
                        second: database.to_string(),
                    });
                }
            }
            Ok(())
        };
        for entity in self.to_entities() {
            observe(entity.database())?;
            if let MySQLEntity::ForeignKey(foreign_key) = entity {
                observe(foreign_key.foreign_database.as_deref())?;
            }
        }
        Ok(scope)
    }

    pub fn validate_database_scope(&self) -> Result<(), ValidationError> {
        let scope = self.database_scope()?;
        if let Some(database) = scope {
            let has_unqualified = self
                .to_entities()
                .iter()
                .any(|entity| entity.database().is_none())
                || self
                    .fks
                    .list()
                    .iter()
                    .any(|foreign_key| foreign_key.foreign_database.is_none());
            if has_unqualified {
                return Err(ValidationError::MixedDatabaseQualification { database });
            }
        }
        Ok(())
    }

    /// Validate identity, parentage, and one-database scope.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.validate_database_scope()?;
        let table_keys: HashSet<_> = self
            .tables
            .list()
            .iter()
            .map(|table| (table.database.as_deref(), table.name.as_ref()))
            .collect();
        if table_keys.len() != self.tables.len() {
            return Err(ValidationError::Duplicate {
                kind: "table",
                name: "<same database and name>".to_string(),
            });
        }

        let mut view_keys = HashSet::new();
        for view in self.views.list() {
            let key = (view.database.as_deref(), view.name.as_ref());
            if table_keys.contains(&key) {
                return Err(ValidationError::Duplicate {
                    kind: "table/view",
                    name: view.name.to_string(),
                });
            }
            if !view_keys.insert(key) {
                return Err(ValidationError::Duplicate {
                    kind: "view",
                    name: view.name.to_string(),
                });
            }
            if !view.is_existing
                && view
                    .definition
                    .as_deref()
                    .is_none_or(|definition| definition.trim().is_empty())
            {
                return Err(ValidationError::EmptySql {
                    field: "view definition",
                    name: view.name.to_string(),
                });
            }
        }

        validate_children(&self.columns, "column", &table_keys, |column| {
            (
                column.database.as_deref(),
                column.table.as_ref(),
                column.name.as_ref(),
            )
        })?;
        validate_children(&self.indexes, "index", &table_keys, |index| {
            (
                index.database.as_deref(),
                index.table.as_ref(),
                index.name.as_ref(),
            )
        })?;
        validate_children(&self.uniques, "unique constraint", &table_keys, |unique| {
            (
                unique.database.as_deref(),
                unique.table.as_ref(),
                unique.name.as_ref(),
            )
        })?;
        validate_children(&self.checks, "check constraint", &table_keys, |check| {
            (
                check.database.as_deref(),
                check.table.as_ref(),
                check.name.as_ref(),
            )
        })?;
        validate_children(&self.fks, "foreign key", &table_keys, |foreign_key| {
            (
                foreign_key.database.as_deref(),
                foreign_key.table.as_ref(),
                foreign_key.name.as_ref(),
            )
        })?;

        let column_keys: HashSet<_> = self
            .columns
            .list()
            .iter()
            .map(|column| {
                (
                    column.database.as_deref(),
                    column.table.as_ref(),
                    column.name.as_ref(),
                )
            })
            .collect();
        for table in self.tables.list() {
            if !column_keys.iter().any(|(database, table_name, _)| {
                *database == table.database.as_deref() && *table_name == table.name.as_ref()
            }) {
                return Err(ValidationError::EmptyTable {
                    table: table.name.to_string(),
                });
            }
        }
        for column in self.columns.list() {
            let name = format!("{}.{}", column.table, column.name);
            if column.sql_type.trim().is_empty() {
                return Err(ValidationError::EmptySql {
                    field: "column type",
                    name,
                });
            }
            if column
                .generated
                .as_ref()
                .is_some_and(|generated| generated.expression.trim().is_empty())
            {
                return Err(ValidationError::EmptySql {
                    field: "generated expression",
                    name,
                });
            }
        }
        let require_columns = |kind: &'static str,
                               name: &str,
                               database: Option<&str>,
                               table: &str,
                               columns: &[Cow<'static, str>]| {
            if columns.is_empty() {
                return Err(ValidationError::EmptyColumns {
                    kind,
                    name: name.to_string(),
                });
            }
            for column in columns {
                if !column_keys.contains(&(database, table, column.as_ref())) {
                    return Err(ValidationError::MissingColumn {
                        kind,
                        name: name.to_string(),
                        table: table.to_string(),
                        column: column.to_string(),
                    });
                }
            }
            Ok(())
        };

        for index in self.indexes.list() {
            if index.columns.is_empty() {
                return Err(ValidationError::EmptyColumns {
                    kind: "index",
                    name: index.name.to_string(),
                });
            }
            if let Some(column) = index
                .columns
                .iter()
                .find(|column| column.expression.trim().is_empty())
            {
                return Err(ValidationError::EmptySql {
                    field: if column.is_expression {
                        "index expression"
                    } else {
                        "index column"
                    },
                    name: index.name.to_string(),
                });
            }
            for column in index.columns.iter().filter(|column| !column.is_expression) {
                if !column_keys.contains(&(
                    index.database.as_deref(),
                    index.table.as_ref(),
                    column.expression.as_ref(),
                )) {
                    return Err(ValidationError::MissingColumn {
                        kind: "index",
                        name: index.name.to_string(),
                        table: index.table.to_string(),
                        column: column.expression.to_string(),
                    });
                }
            }
        }

        for unique in self.uniques.list() {
            require_columns(
                "unique constraint",
                unique.name.as_ref(),
                unique.database.as_deref(),
                unique.table.as_ref(),
                &unique.columns,
            )?;
        }

        // MySQL constraint symbols are scoped to the selected database, not
        // to an individual table. Generated names include the table, but
        // explicit names must still be rejected before the server does.
        let mut constraint_symbols = HashSet::new();
        for (database, name) in self
            .fks
            .list()
            .iter()
            .map(|foreign_key| (foreign_key.database.as_deref(), foreign_key.name.as_ref()))
            .chain(
                self.checks
                    .list()
                    .iter()
                    .map(|check| (check.database.as_deref(), check.name.as_ref())),
            )
        {
            if !constraint_symbols.insert((database, name)) {
                return Err(ValidationError::Duplicate {
                    kind: "constraint",
                    name: name.to_string(),
                });
            }
        }

        for check in self.checks.list() {
            if check.expression.trim().is_empty() {
                return Err(ValidationError::EmptySql {
                    field: "check expression",
                    name: check.name.to_string(),
                });
            }
        }

        let mut pk_tables = HashSet::new();
        for primary_key in self.pks.list() {
            if !table_keys.contains(&(primary_key.database.as_deref(), primary_key.table.as_ref()))
            {
                return Err(ValidationError::MissingTable {
                    kind: "primary key",
                    name: primary_key.name.as_deref().unwrap_or("PRIMARY").to_string(),
                    table: primary_key.table.to_string(),
                });
            }
            if !pk_tables.insert((primary_key.database.as_deref(), primary_key.table.as_ref())) {
                return Err(ValidationError::Duplicate {
                    kind: "primary key",
                    name: primary_key.table.to_string(),
                });
            }
            require_columns(
                "primary key",
                primary_key.name.as_deref().unwrap_or("PRIMARY"),
                primary_key.database.as_deref(),
                primary_key.table.as_ref(),
                &primary_key.columns,
            )?;
        }

        for foreign_key in self.fks.list() {
            if foreign_key.columns.len() != foreign_key.foreign_columns.len() {
                return Err(ValidationError::MismatchedForeignKeyColumns {
                    name: foreign_key.name.to_string(),
                    local: foreign_key.columns.len(),
                    foreign: foreign_key.foreign_columns.len(),
                });
            }
            require_columns(
                "foreign key",
                foreign_key.name.as_ref(),
                foreign_key.database.as_deref(),
                foreign_key.table.as_ref(),
                &foreign_key.columns,
            )?;
            let foreign_database = foreign_key
                .foreign_database
                .as_deref()
                .or(foreign_key.database.as_deref());
            if foreign_database != foreign_key.database.as_deref() {
                return Err(ValidationError::CrossDatabaseForeignKey {
                    name: foreign_key.name.to_string(),
                });
            }
            if !table_keys.contains(&(foreign_database, foreign_key.foreign_table.as_ref())) {
                return Err(ValidationError::MissingReferencedTable {
                    name: foreign_key.name.to_string(),
                    table: foreign_key.foreign_table.to_string(),
                });
            }
            require_columns(
                "foreign key",
                foreign_key.name.as_ref(),
                foreign_database,
                foreign_key.foreign_table.as_ref(),
                &foreign_key.foreign_columns,
            )?;
        }
        Ok(())
    }
}

/// Entity sets attached to one table.
pub struct TableEntities<'a> {
    pub columns: Vec<&'a Column>,
    pub indexes: Vec<&'a Index>,
    pub primary_key: Option<&'a PrimaryKey>,
    pub uniques: Vec<&'a UniqueConstraint>,
    pub foreign_keys: Vec<&'a ForeignKey>,
    pub checks: Vec<&'a CheckConstraint>,
}

impl MySQLDDL {
    #[must_use]
    pub fn table_entities<'a>(&'a self, database: Option<&str>, table: &str) -> TableEntities<'a> {
        TableEntities {
            columns: self.columns.for_table(database, table),
            indexes: self.indexes.for_table(database, table),
            primary_key: self.pks.for_table(database, table),
            uniques: self.uniques.for_table(database, table),
            foreign_keys: self.fks.for_table(database, table),
            checks: self.checks.for_table(database, table),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mysql::ddl::{Generated, IndexColumn};

    #[test]
    fn rejects_multiple_explicit_database_scopes() {
        let mut left = Table::new("users");
        left.database = Some("one".into());
        let mut right = Table::new("posts");
        right.database = Some("two".into());
        let ddl =
            MySQLDDL::from_entities(vec![MySQLEntity::Table(left), MySQLEntity::Table(right)]);
        assert!(matches!(
            ddl.validate_database_scope(),
            Err(ValidationError::MultipleDatabases { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_views_and_shared_table_view_names() {
        let duplicate_views = MySQLDDL::from_entities(vec![
            MySQLEntity::View(View::new("active_users", "select 1")),
            MySQLEntity::View(View::new("active_users", "select 2")),
        ]);
        assert!(matches!(
            duplicate_views.validate(),
            Err(ValidationError::Duplicate { kind: "view", .. })
        ));

        let table_and_view = MySQLDDL::from_entities(vec![
            MySQLEntity::Table(Table::new("users")),
            MySQLEntity::View(View::new("users", "select 1")),
        ]);
        assert!(matches!(
            table_and_view.validate(),
            Err(ValidationError::Duplicate {
                kind: "table/view",
                ..
            })
        ));
    }

    #[test]
    fn canonicalizes_every_entity_into_the_selected_database() {
        let mut table = Table::new("users");
        table.database = Some("app".into());
        let ddl = MySQLDDL::try_from_entities(vec![
            MySQLEntity::Table(table),
            MySQLEntity::Column(Column::new("users", "id", "bigint")),
        ])
        .unwrap();
        assert!(
            ddl.to_entities()
                .iter()
                .all(|entity| entity.database() == Some("app"))
        );
    }

    #[test]
    fn canonicalizes_mysql_view_defaults() {
        let ddl = MySQLDDL::try_from_entities(vec![MySQLEntity::View(View::new(
            "active_users",
            "select 1",
        ))])
        .unwrap();
        let view = ddl.views.one(None, "active_users").unwrap();
        assert_eq!(view.algorithm, Some(ViewAlgorithm::Undefined));
        assert_eq!(view.sql_security, Some(ViewSqlSecurity::Definer));
    }

    #[test]
    fn rejects_database_scoped_constraint_name_collisions() {
        let mut ddl = MySQLDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.tables.push(Table::new("posts"));
        ddl.columns.push(Column::new("users", "id", "bigint"));
        ddl.columns.push(Column::new("posts", "user_id", "bigint"));
        ddl.fks.push(ForeignKey::new(
            "posts",
            "shared_constraint",
            ["user_id"],
            "users",
            ["id"],
        ));
        ddl.checks
            .push(CheckConstraint::new("users", "shared_constraint", "id > 0"));
        assert!(matches!(
            ddl.validate(),
            Err(ValidationError::Duplicate {
                kind: "constraint",
                ..
            })
        ));
    }

    #[test]
    fn rejects_missing_and_mismatched_constraint_columns() {
        let ddl = MySQLDDL::from_entities(vec![
            MySQLEntity::Table(Table::new("users")),
            MySQLEntity::Column(Column::new("users", "id", "bigint")),
            MySQLEntity::PrimaryKey(PrimaryKey::new("users", ["missing"])),
        ]);
        assert!(matches!(
            ddl.validate(),
            Err(ValidationError::MissingColumn {
                kind: "primary key",
                ..
            })
        ));

        let ddl = MySQLDDL::from_entities(vec![
            MySQLEntity::Table(Table::new("parents")),
            MySQLEntity::Column(Column::new("parents", "id", "bigint")),
            MySQLEntity::Table(Table::new("children")),
            MySQLEntity::Column(Column::new("children", "parent_id", "bigint")),
            MySQLEntity::ForeignKey(ForeignKey::new(
                "children",
                "children_parent_fk",
                ["parent_id"],
                "parents",
                ["id", "tenant_id"],
            )),
        ]);
        assert!(matches!(
            ddl.validate(),
            Err(ValidationError::MismatchedForeignKeyColumns { .. })
        ));
    }

    #[test]
    fn rejects_tables_without_columns() {
        let ddl = MySQLDDL::from_entities(vec![MySQLEntity::Table(Table::new("users"))]);
        assert!(matches!(
            ddl.validate(),
            Err(ValidationError::EmptyTable { table }) if table == "users"
        ));
    }

    #[test]
    fn rejects_blank_render_required_sql() {
        let invalid_cases = [
            (
                vec![
                    MySQLEntity::Table(Table::new("users")),
                    MySQLEntity::Column(Column::new("users", "id", "  ")),
                ],
                "column type",
            ),
            (
                {
                    let mut column = Column::new("users", "id", "bigint");
                    column.generated = Some(Generated::stored("\t"));
                    vec![
                        MySQLEntity::Table(Table::new("users")),
                        MySQLEntity::Column(column),
                    ]
                },
                "generated expression",
            ),
            (
                vec![
                    MySQLEntity::Table(Table::new("users")),
                    MySQLEntity::Column(Column::new("users", "id", "bigint")),
                    MySQLEntity::CheckConstraint(CheckConstraint::new(
                        "users",
                        "users_id_check",
                        " ",
                    )),
                ],
                "check expression",
            ),
            (
                vec![
                    MySQLEntity::Table(Table::new("users")),
                    MySQLEntity::Column(Column::new("users", "id", "bigint")),
                    MySQLEntity::Index(Index::new(
                        "users",
                        "users_functional_idx",
                        vec![IndexColumn::expression("\n")],
                    )),
                ],
                "index expression",
            ),
            (
                vec![MySQLEntity::View(View::new("active_users", "  "))],
                "view definition",
            ),
        ];

        for (entities, expected_field) in invalid_cases {
            assert!(matches!(
                MySQLDDL::from_entities(entities).validate(),
                Err(ValidationError::EmptySql { field, .. }) if field == expected_field
            ));
        }
    }

    #[test]
    fn existing_views_do_not_require_a_definition() {
        let mut view = View::new("external_users", "");
        view.definition = None;
        view.is_existing = true;
        assert!(
            MySQLDDL::try_from_entities(vec![MySQLEntity::View(view)]).is_ok(),
            "an existing view is a reference and is never rendered"
        );
    }
}
