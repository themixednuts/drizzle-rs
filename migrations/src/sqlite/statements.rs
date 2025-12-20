//! SQLite SQL statement types and generation (v7 DDL format)
//!
//! This implements the full statement generation from drizzle-kit beta.
//! - JsonStatement enum represents migration operations
//! - Convertor functions convert statements to SQL strings

use crate::sqlite::ddl::{
    CheckConstraint, Column, ForeignKey, GeneratedType, Index, PrimaryKey, UniqueConstraint, View,
};
use serde::{Deserialize, Serialize};

/// SQL statement breakpoint marker (used by drizzle-kit)
pub const BREAKPOINT: &str = "--> statement-breakpoint";

// =============================================================================
// JSON Statement Types (matching statements.ts)
// =============================================================================

/// Full table information for create/recreate operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableFull {
    pub name: String,
    pub columns: Vec<Column>,
    pub pk: Option<PrimaryKey>,
    pub fks: Vec<ForeignKey>,
    pub uniques: Vec<UniqueConstraint>,
    pub checks: Vec<CheckConstraint>,
    /// Whether the table has STRICT mode enabled
    #[serde(default)]
    pub strict: bool,
    /// Whether the table is WITHOUT ROWID
    #[serde(default)]
    pub without_rowid: bool,
}

impl TableFull {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            columns: Vec::new(),
            pk: None,
            fks: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            strict: false,
            without_rowid: false,
        }
    }
}

/// All possible JSON statement types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonStatement {
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    RenameTable(RenameTableStatement),
    AddColumn(AddColumnStatement),
    DropColumn(DropColumnStatement),
    RenameColumn(RenameColumnStatement),
    RecreateColumn(RecreateColumnStatement),
    RecreateTable(RecreateTableStatement),
    CreateIndex(CreateIndexStatement),
    DropIndex(DropIndexStatement),
    CreateView(CreateViewStatement),
    DropView(DropViewStatement),
    RenameView(RenameViewStatement),
}

