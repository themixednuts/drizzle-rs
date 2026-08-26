//! Typed MySQL migration operations and SQL rendering.
//!
//! A rendered operation is one MySQL DDL step. Some operations, such as a
//! generated-column storage transition, deliberately expand to more than one
//! step. MySQL commits DDL implicitly, so this module never emits transaction
//! control or describes a group of statements as atomic.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Quotes one MySQL identifier, including embedded backticks.
#[must_use]
pub fn quote_identifier(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

fn qualified_name(database: Option<&str>, name: &str) -> String {
    database.map_or_else(
        || quote_identifier(name),
        |database| format!("{}.{}", quote_identifier(database), quote_identifier(name)),
    )
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
}

fn bare_option<'a>(field: &'static str, value: &'a str) -> Result<&'a str, RenderError> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        Ok(value)
    } else {
        Err(RenderError::InvalidOption {
            field,
            value: value.to_string(),
        })
    }
}

/// A MySQL column type as represented in a migration operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ColumnType {
    /// A trusted MySQL type expression such as `varchar(255)` or `bigint unsigned`.
    Sql { sql: String },
    /// A MySQL inline enum. Values are escaped as SQL string literals.
    InlineEnum { values: Vec<String> },
    /// A MySQL inline set. Values are escaped as SQL string literals.
    InlineSet { values: Vec<String> },
}

/// Storage mode for a generated column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GeneratedKind {
    Virtual,
    Stored,
}

/// A generated-column expression and storage mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedDefinition {
    pub expression: String,
    pub kind: GeneratedKind,
}

/// Complete column definition used by `ADD COLUMN` and `MODIFY COLUMN`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDefinition {
    pub name: String,
    pub column_type: ColumnType,
    pub not_null: bool,
    pub auto_increment: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub on_update: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub generated: Option<GeneratedDefinition>,
    pub comment: Option<String>,
}

/// Sort direction for an index key part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// One column or trusted SQL expression in an index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IndexColumnDefinition {
    Column {
        name: String,
        length: Option<u32>,
        order: Option<SortOrder>,
    },
    /// `sql` is a trusted schema expression. It is not identifier-quoted.
    Expression {
        sql: String,
        order: Option<SortOrder>,
    },
}

/// MySQL index access method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IndexUsing {
    Btree,
    Hash,
}

/// MySQL online-DDL algorithm accepted by `CREATE INDEX` and `ALTER TABLE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IndexAlgorithm {
    Default,
    Inplace,
    Copy,
}

/// MySQL metadata/data lock policy accepted by index DDL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IndexLock {
    Default,
    None,
    Shared,
    Exclusive,
}

/// Complete index definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexDefinition {
    pub database: Option<String>,
    pub table: String,
    pub name: String,
    pub columns: Vec<IndexColumnDefinition>,
    pub unique: bool,
    pub using: Option<IndexUsing>,
    pub algorithm: Option<IndexAlgorithm>,
    pub lock: Option<IndexLock>,
    pub comment: Option<String>,
    pub visible: Option<bool>,
}

/// Primary-key definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryKeyDefinition {
    pub database: Option<String>,
    pub table: String,
    pub columns: Vec<String>,
}

/// Unique constraint. MySQL stores and drops it as a unique index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniqueDefinition {
    pub database: Option<String>,
    pub table: String,
    pub name: String,
    pub columns: Vec<IndexColumnDefinition>,
}

/// Referential action in a foreign-key definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferentialAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
}

/// Foreign-key definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKeyDefinition {
    pub database: Option<String>,
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_database: Option<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<ReferentialAction>,
    pub on_update: Option<ReferentialAction>,
}

/// Check-constraint definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDefinition {
    pub database: Option<String>,
    pub table: String,
    pub name: String,
    /// Trusted schema SQL.
    pub expression: String,
    pub enforced: Option<bool>,
}

/// Table definition used by `CREATE TABLE`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDefinition {
    pub database: Option<String>,
    pub name: String,
    pub temporary: bool,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: Option<PrimaryKeyDefinition>,
    pub uniques: Vec<UniqueDefinition>,
    pub checks: Vec<CheckDefinition>,
    pub engine: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub comment: Option<String>,
}

