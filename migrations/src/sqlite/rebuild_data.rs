//! Typed data movement for SQLite table rebuild migrations.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::collection::SQLiteDDL;
use super::ddl::Column;
use super::statements::{JsonStatement, RebuildTableData};

pub const SQLITE_REBUILD_DATA_PLAN_VERSION: u32 = 1;

/// Versioned data movement attached to one exact predecessor snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqliteRebuildDataPlanRegistry {
    pub version: u32,
    pub plans: Vec<SqliteRebuildDataPlan>,
}

/// Data movement selected only when its predecessor is the loaded snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqliteRebuildDataPlan {
    pub predecessor_snapshot_id: uuid::Uuid,
    pub tables: Vec<SqliteTableRebuildPlan>,
}

impl SqliteRebuildDataPlanRegistry {
    #[must_use]
    pub fn single(plan: SqliteRebuildDataPlan) -> Self {
        Self {
            version: SQLITE_REBUILD_DATA_PLAN_VERSION,
            plans: vec![plan],
        }
    }
}

/// Data movement and validation for one table rebuilt by the schema diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqliteTableRebuildPlan {
    pub table: String,
    #[serde(default)]
    pub columns: Vec<SqliteColumnCopy>,
    #[serde(default)]
    pub validations: Vec<SqliteDataValidation>,
}

/// One target column whose copied value differs from an identity projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqliteColumnCopy {
    pub target: String,
    pub expression: SqliteCopyExpression,
}

/// Closed, generator-owned expressions admitted in a rebuild copy projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SqliteCopyExpression {
    HexTextToBlob {
        source: String,
        bytes: usize,
    },
    IntegerMap {
        source: String,
        cases: Vec<SqliteIntegerMapping>,
    },
}

/// One exact integer remap in a [`SqliteCopyExpression::IntegerMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqliteIntegerMapping {
    pub from: i64,
    pub to: i64,
}

/// Closed, generator-owned source-data invariants checked before rebuilding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SqliteDataValidation {
    JsonValid { column: String },
    IntegerSet { column: String, allowed: Vec<i64> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SqliteAffinity {
    Integer,
    Text,
    Blob,
    Real,
    Numeric,
}

pub(crate) fn apply_rebuild_data_plan(
    predecessor_snapshot_id: &str,
    previous: &SQLiteDDL,
    current: &SQLiteDDL,
    statements: &mut [JsonStatement],
    registry: Option<&SqliteRebuildDataPlanRegistry>,
) -> Result<(), String> {
    let affinity_changes = changed_affinities(previous, current);
    let plan = select_plan(predecessor_snapshot_id, registry)?;
    if plan.is_none() && affinity_changes.is_empty() {
        return Ok(());
    }
    let plan = plan.ok_or_else(|| {
        format!(
            "SQLite columns change storage affinity without a rebuild-data plan: {}",
            format_columns(&affinity_changes)
        )
    })?;
    let mut plans = BTreeMap::new();
    for table in &plan.tables {
        if table.table.trim().is_empty() {
            return Err("SQLite rebuild-data plan contains an empty table name".to_string());
        }
        if plans.insert(table.table.clone(), table).is_some() {
            return Err(format!(
                "SQLite rebuild-data plan contains duplicate table `{}`",
                table.table
            ));
        }
    }

    let mut consumed = BTreeSet::new();
    let mut handled_affinity_changes = BTreeSet::new();
    for statement in statements {
        let JsonStatement::RecreateTable(recreate) = statement else {
            continue;
        };
        let table_name = recreate.to.name.as_str();
        let table_plan = plans.get(table_name);
        let required = affinity_changes
            .iter()
            .filter(|(table, _)| table == table_name)
            .map(|(_, column)| column.as_str())
            .collect::<BTreeSet<_>>();
        if table_plan.is_none() && required.is_empty() {
            continue;
        }
        let table_plan = table_plan.ok_or_else(|| {
            format!(
                "SQLite table `{table_name}` changes storage affinity for {} without a table rebuild-data plan",
                required.iter().copied().collect::<Vec<_>>().join(", ")
            )
        })?;
        recreate.data = Some(validate_table_plan(
            table_plan,
            &recreate.from.columns,
            &recreate.to.columns,
            &required,
        )?);
        handled_affinity_changes.extend(
            required
                .iter()
                .map(|column| (table_name.to_string(), (*column).to_string())),
        );
        consumed.insert(table_name.to_string());
    }

    let unhandled = affinity_changes
        .difference(&handled_affinity_changes)
        .cloned()
        .collect::<BTreeSet<_>>();
    if !unhandled.is_empty() {
        return Err(format!(
            "SQLite affinity changes were not attached to a table rebuild: {}",
            format_columns(&unhandled)
        ));
    }

    let unused = plans
        .keys()
        .filter(|table| !consumed.contains(*table))
        .cloned()
        .collect::<Vec<_>>();
    if !unused.is_empty() {
        return Err(format!(
            "SQLite rebuild-data plan contains unused tables: {}",
            unused.join(", ")
        ));
    }
    Ok(())
}

fn select_plan<'a>(
    predecessor_snapshot_id: &str,
    registry: Option<&'a SqliteRebuildDataPlanRegistry>,
) -> Result<Option<&'a SqliteRebuildDataPlan>, String> {
    let Some(registry) = registry else {
        return Ok(None);
    };
    if registry.version != SQLITE_REBUILD_DATA_PLAN_VERSION {
        return Err(format!(
            "unsupported SQLite rebuild-data plan registry version {}; expected {}",
            registry.version, SQLITE_REBUILD_DATA_PLAN_VERSION
        ));
    }
    let predecessor_snapshot_id = uuid::Uuid::parse_str(predecessor_snapshot_id).map_err(|error| {
        format!(
            "loaded SQLite predecessor snapshot ID `{predecessor_snapshot_id}` is not a UUID: {error}"
        )
    })?;
    let mut plans = BTreeMap::new();
    for plan in &registry.plans {
        if plans.insert(plan.predecessor_snapshot_id, plan).is_some() {
            return Err(format!(
                "SQLite rebuild-data plan registry repeats predecessor `{}`",
                plan.predecessor_snapshot_id
            ));
        }
    }
    Ok(plans.get(&predecessor_snapshot_id).copied())
}