impl JsonStatement {
    /// Get the type name of this statement
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::CreateTable(_) => "create_table",
            Self::DropTable(_) => "drop_table",
            Self::RenameTable(_) => "rename_table",
            Self::AddColumn(_) => "add_column",
            Self::DropColumn(_) => "drop_column",
            Self::RenameColumn(_) => "rename_column",
            Self::RecreateColumn(_) => "recreate_column",
            Self::RecreateTable(_) => "recreate_table",
            Self::CreateIndex(_) => "create_index",
            Self::DropIndex(_) => "drop_index",
            Self::CreateView(_) => "create_view",
            Self::DropView(_) => "drop_view",
            Self::RenameView(_) => "rename_view",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTableStatement {
    pub table: TableFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropTableStatement {
    pub table_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameTableStatement {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddColumnStatement {
    pub column: Column,
    pub fk: Option<ForeignKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropColumnStatement {
    pub column: Column,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameColumnStatement {
    pub table: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecreateColumnStatement {
    pub column: Column,
    pub fk: Option<ForeignKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecreateTableStatement {
    pub from: TableFull,
    pub to: TableFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIndexStatement {
    pub index: Index,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropIndexStatement {
    pub index: Index,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateViewStatement {
    pub view: View,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropViewStatement {
    pub view: View,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameViewStatement {
    pub from: View,
    pub to: View,
}

// =============================================================================
// Convertor - Statement to SQL (matching convertor.ts)
// =============================================================================

/// Convert a JSON statement to SQL string(s)
pub fn convert_statement(statement: &JsonStatement) -> Vec<String> {
    match statement {
        JsonStatement::CreateTable(st) => vec![convert_create_table(st)],
        JsonStatement::DropTable(st) => vec![convert_drop_table(st)],
        JsonStatement::RenameTable(st) => vec![convert_rename_table(st)],
        JsonStatement::AddColumn(st) => vec![convert_add_column(st)],
        JsonStatement::DropColumn(st) => vec![convert_drop_column(st)],
        JsonStatement::RenameColumn(st) => vec![convert_rename_column(st)],
        JsonStatement::RecreateColumn(st) => convert_recreate_column(st),
        JsonStatement::RecreateTable(st) => convert_recreate_table(st),
        JsonStatement::CreateIndex(st) => vec![convert_create_index(st)],
        JsonStatement::DropIndex(st) => vec![convert_drop_index(st)],
        JsonStatement::CreateView(st) => vec![convert_create_view(st)],
        JsonStatement::DropView(st) => vec![convert_drop_view(st)],
        JsonStatement::RenameView(st) => vec![convert_rename_view(st)],
    }
}

/// Convert multiple statements to SQL with optional breakpoints
pub fn statements_to_sql(statements: &[JsonStatement], breakpoints: bool) -> String {
    let sql_statements: Vec<String> = statements.iter().flat_map(convert_statement).collect();

    if breakpoints {
        sql_statements.join(&format!("\n{}\n", BREAKPOINT))
    } else {
        sql_statements.join("\n")
    }
}

/// A grouped statement with its corresponding SQL
#[derive(Debug, Clone)]
pub struct GroupedStatement {
    /// The JSON statement
    pub json_statement: JsonStatement,
    /// The generated SQL statements
    pub sql_statements: Vec<String>,
}

/// Result of converting JSON statements to SQL
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// All SQL statements flattened
    pub sql_statements: Vec<String>,
    /// Statements grouped with their JSON source
    pub grouped_statements: Vec<GroupedStatement>,
}

/// Convert JSON statements to SQL with grouping information
pub fn from_json(statements: Vec<JsonStatement>) -> ConversionResult {
    let grouped: Vec<GroupedStatement> = statements
        .into_iter()
        .map(|statement| {
            let sql_statements = convert_statement(&statement);
            GroupedStatement {
                json_statement: statement,
                sql_statements,
            }
        })
        .collect();

    let sql_statements: Vec<String> = grouped
        .iter()
        .flat_map(|g| g.sql_statements.clone())
        .collect();

    ConversionResult {
        sql_statements,
        grouped_statements: grouped,
    }
}

// =============================================================================
// Individual Convertors
// =============================================================================

fn convert_create_table(st: &CreateTableStatement) -> String {
    let table = &st.table;
    let mut sql = format!("CREATE TABLE `{}` (\n", table.name);

    // Column definitions
    for (i, column) in table.columns.iter().enumerate() {
        // Check if this column is the sole PK (inline PRIMARY KEY)
        let is_column_pk = table.pk.as_ref().is_some_and(|pk| {
            pk.columns.len() == 1 && pk.columns[0] == column.name && !pk.name_explicit
        });

        // For INTEGER PRIMARY KEY, SQLite allows NULL unless NOT NULL is explicit
        let omit_not_null = is_column_pk && column.sql_type.to_lowercase().starts_with("int");

        let pk_statement = if is_column_pk { " PRIMARY KEY" } else { "" };
        let not_null = if column.not_null && !omit_not_null {
            " NOT NULL"
        } else {
            ""
        };

        // Check for single-column unique constraint
        let unique = table
            .uniques
            .iter()
            .find(|u| u.columns.len() == 1 && u.columns[0] == column.name && !u.name_explicit);
        let unique_statement = if unique.is_some() { " UNIQUE" } else { "" };

        let default = column
            .default
            .as_ref()
            .map(|d| format!(" DEFAULT {}", d))
            .unwrap_or_default();

        let autoincrement = if column.autoincrement.unwrap_or(false) {
            " AUTOINCREMENT"
        } else {
            ""
        };

        let generated = column
            .generated
            .as_ref()
            .map(|g| {
                let gen_type = match g.gen_type {
                    GeneratedType::Stored => "STORED",
                    GeneratedType::Virtual => "VIRTUAL",
                };
                format!(" GENERATED ALWAYS AS {} {}", g.expression, gen_type)
            })
            .unwrap_or_default();

        sql.push_str(&format!(
            "\t`{}` {}{}{}{}{}{}{}",
            column.name,
            column.sql_type.to_uppercase(),
            pk_statement,
            autoincrement,
            default,
            generated,
            not_null,
            unique_statement
        ));

        if i < table.columns.len() - 1 {
            sql.push_str(",\n");
        }
    }

    // Composite PK or explicit named PK
    if let Some(pk) = &table.pk
        && (pk.columns.len() > 1 || pk.name_explicit)
    {
        sql.push_str(",\n\t");
        let cols = pk
            .columns
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(&format!("CONSTRAINT `{}` PRIMARY KEY({})", pk.name, cols));
    }

    // Foreign keys
    for fk in &table.fks {
        sql.push_str(",\n\t");
        let from_cols = fk
            .columns
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(",");
        let to_cols = fk
            .columns_to
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(",");

        let on_update = fk
            .on_update
            .as_ref()
            .filter(|a| *a != "NO ACTION")
            .map(|a| format!(" ON UPDATE {}", a))
            .unwrap_or_default();
        let on_delete = fk
            .on_delete
            .as_ref()
            .filter(|a| *a != "NO ACTION")
            .map(|a| format!(" ON DELETE {}", a))
            .unwrap_or_default();

        sql.push_str(&format!(
            "CONSTRAINT `{}` FOREIGN KEY ({}) REFERENCES `{}`({}){}{}",
            fk.name, from_cols, fk.table_to, to_cols, on_update, on_delete
        ));
    }

    // Multi-column unique constraints
    for unique in table.uniques.iter().filter(|u| u.columns.len() > 1) {
        sql.push_str(",\n\t");
        let cols = unique
            .columns
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join("`,`");
        sql.push_str(&format!("CONSTRAINT `{}` UNIQUE(`{}`)", unique.name, cols));
    }

    // Check constraints
    for check in &table.checks {
        sql.push_str(",\n\t");
        sql.push_str(&format!(
            "CONSTRAINT \"{}\" CHECK({})",
            check.name, check.value
        ));
    }

    sql.push_str("\n)");

    // Add table options
    if table.without_rowid {
        sql.push_str(" WITHOUT ROWID");
    }
    if table.strict {
        sql.push_str(" STRICT");
    }

    sql.push_str(";\n");
    sql
}

fn convert_drop_table(st: &DropTableStatement) -> String {
    format!("DROP TABLE `{}`;", st.table_name)
}

fn convert_rename_table(st: &RenameTableStatement) -> String {
    format!("ALTER TABLE `{}` RENAME TO `{}`;", st.from, st.to)
}

fn convert_add_column(st: &AddColumnStatement) -> String {
    let column = &st.column;

    let default = column
        .default
        .as_ref()
        .map(|d| format!(" DEFAULT {}", d))
        .unwrap_or_default();

    let not_null = if column.not_null { " NOT NULL" } else { "" };

    let generated = column
        .generated
        .as_ref()
        .map(|g| {
            let gen_type = match g.gen_type {
                GeneratedType::Stored => "STORED",
                GeneratedType::Virtual => "VIRTUAL",
            };
            format!(" GENERATED ALWAYS AS {} {}", g.expression, gen_type)
        })
        .unwrap_or_default();

    let reference = st
        .fk
        .as_ref()
        .map(|fk| {
            if fk.name_explicit {
                format!(
                    " CONSTRAINT `{}` REFERENCES {}({})",
                    fk.name,
                    fk.table_to,
                    fk.columns_to.join(",")
                )
            } else {
                format!(" REFERENCES {}({})", fk.table_to, fk.columns_to.join(","))
            }
        })
        .unwrap_or_default();

    format!(
        "ALTER TABLE `{}` ADD `{}` {}{}{}{}{};",
        column.table,
        column.name,
        column.sql_type.to_uppercase(),
        default,
        generated,
        not_null,
        reference
    )
}

fn convert_drop_column(st: &DropColumnStatement) -> String {
    format!(
        "ALTER TABLE `{}` DROP COLUMN `{}`;",
        st.column.table, st.column.name
    )
}

fn convert_rename_column(st: &RenameColumnStatement) -> String {
    format!(
        "ALTER TABLE `{}` RENAME COLUMN `{}` TO `{}`;",
        st.table, st.from, st.to
    )
}

fn convert_recreate_column(st: &RecreateColumnStatement) -> Vec<String> {
    // Drop and re-add the column
    let drop = format!(
        "ALTER TABLE `{}` DROP COLUMN `{}`;",
        st.column.table, st.column.name
    );
    let add = convert_add_column(&AddColumnStatement {
        column: st.column.clone(),
        fk: st.fk.clone(),
    });
    vec![drop, add]
}

fn convert_recreate_table(st: &RecreateTableStatement) -> Vec<String> {
    let name = &st.to.name;
    let new_table_name = format!("__new_{}", name);

    // Get columns to copy (non-generated columns that exist in both)
    let column_names: Vec<String> = st
        .from
        .columns
        .iter()
        .filter(|col| {
            col.generated.is_none()
                && st
                    .to
                    .columns
                    .iter()
                    .any(|c| c.name == col.name && c.generated.is_none())
        })
        .map(|col| format!("`{}`", col.name))
        .collect();
    let cols_str = column_names.join(", ");

    let mut statements = Vec::new();

    // 1. Disable foreign keys
    statements.push("PRAGMA foreign_keys=OFF;".to_string());

    // 2. Create new table with temp name
    let mut tmp_table = st.to.clone();
    tmp_table.name = new_table_name.clone();
    // Update check constraint table references
    for check in &mut tmp_table.checks {
        check.table = new_table_name.clone().into();
    }
    statements.push(convert_create_table(&CreateTableStatement {
        table: tmp_table,
    }));

    // 3. Copy data
    statements.push(format!(
        "INSERT INTO `{}`({}) SELECT {} FROM `{}`;",
        new_table_name, cols_str, cols_str, name
    ));

    // 4. Drop old table
    statements.push(format!("DROP TABLE `{}`;", name));

    // 5. Rename new table
    statements.push(format!(
        "ALTER TABLE `{}` RENAME TO `{}`;",
        new_table_name, name
    ));

    // 6. Re-enable foreign keys
    statements.push("PRAGMA foreign_keys=ON;".to_string());

    statements
}

fn convert_create_index(st: &CreateIndexStatement) -> String {
    let index = &st.index;
    let unique = if index.is_unique { "UNIQUE " } else { "" };

    let cols = index
        .columns
        .iter()
        .map(|c| {
            if c.is_expression {
                c.value.to_string()
            } else {
                format!("`{}`", c.value)
            }
        })
        .collect::<Vec<_>>()
        .join(",");

    let where_clause = index
        .where_clause
        .as_ref()
        .map(|w| format!(" WHERE {}", w))
        .unwrap_or_default();

    format!(
        "CREATE {}INDEX `{}` ON `{}` ({}){};",
        unique, index.name, index.table, cols, where_clause
    )
}

fn convert_drop_index(st: &DropIndexStatement) -> String {
    format!("DROP INDEX IF EXISTS `{}`;", st.index.name)
}

fn convert_create_view(st: &CreateViewStatement) -> String {
    let default_def = std::borrow::Cow::Borrowed("");
    format!(
        "CREATE VIEW `{}` AS {};",
        st.view.name,
        st.view.definition.as_ref().unwrap_or(&default_def)
    )
}

fn convert_drop_view(st: &DropViewStatement) -> String {
    format!("DROP VIEW `{}`;", st.view.name)
}

fn convert_rename_view(st: &RenameViewStatement) -> String {
    let default_def = std::borrow::Cow::Borrowed("");
    // SQLite doesn't support RENAME VIEW, so we drop and recreate
    format!(
        "DROP VIEW IF EXISTS `{}`;\nCREATE VIEW `{}` AS {};",
        st.from.name,
        st.to.name,
        st.to.definition.as_ref().unwrap_or(&default_def)
    )
}

// =============================================================================
// Statement Preparation Helpers
// =============================================================================

/// Prepare add column statements with FK associations
pub fn prepare_add_columns(columns: &[Column], fks: &[ForeignKey]) -> Vec<AddColumnStatement> {
    columns
        .iter()
        .map(|col| {
            let fk = fks
                .iter()
                .find(|fk| {
                    fk.columns.len() == 1 && fk.columns[0] == col.name && fk.table == col.table
                })
                .cloned();
            AddColumnStatement {
                column: col.clone(),
                fk,
            }
        })
        .collect()
}

// =============================================================================
// Topological Sorting for Table Dependencies
// =============================================================================

use crate::sqlite::SchemaDiff;
use crate::sqlite::collection::{DiffType, EntityDiff};
use crate::traits::EntityKind;
use std::collections::{HashMap, HashSet};

/// Result of topological sorting with circular dependency detection
pub struct TopologicalSortResult<'a> {
    /// Sorted tables
    pub tables: Vec<&'a EntityDiff>,
    /// Whether circular dependencies were detected
    pub has_circular_deps: bool,
}

/// Topological sort tables for CREATE: referenced tables come first
fn topological_sort_tables_for_create<'a>(
    tables: &[&'a EntityDiff],
    diff: &SchemaDiff,
) -> TopologicalSortResult<'a> {
    if tables.len() <= 1 {
        return TopologicalSortResult {
            tables: tables.to_vec(),
            has_circular_deps: false,
        };
    }

    // Build a map of table name -> entity diff
    let mut table_map: HashMap<String, &EntityDiff> = HashMap::new();
    for t in tables {
        if let Some(name) = t.name.split(':').next_back() {
            table_map.insert(name.to_string(), *t);
        }
    }

    // Build dependency graph: table -> tables it depends on (via FKs)
    let mut dependencies: HashMap<String, HashSet<String>> = HashMap::new();
    for table_name in table_map.keys() {
        dependencies.insert(table_name.clone(), HashSet::new());
    }

    // Find FK dependencies
    for fk_diff in diff.by_kind(EntityKind::ForeignKey) {
        if fk_diff.diff_type == DiffType::Create
            && let Some(crate::sqlite::ddl::SqliteEntity::ForeignKey(fk)) = fk_diff.right.as_ref()
        {
            let from_table = fk.table.to_string();
            let to_table = fk.table_to.to_string();
            // from_table depends on to_table (to_table must be created first)
            if table_map.contains_key(&from_table)
                && table_map.contains_key(&to_table)
                && let Some(deps) = dependencies.get_mut(&from_table)
            {
                deps.insert(to_table);
            }
        }
    }

    // Tables with no dependencies come first, then tables that depend on them, etc.
    let mut result = Vec::new();
    let mut remaining: HashSet<String> = table_map.keys().cloned().collect();
    let mut satisfied: HashSet<String> = HashSet::new();
    let mut has_circular_deps = false;

    while !remaining.is_empty() {
        // Find tables whose dependencies are all satisfied
        let ready: Vec<String> = remaining
            .iter()
            .filter(|t| {
                dependencies
                    .get(*t)
                    .map(|deps| deps.iter().all(|d| satisfied.contains(d)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        if ready.is_empty() {
            // Circular dependency detected - add remaining in any order
            has_circular_deps = true;
            for t in &remaining {
                if let Some(entity) = table_map.get(t) {
                    result.push(*entity);
                }
            }
            break;
        }

        for t in ready {
            remaining.remove(&t);
            satisfied.insert(t.clone());
            if let Some(entity) = table_map.get(&t) {
                result.push(*entity);
            }
        }
    }

    TopologicalSortResult {
        tables: result,
        has_circular_deps,
    }
}

/// Topological sort tables for DROP: tables with FKs come first (reverse of create)
fn topological_sort_tables_for_drop<'a>(
    tables: &[&'a EntityDiff],
    diff: &SchemaDiff,
) -> TopologicalSortResult<'a> {
    // For drops, reverse the create order: tables that reference others drop first
    let create_result = topological_sort_tables_for_create(tables, diff);
    TopologicalSortResult {
        tables: create_result.tables.into_iter().rev().collect(),
        has_circular_deps: create_result.has_circular_deps,
    }
}

// =============================================================================
// SQLite SQL Generator
// =============================================================================

/// SQLite SQL generator for migration diffs
///
/// Generates SQL statements from schema diffs with proper ordering:
/// 1. Table drops (reverse dependency order)
/// 2. Table creates (dependency order - referenced tables first)
/// 3. Column additions for existing tables
/// 4. Index operations
pub struct SqliteGenerator {
    /// Whether to include statement breakpoints
    pub breakpoints: bool,
}

impl Default for SqliteGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteGenerator {
    pub fn new() -> Self {
        Self { breakpoints: true }
    }

    pub fn with_breakpoints(mut self, breakpoints: bool) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Generate SQL from a schema diff
    pub fn generate_migration(&self, diff: &SchemaDiff) -> Vec<String> {
        let mut statements = Vec::new();

        // Process table drops first (in reverse dependency order - tables with FKs first)
        let dropped_tables = diff.dropped_tables();
        let drop_result = topological_sort_tables_for_drop(&dropped_tables, diff);
        
        // If there are circular dependencies in drops, wrap with PRAGMA
        if drop_result.has_circular_deps && !drop_result.tables.is_empty() {
            statements.push("PRAGMA foreign_keys=OFF;".to_string());
        }
        
        for entity_diff in &drop_result.tables {
            if let Some(name) = entity_diff.name.split(':').next_back() {
                statements.push(convert_drop_table(&DropTableStatement {
                    table_name: name.to_string(),
                }));
            }
        }
        
        if drop_result.has_circular_deps && !drop_result.tables.is_empty() {
            statements.push("PRAGMA foreign_keys=ON;".to_string());
        }

        // Process table creates (in dependency order - referenced tables first)
        let created_tables = diff.created_tables();
        let create_result = topological_sort_tables_for_create(&created_tables, diff);
        
        // If there are circular dependencies, wrap creates with PRAGMA to allow out-of-order creation
        if create_result.has_circular_deps && !create_result.tables.is_empty() {
            statements.push("PRAGMA foreign_keys=OFF;".to_string());
        }
        
        for entity_diff in &create_result.tables {
            if let Some(crate::sqlite::ddl::SqliteEntity::Table(table)) = entity_diff.right.as_ref()
            {
                // Extract columns for this table
                let columns_for_table: Vec<Column> = diff
                    .by_kind(EntityKind::Column)
                    .into_iter()
                    .filter(|d| d.diff_type == DiffType::Create)
                    .filter_map(|d| d.right.as_ref())
                    .filter_map(|e| {
                        if let crate::sqlite::ddl::SqliteEntity::Column(c) = e {
                            if c.table == table.name {
                                Some(c.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Extract pk for this table
                let pk = diff
                    .by_kind(EntityKind::PrimaryKey)
                    .into_iter()
                    .filter(|d| d.diff_type == DiffType::Create)
                    .filter_map(|d| d.right.as_ref())
                    .find_map(|e| {
                        if let crate::sqlite::ddl::SqliteEntity::PrimaryKey(p) = e {
                            if p.table == table.name {
                                Some(p.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });

                // Extract fks for this table
                let fks: Vec<ForeignKey> = diff
                    .by_kind(EntityKind::ForeignKey)
                    .into_iter()
                    .filter(|d| d.diff_type == DiffType::Create)
                    .filter_map(|d| d.right.as_ref())
                    .filter_map(|e| {
                        if let crate::sqlite::ddl::SqliteEntity::ForeignKey(fk) = e {
                            if fk.table == table.name {
                                Some(fk.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Extract uniques for this table
                let uniques: Vec<UniqueConstraint> = diff
                    .by_kind(EntityKind::UniqueConstraint)
                    .into_iter()
                    .filter(|d| d.diff_type == DiffType::Create)
                    .filter_map(|d| d.right.as_ref())
                    .filter_map(|e| {
                        if let crate::sqlite::ddl::SqliteEntity::UniqueConstraint(u) = e {
                            if u.table == table.name {
                                Some(u.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Extract checks for this table
                let checks: Vec<CheckConstraint> = diff
                    .by_kind(EntityKind::CheckConstraint)
                    .into_iter()
                    .filter(|d| d.diff_type == DiffType::Create)
                    .filter_map(|d| d.right.as_ref())
                    .filter_map(|e| {
                        if let crate::sqlite::ddl::SqliteEntity::CheckConstraint(c) = e {
                            if c.table == table.name {
                                Some(c.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                let table_full = TableFull {
                    name: table.name.to_string(),
                    columns: columns_for_table,
                    pk,
                    fks,
                    uniques,
                    checks,
                    strict: table.strict,
                    without_rowid: table.without_rowid,
                };
                statements.push(convert_create_table(&CreateTableStatement {
                    table: table_full,
                }));
            }
        }
        
        // Re-enable foreign keys if we disabled them for circular dependencies
        if create_result.has_circular_deps && !create_result.tables.is_empty() {
            statements.push("PRAGMA foreign_keys=ON;".to_string());
        }

        // Process column additions (for existing tables)
        for entity_diff in diff.by_kind(EntityKind::Column) {
            if entity_diff.diff_type == DiffType::Create
                && let Some(crate::sqlite::ddl::SqliteEntity::Column(col)) =
                    entity_diff.right.as_ref()
            {
                let table_was_created = diff.created_tables().iter().any(|t| t.name == col.table);

                if !table_was_created {
                    statements.push(convert_add_column(&AddColumnStatement {
                        column: col.clone(),
                        fk: None,
                    }));
                }
            }
        }

        // Process index operations
        for entity_diff in diff.by_kind(EntityKind::Index) {
            match entity_diff.diff_type {
                DiffType::Drop => {
                    if let Some(crate::sqlite::ddl::SqliteEntity::Index(idx)) =
                        entity_diff.left.as_ref()
                    {
                        statements.push(convert_drop_index(&DropIndexStatement {
                            index: idx.clone(),
                        }));
                    }
                }
                DiffType::Create => {
                    if let Some(crate::sqlite::ddl::SqliteEntity::Index(idx)) =
                        entity_diff.right.as_ref()
                    {
                        statements.push(convert_create_index(&CreateIndexStatement {
                            index: idx.clone(),
                        }));
                    }
                }
                DiffType::Alter => {
                    // For index alter: drop old, create new
                    if let (
                        Some(crate::sqlite::ddl::SqliteEntity::Index(old)),
                        Some(crate::sqlite::ddl::SqliteEntity::Index(new)),
                    ) = (entity_diff.left.as_ref(), entity_diff.right.as_ref())
                    {
                        statements.push(convert_drop_index(&DropIndexStatement {
                            index: old.clone(),
                        }));
                        statements.push(convert_create_index(&CreateIndexStatement {
                            index: new.clone(),
                        }));
                    }
                }
            }
        }

        statements
    }

    /// Generate SQL from migration statements
    pub fn statements_to_sql(&self, statements: &[String]) -> String {
        if self.breakpoints {
            statements.join(&format!("\n{}\n", BREAKPOINT))
        } else {
            statements.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::ddl::{Column, IndexColumn};

    #[test]
    fn test_create_table_simple() {
        let table = TableFull {
            name: "users".to_string(),
            columns: vec![
                Column::new("users", "id", "integer").not_null(),
                Column::new("users", "name", "text").not_null(),
            ],
            pk: None,
            fks: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            strict: false,
            without_rowid: false,
        };

        let sql = convert_create_table(&CreateTableStatement { table });
        assert!(sql.contains("CREATE TABLE `users`"));
        assert!(sql.contains("`id` INTEGER NOT NULL"));
        assert!(sql.contains("`name` TEXT NOT NULL"));
    }

    #[test]
    fn test_create_table_with_pk() {
        let table = TableFull {
            name: "users".to_string(),
            columns: vec![Column::new("users", "id", "integer")],
            pk: Some(PrimaryKey::from_strings(
                "users".to_string(),
                "users_pk".to_string(),
                vec!["id".to_string()],
            )),
            fks: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            strict: false,
            without_rowid: false,
        };

        let sql = convert_create_table(&CreateTableStatement { table });
        assert!(sql.contains("`id` INTEGER PRIMARY KEY"));
    }

    #[test]
    fn test_drop_table() {
        let sql = convert_drop_table(&DropTableStatement {
            table_name: "users".to_string(),
        });
        assert_eq!(sql, "DROP TABLE `users`;");
    }

    #[test]
    fn test_add_column() {
        let sql = convert_add_column(&AddColumnStatement {
            column: Column::new("users", "email", "text").not_null(),
            fk: None,
        });
        assert_eq!(sql, "ALTER TABLE `users` ADD `email` TEXT NOT NULL;");
    }

    #[test]
    fn test_create_index() {
        let index = Index::new(
            "users",
            "idx_users_email",
            vec![IndexColumn {
                value: "email".into(),
                is_expression: false,
            }],
        )
        .unique();

        let sql = convert_create_index(&CreateIndexStatement { index });
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("`idx_users_email`"));
    }

    #[test]
    fn test_create_table_strict() {
        let table = TableFull {
            name: "data".to_string(),
            columns: vec![
                Column::new("data", "id", "integer").not_null(),
                Column::new("data", "value", "text").not_null(),
            ],
            pk: None,
            fks: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            strict: true,
            without_rowid: false,
        };

        let sql = convert_create_table(&CreateTableStatement { table });
        assert!(sql.contains("CREATE TABLE `data`"));
        assert!(sql.contains(") STRICT;"), "Expected STRICT suffix, got: {}", sql);
    }

    #[test]
    fn test_create_table_without_rowid() {
        let table = TableFull {
            name: "kv".to_string(),
            columns: vec![
                Column::new("kv", "key", "text").not_null(),
                Column::new("kv", "value", "blob"),
            ],
            pk: Some(PrimaryKey::from_strings(
                "kv".to_string(),
                "kv_pk".to_string(),
                vec!["key".to_string()],
            )),
            fks: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            strict: false,
            without_rowid: true,
        };

        let sql = convert_create_table(&CreateTableStatement { table });
        assert!(sql.contains("WITHOUT ROWID"), "Expected WITHOUT ROWID, got: {}", sql);
    }

    #[test]
    fn test_create_table_strict_without_rowid() {
        let table = TableFull {
            name: "cache".to_string(),
            columns: vec![
                Column::new("cache", "key", "text").not_null(),
                Column::new("cache", "data", "blob"),
            ],
            pk: Some(PrimaryKey::from_strings(
                "cache".to_string(),
                "cache_pk".to_string(),
                vec!["key".to_string()],
            )),
            fks: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            strict: true,
            without_rowid: true,
        };

        let sql = convert_create_table(&CreateTableStatement { table });
        assert!(sql.contains("WITHOUT ROWID STRICT"), "Expected 'WITHOUT ROWID STRICT', got: {}", sql);
    }
}