/// MySQL view algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ViewAlgorithm {
    Undefined,
    Merge,
    Temptable,
}

/// MySQL view SQL security mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ViewSecurity {
    Definer,
    Invoker,
}

/// MySQL view check option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ViewCheckOption {
    Cascaded,
    Local,
}

/// View definition used by create and replace operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewDefinition {
    pub database: Option<String>,
    pub name: String,
    /// Trusted SELECT SQL without `CREATE VIEW`.
    pub definition: String,
    pub algorithm: Option<ViewAlgorithm>,
    pub definer: Option<String>,
    pub security: Option<ViewSecurity>,
    pub check_option: Option<ViewCheckOption>,
}

/// Every DDL operation emitted by the MySQL schema diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MySQLStatement {
    CreateTable {
        table: TableDefinition,
    },
    DropTable {
        database: Option<String>,
        table: String,
    },
    RenameTable {
        database: Option<String>,
        from: String,
        to: String,
    },
    AlterTableOptions {
        database: Option<String>,
        table: String,
        engine: Option<String>,
        charset: Option<String>,
        collation: Option<String>,
        comment: Option<String>,
    },
    AddColumn {
        database: Option<String>,
        table: String,
        column: ColumnDefinition,
    },
    DropColumn {
        database: Option<String>,
        table: String,
        column: String,
    },
    RenameColumn {
        database: Option<String>,
        table: String,
        from: String,
        to: String,
    },
    ModifyColumn {
        database: Option<String>,
        table: String,
        column: ColumnDefinition,
    },
    /// A required drop/add transition. Rendering produces two DDL statements.
    RecreateColumn {
        database: Option<String>,
        table: String,
        column: ColumnDefinition,
    },
    CreateIndex {
        index: IndexDefinition,
    },
    DropIndex {
        database: Option<String>,
        table: String,
        name: String,
    },
    AddPrimaryKey {
        primary_key: PrimaryKeyDefinition,
    },
    DropPrimaryKey {
        database: Option<String>,
        table: String,
    },
    AddUnique {
        unique: UniqueDefinition,
    },
    DropUnique {
        database: Option<String>,
        table: String,
        name: String,
    },
    AddForeignKey {
        foreign_key: ForeignKeyDefinition,
    },
    DropForeignKey {
        database: Option<String>,
        table: String,
        name: String,
    },
    AddCheck {
        check: CheckDefinition,
    },
    DropCheck {
        database: Option<String>,
        table: String,
        name: String,
    },
    CreateView {
        view: ViewDefinition,
    },
    ReplaceView {
        view: ViewDefinition,
    },
    DropView {
        database: Option<String>,
        view: String,
    },
    RenameView {
        database: Option<String>,
        from: String,
        to: String,
    },
}