fn validate_table_plan(
    plan: &SqliteTableRebuildPlan,
    from: &[Column],
    to: &[Column],
    required: &BTreeSet<&str>,
) -> Result<RebuildTableData, String> {
    let from_columns = from
        .iter()
        .map(|column| (column.name.to_string(), column))
        .collect::<BTreeMap<_, _>>();
    let to_columns = to
        .iter()
        .map(|column| (column.name.to_string(), column))
        .collect::<BTreeMap<_, _>>();
    let mut copies = BTreeMap::new();
    let mut derived_validations = Vec::new();

    for copy in &plan.columns {
        let target = to_columns.get(&copy.target).ok_or_else(|| {
            format!(
                "SQLite rebuild-data plan for `{}` targets missing current column `{}`",
                plan.table, copy.target
            )
        })?;
        if target.generated.is_some() {
            return Err(format!(
                "SQLite rebuild-data plan for `{}` cannot map generated target column `{}`",
                plan.table, copy.target
            ));
        }
        if copies.contains_key(&copy.target) {
            return Err(format!(
                "SQLite rebuild-data plan for `{}` maps target column `{}` more than once",
                plan.table, copy.target
            ));
        }
        let expression = match &copy.expression {
            SqliteCopyExpression::HexTextToBlob { source, bytes } => {
                let source_column = source_column(&plan.table, source, &from_columns)?;
                if *bytes == 0 {
                    return Err(format!(
                        "SQLite HexTextToBlob mapping for `{}.{}` requires a positive byte length",
                        plan.table, copy.target
                    ));
                }
                require_affinities(
                    &plan.table,
                    &copy.target,
                    affinity(&source_column.sql_type),
                    SqliteAffinity::Text,
                    affinity(&target.sql_type),
                    SqliteAffinity::Blob,
                )?;
                derived_validations.push(super::statements::RebuildDataValidation::HexText {
                    column: source.clone(),
                    bytes: *bytes,
                });
                super::statements::RebuildCopyExpression::HexTextToBlob {
                    source: source.clone(),
                }
            }
            SqliteCopyExpression::IntegerMap { source, cases } => {
                let source_column = source_column(&plan.table, source, &from_columns)?;
                require_affinities(
                    &plan.table,
                    &copy.target,
                    affinity(&source_column.sql_type),
                    SqliteAffinity::Integer,
                    affinity(&target.sql_type),
                    SqliteAffinity::Integer,
                )?;
                if cases.is_empty() {
                    return Err(format!(
                        "SQLite IntegerMap for `{}.{}` has no cases",
                        plan.table, copy.target
                    ));
                }
                let mut normalized = BTreeMap::new();
                for case in cases {
                    if normalized.insert(case.from, case.to).is_some() {
                        return Err(format!(
                            "SQLite IntegerMap for `{}.{}` repeats source value {}",
                            plan.table, copy.target, case.from
                        ));
                    }
                }
                derived_validations.push(super::statements::RebuildDataValidation::IntegerSet {
                    column: source.clone(),
                    allowed: normalized.keys().copied().collect(),
                });
                super::statements::RebuildCopyExpression::IntegerMap {
                    source: source.clone(),
                    cases: normalized.into_iter().collect(),
                }
            }
        };
        copies.insert(copy.target.clone(), expression);
    }

    let missing = required
        .iter()
        .filter(|column| !copies.contains_key(**column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "SQLite rebuild-data plan for `{}` omits affinity-changing columns: {}",
            plan.table,
            missing.join(", ")
        ));
    }

    let mut validations = BTreeSet::new();
    for validation in derived_validations {
        validations.insert(validation);
    }
    for validation in &plan.validations {
        let normalized = match validation {
            SqliteDataValidation::JsonValid { column } => {
                let source = source_column(&plan.table, column, &from_columns)?;
                if affinity(&source.sql_type) != SqliteAffinity::Text {
                    return Err(format!(
                        "SQLite JsonValid validation requires TEXT source `{}.{column}`",
                        plan.table
                    ));
                }
                validation_target(
                    &plan.table,
                    column,
                    &to_columns,
                    SqliteAffinity::Text,
                    "JsonValid",
                )?;
                super::statements::RebuildDataValidation::JsonValid {
                    column: column.clone(),
                }
            }
            SqliteDataValidation::IntegerSet { column, allowed } => {
                let source = source_column(&plan.table, column, &from_columns)?;
                if affinity(&source.sql_type) != SqliteAffinity::Integer {
                    return Err(format!(
                        "SQLite IntegerSet validation requires INTEGER source `{}.{column}`",
                        plan.table
                    ));
                }
                validation_target(
                    &plan.table,
                    column,
                    &to_columns,
                    SqliteAffinity::Integer,
                    "IntegerSet",
                )?;
                let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
                if allowed.is_empty() {
                    return Err(format!(
                        "SQLite IntegerSet validation for `{}.{column}` has no allowed values",
                        plan.table
                    ));
                }
                super::statements::RebuildDataValidation::IntegerSet {
                    column: column.clone(),
                    allowed: allowed.into_iter().collect(),
                }
            }
        };
        validations.insert(normalized);
    }

    Ok(RebuildTableData {
        copies,
        validations: validations.into_iter().collect(),
    })
}

fn validation_target(
    table: &str,
    name: &str,
    columns: &BTreeMap<String, &Column>,
    expected_affinity: SqliteAffinity,
    validation: &str,
) -> Result<(), String> {
    let column = columns.get(name).copied().ok_or_else(|| {
        format!("SQLite {validation} validation protects removed column `{table}.{name}`")
    })?;
    if column.generated.is_some() {
        return Err(format!(
            "SQLite {validation} validation cannot target generated current column `{table}.{name}`"
        ));
    }
    if affinity(&column.sql_type) != expected_affinity {
        return Err(format!(
            "SQLite {validation} validation target `{table}.{name}` has incompatible {} affinity",
            column.sql_type
        ));
    }
    Ok(())
}

fn source_column<'a>(
    table: &str,
    name: &str,
    columns: &'a BTreeMap<String, &'a Column>,
) -> Result<&'a Column, String> {
    let column = columns.get(name).copied().ok_or_else(|| {
        format!("SQLite rebuild-data plan references missing predecessor column `{table}.{name}`")
    })?;
    if column.generated.is_some() {
        return Err(format!(
            "SQLite rebuild-data plan cannot read generated predecessor column `{table}.{name}`"
        ));
    }
    Ok(column)
}

fn require_affinities(
    table: &str,
    target: &str,
    actual_source: SqliteAffinity,
    expected_source: SqliteAffinity,
    actual_target: SqliteAffinity,
    expected_target: SqliteAffinity,
) -> Result<(), String> {
    if actual_source != expected_source || actual_target != expected_target {
        return Err(format!(
            "SQLite rebuild-data mapping for `{table}.{target}` requires {expected_source:?}->{expected_target:?}, found {actual_source:?}->{actual_target:?}"
        ));
    }
    Ok(())
}