/// Failure to render a typed operation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RenderError {
    #[error("MySQL {operation} requires at least one {item}")]
    EmptyList {
        operation: &'static str,
        item: &'static str,
    },
    #[error("MySQL {field} cannot be empty")]
    EmptySql { field: &'static str },
    #[error("MySQL foreign key cannot reference another database")]
    CrossDatabaseForeignKey,
    #[error("invalid MySQL {field} value {value:?}")]
    InvalidOption { field: &'static str, value: String },
    #[error("MySQL column {column:?} cannot combine {left} with {right}")]
    IncompatibleColumnOptions {
        column: String,
        left: &'static str,
        right: &'static str,
    },
}

fn render_column_type(column_type: &ColumnType) -> Result<String, RenderError> {
    match column_type {
        ColumnType::Sql { sql } if sql.trim().is_empty() => Err(RenderError::EmptySql {
            field: "column type",
        }),
        ColumnType::Sql { sql } => Ok(sql.trim().to_string()),
        ColumnType::InlineEnum { values } | ColumnType::InlineSet { values }
            if values.is_empty() =>
        {
            Err(RenderError::EmptyList {
                operation: "inline type",
                item: "value",
            })
        }
        ColumnType::InlineEnum { values } => Ok(format!(
            "enum({})",
            values
                .iter()
                .map(|value| quote_literal(value))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        ColumnType::InlineSet { values } => Ok(format!(
            "set({})",
            values
                .iter()
                .map(|value| quote_literal(value))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn render_column(column: &ColumnDefinition) -> Result<String, RenderError> {
    if column.auto_increment && column.default.is_some() {
        return Err(RenderError::IncompatibleColumnOptions {
            column: column.name.clone(),
            left: "AUTO_INCREMENT",
            right: "DEFAULT",
        });
    }
    if column.generated.is_some()
        && (column.auto_increment || column.default.is_some() || column.on_update.is_some())
    {
        return Err(RenderError::IncompatibleColumnOptions {
            column: column.name.clone(),
            left: "GENERATED",
            right: if column.auto_increment {
                "AUTO_INCREMENT"
            } else if column.default.is_some() {
                "DEFAULT"
            } else {
                "ON UPDATE"
            },
        });
    }
    let mut sql = format!(
        "{} {}",
        quote_identifier(&column.name),
        render_column_type(&column.column_type)?
    );
    if let Some(charset) = &column.charset {
        sql.push_str(" CHARACTER SET ");
        sql.push_str(bare_option("character set", charset)?);
    }
    if let Some(collation) = &column.collation {
        sql.push_str(" COLLATE ");
        sql.push_str(bare_option("collation", collation)?);
    }
    if let Some(generated) = &column.generated {
        if generated.expression.trim().is_empty() {
            return Err(RenderError::EmptySql {
                field: "generated expression",
            });
        }
        sql.push_str(" GENERATED ALWAYS AS (");
        sql.push_str(generated.expression.trim());
        sql.push(')');
        sql.push_str(match generated.kind {
            GeneratedKind::Virtual => " VIRTUAL",
            GeneratedKind::Stored => " STORED",
        });
    }
    sql.push_str(if column.not_null {
        " NOT NULL"
    } else {
        " NULL"
    });
    if let Some(default) = &column.default {
        sql.push_str(" DEFAULT ");
        sql.push_str(default);
    }
    if let Some(on_update) = &column.on_update {
        sql.push_str(" ON UPDATE ");
        sql.push_str(on_update);
    }
    if column.auto_increment {
        sql.push_str(" AUTO_INCREMENT");
    }
    if column.unique {
        sql.push_str(" UNIQUE");
    }
    if column.primary_key {
        sql.push_str(" PRIMARY KEY");
    }
    if let Some(comment) = &column.comment {
        sql.push_str(" COMMENT ");
        sql.push_str(&quote_literal(comment));
    }
    Ok(sql)
}

fn render_order(order: Option<SortOrder>) -> &'static str {
    match order {
        None => "",
        Some(SortOrder::Asc) => " ASC",
        Some(SortOrder::Desc) => " DESC",
    }
}

fn render_index_column(column: &IndexColumnDefinition) -> Result<String, RenderError> {
    match column {
        IndexColumnDefinition::Column {
            name,
            length,
            order,
        } => Ok(format!(
            "{}{}{}",
            quote_identifier(name),
            length.map_or_else(String::new, |length| format!("({length})")),
            render_order(*order)
        )),
        IndexColumnDefinition::Expression { sql, .. } if sql.trim().is_empty() => {
            Err(RenderError::EmptySql {
                field: "index expression",
            })
        }
        IndexColumnDefinition::Expression { sql, order } => {
            Ok(format!("({}){}", sql.trim(), render_order(*order)))
        }
    }
}

fn render_index_columns(
    operation: &'static str,
    columns: &[IndexColumnDefinition],
) -> Result<String, RenderError> {
    if columns.is_empty() {
        return Err(RenderError::EmptyList {
            operation,
            item: "column",
        });
    }
    columns
        .iter()
        .map(render_index_column)
        .collect::<Result<Vec<_>, _>>()
        .map(|columns| columns.join(", "))
}

fn render_names(operation: &'static str, names: &[String]) -> Result<String, RenderError> {
    if names.is_empty() {
        return Err(RenderError::EmptyList {
            operation,
            item: "column",
        });
    }
    Ok(names
        .iter()
        .map(|name| quote_identifier(name))
        .collect::<Vec<_>>()
        .join(", "))
}

fn render_action(action: ReferentialAction) -> &'static str {
    match action {
        ReferentialAction::NoAction => "NO ACTION",
        ReferentialAction::Restrict => "RESTRICT",
        ReferentialAction::Cascade => "CASCADE",
        ReferentialAction::SetNull => "SET NULL",
    }
}

fn render_primary_key(primary_key: &PrimaryKeyDefinition) -> Result<String, RenderError> {
    Ok(format!(
        "PRIMARY KEY ({})",
        render_names("primary key", &primary_key.columns)?
    ))
}

fn render_unique(unique: &UniqueDefinition) -> Result<String, RenderError> {
    Ok(format!(
        "CONSTRAINT {} UNIQUE ({})",
        quote_identifier(&unique.name),
        render_index_columns("unique constraint", &unique.columns)?
    ))
}

fn render_check(check: &CheckDefinition) -> Result<String, RenderError> {
    if check.expression.trim().is_empty() {
        return Err(RenderError::EmptySql {
            field: "check expression",
        });
    }
    let mut sql = format!(
        "CONSTRAINT {} CHECK ({})",
        quote_identifier(&check.name),
        check.expression.trim()
    );
    if let Some(enforced) = check.enforced {
        sql.push_str(if enforced {
            " ENFORCED"
        } else {
            " NOT ENFORCED"
        });
    }
    Ok(sql)
}

fn render_create_table(table: &TableDefinition) -> Result<String, RenderError> {
    if table.columns.is_empty() {
        return Err(RenderError::EmptyList {
            operation: "create table",
            item: "column",
        });
    }
    let mut definitions = table
        .columns
        .iter()
        .map(render_column)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(primary_key) = &table.primary_key {
        definitions.push(render_primary_key(primary_key)?);
    }
    definitions.extend(
        table
            .uniques
            .iter()
            .map(render_unique)
            .collect::<Result<Vec<_>, _>>()?,
    );
    definitions.extend(
        table
            .checks
            .iter()
            .map(render_check)
            .collect::<Result<Vec<_>, _>>()?,
    );

    let mut sql = format!(
        "CREATE {}TABLE {} (\n\t{}\n)",
        if table.temporary { "TEMPORARY " } else { "" },
        qualified_name(table.database.as_deref(), &table.name),
        definitions.join(",\n\t")
    );
    if let Some(engine) = &table.engine {
        sql.push_str(" ENGINE=");
        sql.push_str(bare_option("storage engine", engine)?);
    }
    if let Some(charset) = &table.charset {
        sql.push_str(" DEFAULT CHARACTER SET=");
        sql.push_str(bare_option("character set", charset)?);
    }
    if let Some(collation) = &table.collation {
        sql.push_str(" COLLATE=");
        sql.push_str(bare_option("collation", collation)?);
    }
    if let Some(comment) = &table.comment {
        sql.push_str(" COMMENT=");
        sql.push_str(&quote_literal(comment));
    }
    sql.push(';');
    Ok(sql)
}

fn render_create_index(index: &IndexDefinition) -> Result<String, RenderError> {
    let columns = render_index_columns("create index", &index.columns)?;
    let mut sql = format!(
        "CREATE {}INDEX {}",
        if index.unique { "UNIQUE " } else { "" },
        quote_identifier(&index.name)
    );
    if let Some(using) = index.using {
        sql.push_str(match using {
            IndexUsing::Btree => " USING BTREE",
            IndexUsing::Hash => " USING HASH",
        });
    }
    sql.push_str(" ON ");
    sql.push_str(&qualified_name(index.database.as_deref(), &index.table));
    sql.push_str(" (");
    sql.push_str(&columns);
    sql.push(')');
    if let Some(comment) = &index.comment {
        sql.push_str(" COMMENT ");
        sql.push_str(&quote_literal(comment));
    }
    if let Some(visible) = index.visible {
        sql.push_str(if visible { " VISIBLE" } else { " INVISIBLE" });
    }
    if let Some(algorithm) = index.algorithm {
        sql.push_str(match algorithm {
            IndexAlgorithm::Default => " ALGORITHM=DEFAULT",
            IndexAlgorithm::Inplace => " ALGORITHM=INPLACE",
            IndexAlgorithm::Copy => " ALGORITHM=COPY",
        });
    }
    if let Some(lock) = index.lock {
        sql.push_str(match lock {
            IndexLock::Default => " LOCK=DEFAULT",
            IndexLock::None => " LOCK=NONE",
            IndexLock::Shared => " LOCK=SHARED",
            IndexLock::Exclusive => " LOCK=EXCLUSIVE",
        });
    }
    sql.push(';');
    Ok(sql)
}

fn render_foreign_key(foreign_key: &ForeignKeyDefinition) -> Result<String, RenderError> {
    if foreign_key.database != foreign_key.referenced_database {
        return Err(RenderError::CrossDatabaseForeignKey);
    }
    let mut sql = format!(
        "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
        quote_identifier(&foreign_key.name),
        render_names("foreign key", &foreign_key.columns)?,
        qualified_name(
            foreign_key.referenced_database.as_deref(),
            &foreign_key.referenced_table,
        ),
        render_names("foreign key reference", &foreign_key.referenced_columns)?,
    );
    if let Some(action) = foreign_key.on_delete {
        sql.push_str(" ON DELETE ");
        sql.push_str(render_action(action));
    }
    if let Some(action) = foreign_key.on_update {
        sql.push_str(" ON UPDATE ");
        sql.push_str(render_action(action));
    }
    Ok(sql)
}

fn render_view(view: &ViewDefinition, replace: bool) -> Result<String, RenderError> {
    if view.definition.trim().is_empty() {
        return Err(RenderError::EmptySql {
            field: "view definition",
        });
    }
    let mut sql = String::from("CREATE ");
    if replace {
        sql.push_str("OR REPLACE ");
    }
    if let Some(algorithm) = view.algorithm {
        sql.push_str(match algorithm {
            ViewAlgorithm::Undefined => "ALGORITHM=UNDEFINED ",
            ViewAlgorithm::Merge => "ALGORITHM=MERGE ",
            ViewAlgorithm::Temptable => "ALGORITHM=TEMPTABLE ",
        });
    }
    if let Some(definer) = &view.definer {
        sql.push_str("DEFINER=");
        sql.push_str(definer);
        sql.push(' ');
    }
    if let Some(security) = view.security {
        sql.push_str(match security {
            ViewSecurity::Definer => "SQL SECURITY DEFINER ",
            ViewSecurity::Invoker => "SQL SECURITY INVOKER ",
        });
    }
    sql.push_str("VIEW ");
    sql.push_str(&qualified_name(view.database.as_deref(), &view.name));
    sql.push_str(" AS ");
    sql.push_str(view.definition.trim());
    if let Some(check_option) = view.check_option {
        sql.push_str(match check_option {
            ViewCheckOption::Cascaded => " WITH CASCADED CHECK OPTION",
            ViewCheckOption::Local => " WITH LOCAL CHECK OPTION",
        });
    }
    sql.push(';');
    Ok(sql)
}

/// Renders one typed operation into one or more independently committed DDL statements.
///
/// # Errors
///
/// Returns [`RenderError`] for empty required lists or SQL fragments and for
/// cross-database foreign keys.
pub fn render_statement(statement: &MySQLStatement) -> Result<Vec<String>, RenderError> {
    let sql = match statement {
        MySQLStatement::CreateTable { table } => vec![render_create_table(table)?],
        MySQLStatement::DropTable { database, table } => vec![format!(
            "DROP TABLE {};",
            qualified_name(database.as_deref(), table)
        )],
        MySQLStatement::RenameTable { database, from, to } => vec![format!(
            "RENAME TABLE {} TO {};",
            qualified_name(database.as_deref(), from),
            qualified_name(database.as_deref(), to)
        )],
        MySQLStatement::AlterTableOptions {
            database,
            table,
            engine,
            charset,
            collation,
            comment,
        } => {
            let mut clauses = Vec::new();
            if let Some(engine) = engine {
                clauses.push(format!("ENGINE={}", bare_option("storage engine", engine)?));
            }
            if let Some(charset) = charset {
                clauses.push(format!(
                    "DEFAULT CHARACTER SET={}",
                    bare_option("character set", charset)?
                ));
            }
            if let Some(collation) = collation {
                clauses.push(format!("COLLATE={}", bare_option("collation", collation)?));
            }
            if let Some(comment) = comment {
                clauses.push(format!("COMMENT={}", quote_literal(comment)));
            }
            if clauses.is_empty() {
                return Err(RenderError::EmptyList {
                    operation: "alter table options",
                    item: "option",
                });
            }
            vec![format!(
                "ALTER TABLE {} {};",
                qualified_name(database.as_deref(), table),
                clauses.join(" ")
            )]
        }
        MySQLStatement::AddColumn {
            database,
            table,
            column,
        } => vec![format!(
            "ALTER TABLE {} ADD COLUMN {};",
            qualified_name(database.as_deref(), table),
            render_column(column)?
        )],
        MySQLStatement::DropColumn {
            database,
            table,
            column,
        } => vec![format!(
            "ALTER TABLE {} DROP COLUMN {};",
            qualified_name(database.as_deref(), table),
            quote_identifier(column)
        )],
        MySQLStatement::RenameColumn {
            database,
            table,
            from,
            to,
        } => vec![format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {};",
            qualified_name(database.as_deref(), table),
            quote_identifier(from),
            quote_identifier(to)
        )],
        MySQLStatement::ModifyColumn {
            database,
            table,
            column,
        } => vec![format!(
            "ALTER TABLE {} MODIFY COLUMN {};",
            qualified_name(database.as_deref(), table),
            render_column(column)?
        )],
        MySQLStatement::RecreateColumn {
            database,
            table,
            column,
        } => vec![
            format!(
                "ALTER TABLE {} DROP COLUMN {};",
                qualified_name(database.as_deref(), table),
                quote_identifier(&column.name)
            ),
            format!(
                "ALTER TABLE {} ADD COLUMN {};",
                qualified_name(database.as_deref(), table),
                render_column(column)?
            ),
        ],
        MySQLStatement::CreateIndex { index } => vec![render_create_index(index)?],
        MySQLStatement::DropIndex {
            database,
            table,
            name,
        }
        | MySQLStatement::DropUnique {
            database,
            table,
            name,
        } => vec![format!(
            "ALTER TABLE {} DROP INDEX {};",
            qualified_name(database.as_deref(), table),
            quote_identifier(name)
        )],
        MySQLStatement::AddPrimaryKey { primary_key } => vec![format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(primary_key.database.as_deref(), &primary_key.table),
            render_primary_key(primary_key)?
        )],
        MySQLStatement::DropPrimaryKey { database, table } => vec![format!(
            "ALTER TABLE {} DROP PRIMARY KEY;",
            qualified_name(database.as_deref(), table)
        )],
        MySQLStatement::AddUnique { unique } => vec![format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(unique.database.as_deref(), &unique.table),
            render_unique(unique)?
        )],
        MySQLStatement::AddForeignKey { foreign_key } => vec![format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(foreign_key.database.as_deref(), &foreign_key.table),
            render_foreign_key(foreign_key)?
        )],
        MySQLStatement::DropForeignKey {
            database,
            table,
            name,
        } => vec![format!(
            "ALTER TABLE {} DROP FOREIGN KEY {};",
            qualified_name(database.as_deref(), table),
            quote_identifier(name)
        )],
        MySQLStatement::AddCheck { check } => vec![format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(check.database.as_deref(), &check.table),
            render_check(check)?
        )],
        MySQLStatement::DropCheck {
            database,
            table,
            name,
        } => vec![format!(
            "ALTER TABLE {} DROP CHECK {};",
            qualified_name(database.as_deref(), table),
            quote_identifier(name)
        )],
        MySQLStatement::CreateView { view } => vec![render_view(view, false)?],
        MySQLStatement::ReplaceView { view } => vec![render_view(view, true)?],
        MySQLStatement::DropView { database, view } => vec![format!(
            "DROP VIEW {};",
            qualified_name(database.as_deref(), view)
        )],
        MySQLStatement::RenameView { database, from, to } => vec![format!(
            "RENAME TABLE {} TO {};",
            qualified_name(database.as_deref(), from),
            qualified_name(database.as_deref(), to)
        )],
    };
    Ok(sql)
}