fn changed_affinities(previous: &SQLiteDDL, current: &SQLiteDDL) -> BTreeSet<(String, String)> {
    let mut changed = BTreeSet::new();
    for current_column in current.columns.list() {
        let Some(previous_column) = previous
            .columns
            .one(&current_column.table, &current_column.name)
        else {
            continue;
        };
        if previous_column.generated.is_none()
            && current_column.generated.is_none()
            && affinity(&previous_column.sql_type) != affinity(&current_column.sql_type)
        {
            changed.insert((
                current_column.table.to_string(),
                current_column.name.to_string(),
            ));
        }
    }
    changed
}

fn affinity(sql_type: &str) -> SqliteAffinity {
    let sql_type = sql_type.to_ascii_uppercase();
    if sql_type.contains("INT") {
        SqliteAffinity::Integer
    } else if sql_type.contains("CHAR") || sql_type.contains("CLOB") || sql_type.contains("TEXT") {
        SqliteAffinity::Text
    } else if sql_type.contains("BLOB") || sql_type.trim().is_empty() {
        SqliteAffinity::Blob
    } else if sql_type.contains("REAL") || sql_type.contains("FLOA") || sql_type.contains("DOUB") {
        SqliteAffinity::Real
    } else {
        SqliteAffinity::Numeric
    }
}

fn format_columns(columns: &BTreeSet<(String, String)>) -> String {
    columns
        .iter()
        .map(|(table, column)| format!("{table}.{column}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::ddl::{ColumnDef, TableDef};
    use crate::sqlite::diff::compute_migration;

    const PREDECESSOR: &str = "11111111-1111-4111-8111-111111111111";
    const OTHER: &str = "22222222-2222-4222-8222-222222222222";

    fn uuid(value: &str) -> uuid::Uuid {
        uuid::Uuid::parse_str(value).expect("valid fixture UUID")
    }

    fn schemas() -> (SQLiteDDL, SQLiteDDL) {
        let mut previous = SQLiteDDL::new();
        previous.tables.push(TableDef::new("assets").into_table());
        previous
            .columns
            .push(ColumnDef::new("assets", "digest", "text").into_column());

        let mut current = SQLiteDDL::new();
        current
            .tables
            .push(TableDef::new("assets").strict().into_table());
        current
            .columns
            .push(ColumnDef::new("assets", "digest", "blob").into_column());
        (previous, current)
    }

    fn registry(plan: SqliteRebuildDataPlan) -> SqliteRebuildDataPlanRegistry {
        SqliteRebuildDataPlanRegistry::single(plan)
    }

    #[test]
    fn affinity_change_requires_an_explicit_mapping() {
        let (previous, current) = schemas();
        let mut migration = compute_migration(&previous, &current);
        let error = apply_rebuild_data_plan(
            PREDECESSOR,
            &previous,
            &current,
            &mut migration.statements,
            None,
        )
        .expect_err("affinity change must fail without a plan");
        assert!(error.contains("assets.digest"), "{error}");
    }

    #[test]
    fn plan_is_bound_to_the_exact_predecessor_snapshot() {
        let (previous, current) = schemas();
        let mut migration = compute_migration(&previous, &current);
        let plan = SqliteRebuildDataPlan {
            predecessor_snapshot_id: uuid(OTHER),
            tables: Vec::new(),
        };
        let registry = registry(plan);
        let error = apply_rebuild_data_plan(
            PREDECESSOR,
            &previous,
            &current,
            &mut migration.statements,
            Some(&registry),
        )
        .expect_err("wrong predecessor must fail");
        assert!(error.contains("assets.digest"), "{error}");
    }

    #[test]
    fn unused_table_plan_is_rejected() {
        let (previous, _) = schemas();
        let current = previous.clone();
        let mut migration = compute_migration(&previous, &current);
        let plan = SqliteRebuildDataPlan {
            predecessor_snapshot_id: uuid(PREDECESSOR),
            tables: vec![SqliteTableRebuildPlan {
                table: "missing".to_string(),
                columns: Vec::new(),
                validations: Vec::new(),
            }],
        };
        let registry = registry(plan);
        let error = apply_rebuild_data_plan(
            PREDECESSOR,
            &previous,
            &current,
            &mut migration.statements,
            Some(&registry),
        )
        .expect_err("unused table must fail");
        assert!(error.contains("unused tables: missing"), "{error}");
    }

    #[test]
    fn every_affinity_change_must_attach_to_a_rebuild_statement() {
        let (previous, current) = schemas();
        let plan = SqliteRebuildDataPlan {
            predecessor_snapshot_id: uuid(PREDECESSOR),
            tables: vec![SqliteTableRebuildPlan {
                table: "assets".to_string(),
                columns: vec![SqliteColumnCopy {
                    target: "digest".to_string(),
                    expression: SqliteCopyExpression::HexTextToBlob {
                        source: "digest".to_string(),
                        bytes: 32,
                    },
                }],
                validations: Vec::new(),
            }],
        };
        let registry = registry(plan);
        let error =
            apply_rebuild_data_plan(PREDECESSOR, &previous, &current, &mut [], Some(&registry))
                .expect_err("unattached affinity change must fail");
        assert!(error.contains("not attached"), "{error}");
    }

    #[test]
    fn historical_registry_entry_is_inert_when_the_schema_is_unchanged() {
        let (previous, _) = schemas();
        let current = previous.clone();
        let registry = registry(SqliteRebuildDataPlan {
            predecessor_snapshot_id: uuid(OTHER),
            tables: vec![SqliteTableRebuildPlan {
                table: "assets".to_string(),
                columns: Vec::new(),
                validations: Vec::new(),
            }],
        });

        apply_rebuild_data_plan(PREDECESSOR, &previous, &current, &mut [], Some(&registry))
            .expect("historical plans must not poison later no-change generation");
    }

    #[test]
    fn generated_copy_targets_and_sources_are_rejected() {
        let from = vec![
            ColumnDef::new("assets", "source", "text")
                .generated_stored("'00'")
                .into(),
        ];
        let to = vec![
            ColumnDef::new("assets", "digest", "blob")
                .generated_stored("x'00'")
                .into(),
        ];
        let plan = SqliteTableRebuildPlan {
            table: "assets".to_string(),
            columns: vec![SqliteColumnCopy {
                target: "digest".to_string(),
                expression: SqliteCopyExpression::HexTextToBlob {
                    source: "source".to_string(),
                    bytes: 1,
                },
            }],
            validations: Vec::new(),
        };
        let error = validate_table_plan(&plan, &from, &to, &BTreeSet::from(["digest"]))
            .expect_err("generated target must fail");
        assert!(error.contains("generated target"), "{error}");

        let plain_target = vec![ColumnDef::new("assets", "digest", "blob").into_column()];
        let error = validate_table_plan(&plan, &from, &plain_target, &BTreeSet::from(["digest"]))
            .expect_err("generated source must fail");
        assert!(error.contains("generated predecessor"), "{error}");
    }

    #[test]
    fn json_validation_requires_text_storage() {
        let from = vec![ColumnDef::new("assets", "metadata", "integer").into_column()];
        let to = from.clone();
        let plan = SqliteTableRebuildPlan {
            table: "assets".to_string(),
            columns: Vec::new(),
            validations: vec![SqliteDataValidation::JsonValid {
                column: "metadata".to_string(),
            }],
        };
        let error = validate_table_plan(&plan, &from, &to, &BTreeSet::new())
            .expect_err("non-text JSON validation must fail");
        assert!(error.contains("requires TEXT"), "{error}");
    }

    #[test]
    fn validations_must_protect_a_copied_current_column() {
        let from = vec![ColumnDef::new("assets", "metadata", "text").into_column()];
        let validation = SqliteTableRebuildPlan {
            table: "assets".to_string(),
            columns: Vec::new(),
            validations: vec![SqliteDataValidation::JsonValid {
                column: "metadata".to_string(),
            }],
        };

        let error = validate_table_plan(&validation, &from, &[], &BTreeSet::new())
            .expect_err("removed validation target must fail");
        assert!(error.contains("protects removed column"), "{error}");

        let generated = vec![
            ColumnDef::new("assets", "metadata", "text")
                .generated_stored("'[]'")
                .into_column(),
        ];
        let error = validate_table_plan(&validation, &from, &generated, &BTreeSet::new())
            .expect_err("generated validation target must fail");
        assert!(error.contains("generated current column"), "{error}");

        let incompatible = vec![ColumnDef::new("assets", "metadata", "integer").into_column()];
        let error = validate_table_plan(&validation, &from, &incompatible, &BTreeSet::new())
            .expect_err("incompatible validation target must fail");
        assert!(error.contains("incompatible integer affinity"), "{error}");
    }
}