/// Renders a sequence while preserving operation order and statement boundaries.
///
/// # Errors
///
/// Returns the first operation rendering error.
pub fn render_statements(statements: &[MySQLStatement]) -> Result<Vec<String>, RenderError> {
    statements
        .iter()
        .try_fold(Vec::new(), |mut sql, statement| {
            sql.extend(render_statement(statement)?);
            Ok(sql)
        })
}

/// MySQL SQL generator used by the dialect integration.
#[derive(Debug, Clone, Default)]
pub struct Generator {
    pub breakpoints: bool,
}

impl Generator {
    #[must_use]
    pub const fn new() -> Self {
        Self { breakpoints: true }
    }

    #[must_use]
    pub const fn with_breakpoints(mut self, breakpoints: bool) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Renders dependency-ordered typed operations.
    ///
    /// # Errors
    ///
    /// Returns the first operation rendering error.
    pub fn generate(&self, statements: &[MySQLStatement]) -> Result<Vec<String>, RenderError> {
        render_statements(statements)
    }

    /// Joins independently executed DDL statements for a migration file.
    #[must_use]
    pub fn statements_to_sql(&self, statements: &[String]) -> String {
        if self.breakpoints {
            statements.join("\n--> statement-breakpoint\n")
        } else {
            statements.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str) -> ColumnDefinition {
        ColumnDefinition {
            name: name.to_string(),
            column_type: ColumnType::Sql {
                sql: "bigint unsigned".to_string(),
            },
            not_null: true,
            auto_increment: false,
            primary_key: false,
            unique: false,
            default: None,
            on_update: None,
            charset: None,
            collation: None,
            generated: None,
            comment: None,
        }
    }

    #[test]
    fn doubles_backticks_in_every_identifier_segment() {
        let statement = MySQLStatement::RenameTable {
            database: Some("app`db".to_string()),
            from: "old`table".to_string(),
            to: "new`table".to_string(),
        };

        assert_eq!(
            render_statement(&statement).unwrap(),
            ["RENAME TABLE `app``db`.`old``table` TO `app``db`.`new``table`;"],
        );
    }

    #[test]
    fn renders_inline_enum_values_without_splitting_commas() {
        let mut enum_column = column("status");
        enum_column.column_type = ColumnType::InlineEnum {
            values: vec!["new,queued".to_string(), "it's done".to_string()],
        };
        let statement = MySQLStatement::AddColumn {
            database: None,
            table: "jobs".to_string(),
            column: enum_column,
        };

        assert_eq!(
            render_statement(&statement).unwrap(),
            ["ALTER TABLE `jobs` ADD COLUMN `status` enum('new,queued', 'it''s done') NOT NULL;"],
        );
    }

    #[test]
    fn renders_create_table_with_mysql_options_and_constraints() {
        let mut id = column("id");
        id.auto_increment = true;
        let mut slug = column("slug");
        slug.column_type = ColumnType::Sql {
            sql: "varchar(255)".to_string(),
        };
        slug.generated = Some(GeneratedDefinition {
            expression: "lower(`name`)".to_string(),
            kind: GeneratedKind::Stored,
        });
        let table = TableDefinition {
            database: Some("app".to_string()),
            name: "users".to_string(),
            temporary: false,
            columns: vec![id, slug],
            primary_key: Some(PrimaryKeyDefinition {
                database: Some("app".to_string()),
                table: "users".to_string(),
                columns: vec!["id".to_string()],
            }),
            uniques: vec![UniqueDefinition {
                database: Some("app".to_string()),
                table: "users".to_string(),
                name: "users_slug_unique".to_string(),
                columns: vec![IndexColumnDefinition::Column {
                    name: "slug".to_string(),
                    length: None,
                    order: None,
                }],
            }],
            checks: vec![CheckDefinition {
                database: Some("app".to_string()),
                table: "users".to_string(),
                name: "id_positive".to_string(),
                expression: "`id` > 0".to_string(),
                enforced: Some(true),
            }],
            engine: Some("InnoDB".to_string()),
            charset: Some("utf8mb4".to_string()),
            collation: Some("utf8mb4_0900_ai_ci".to_string()),
            comment: Some("accounts".to_string()),
        };

        assert_eq!(
            render_statement(&MySQLStatement::CreateTable { table }).unwrap(),
            [concat!(
                "CREATE TABLE `app`.`users` (\n",
                "\t`id` bigint unsigned NOT NULL AUTO_INCREMENT,\n",
                "\t`slug` varchar(255) GENERATED ALWAYS AS (lower(`name`)) STORED NOT NULL,\n",
                "\tPRIMARY KEY (`id`),\n",
                "\tCONSTRAINT `users_slug_unique` UNIQUE (`slug`),\n",
                "\tCONSTRAINT `id_positive` CHECK (`id` > 0) ENFORCED\n",
                ") ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='accounts';"
            )],
        );
    }

    #[test]
    fn renders_index_options_instead_of_dropping_them() {
        let statement = MySQLStatement::CreateIndex {
            index: IndexDefinition {
                database: Some("app".to_string()),
                table: "users".to_string(),
                name: "users_email".to_string(),
                columns: vec![IndexColumnDefinition::Column {
                    name: "email".to_string(),
                    length: Some(32),
                    order: Some(SortOrder::Desc),
                }],
                unique: true,
                using: Some(IndexUsing::Btree),
                algorithm: Some(IndexAlgorithm::Inplace),
                lock: Some(IndexLock::None),
                comment: None,
                visible: None,
            },
        };

        assert_eq!(
            render_statement(&statement).unwrap(),
            [
                "CREATE UNIQUE INDEX `users_email` USING BTREE ON `app`.`users` (`email`(32) DESC) ALGORITHM=INPLACE LOCK=NONE;"
            ],
        );
    }

    #[test]
    fn uses_constraint_specific_drop_syntax() {
        let statements = [
            MySQLStatement::DropPrimaryKey {
                database: None,
                table: "child".to_string(),
            },
            MySQLStatement::DropForeignKey {
                database: None,
                table: "child".to_string(),
                name: "child_parent_fk".to_string(),
            },
            MySQLStatement::DropCheck {
                database: None,
                table: "child".to_string(),
                name: "positive".to_string(),
            },
            MySQLStatement::DropUnique {
                database: None,
                table: "child".to_string(),
                name: "child_code_unique".to_string(),
            },
        ];

        assert_eq!(
            render_statements(&statements).unwrap(),
            [
                "ALTER TABLE `child` DROP PRIMARY KEY;",
                "ALTER TABLE `child` DROP FOREIGN KEY `child_parent_fk`;",
                "ALTER TABLE `child` DROP CHECK `positive`;",
                "ALTER TABLE `child` DROP INDEX `child_code_unique`;",
            ],
        );
    }

    #[test]
    fn generated_recreation_is_two_unwrapped_ddl_statements() {
        let mut generated = column("slug");
        generated.generated = Some(GeneratedDefinition {
            expression: "lower(`name`)".to_string(),
            kind: GeneratedKind::Virtual,
        });

        let sql = render_statement(&MySQLStatement::RecreateColumn {
            database: None,
            table: "users".to_string(),
            column: generated,
        })
        .unwrap();

        assert_eq!(sql.len(), 2);
        assert!(sql[0].contains("DROP COLUMN"));
        assert!(sql[1].contains("ADD COLUMN"));
        assert!(sql.iter().all(|statement| !statement.contains("BEGIN")));
        assert!(sql.iter().all(|statement| !statement.contains("COMMIT")));
    }

    #[test]
    fn rejects_cross_database_foreign_key() {
        let statement = MySQLStatement::AddForeignKey {
            foreign_key: ForeignKeyDefinition {
                database: Some("app".to_string()),
                table: "child".to_string(),
                name: "child_parent_fk".to_string(),
                columns: vec!["parent_id".to_string()],
                referenced_database: Some("other".to_string()),
                referenced_table: "parent".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete: Some(ReferentialAction::Cascade),
                on_update: None,
            },
        };

        assert_eq!(
            render_statement(&statement),
            Err(RenderError::CrossDatabaseForeignKey),
        );
    }

    #[test]
    fn rejects_unsafe_table_option_tokens() {
        let statement = MySQLStatement::AlterTableOptions {
            database: None,
            table: "users".to_string(),
            engine: Some("InnoDB; DROP TABLE users".to_string()),
            charset: None,
            collation: None,
            comment: None,
        };

        assert!(matches!(
            render_statement(&statement),
            Err(RenderError::InvalidOption {
                field: "storage engine",
                ..
            })
        ));
    }
}
