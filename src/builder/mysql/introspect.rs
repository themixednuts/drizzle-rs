use drizzle_core::error::{DrizzleError, Result};
use drizzle_migrations::{
    DiffOptions, Plan,
    mysql::{
        MySQLCatalogDefaults,
        introspect::{
            IntrospectionResult, RawCheckInfo, RawColumnInfo, RawDatabaseInfo, RawForeignKeyPart,
            RawIndexPart, RawIntrospection, RawPrimaryKeyPart, RawTableInfo, RawViewInfo,
            assemble_ddl,
        },
    },
    schema::Snapshot,
};
use mysql_common::{Row, prelude::FromValue};

pub(super) struct Catalog {
    snapshot: Snapshot,
    defaults: MySQLCatalogDefaults,
    database: String,
}

impl Catalog {
    pub(super) fn assemble(raw: RawIntrospection) -> Result<Self> {
        let database = raw.database.name.clone();
        let defaults = raw.database.catalog_defaults();
        let ddl =
            assemble_ddl(raw).map_err(|error| DrizzleError::external("MySQL catalog", error))?;
        let snapshot = Snapshot::MySQL(IntrospectionResult { ddl }.to_snapshot());
        Ok(Self {
            snapshot,
            defaults,
            database,
        })
    }

    pub(super) fn into_snapshot(self) -> Snapshot {
        self.snapshot
    }

    pub(super) fn plan(&self, desired: &Snapshot) -> Result<Plan> {
        let live = match (&self.snapshot, desired) {
            (Snapshot::MySQL(live), Snapshot::MySQL(desired)) => Snapshot::MySQL(
                live.prepare_for_push(desired, &self.database)
                    .map_err(|error| DrizzleError::external("MySQL push", error))?,
            ),
            _ => self.snapshot.clone(),
        };
        drizzle_migrations::diff_with(
            &live,
            desired,
            &DiffOptions::new().mysql_catalog_defaults(self.defaults.clone()),
        )
        .map_err(|error| DrizzleError::external("MySQL schema diff", error))
    }
}

fn value<T>(row: &Row, index: usize, field: &str) -> Result<T>
where
    T: FromValue,
{
    match row.get_opt::<T, _>(index) {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(DrizzleError::ConversionError(
            format!("MySQL returned an invalid value for {field}: {error}").into(),
        )),
        None => Err(DrizzleError::ConversionError(
            format!("MySQL result is missing required column {field}").into(),
        )),
    }
}

fn optional_string(row: &Row, index: usize, field: &str) -> Result<Option<String>> {
    value(row, index, field)
}

fn required_string(row: &Row, index: usize, field: &str) -> Result<String> {
    optional_string(row, index, field)?.ok_or_else(|| {
        DrizzleError::ConversionError(
            format!("MySQL returned NULL for required column {field}").into(),
        )
    })
}

fn required_u32(row: &Row, index: usize, field: &str) -> Result<u32> {
    let value = value::<u64>(row, index, field)?;
    u32::try_from(value).map_err(|_| {
        DrizzleError::ConversionError(
            format!("MySQL value {value} for {field} does not fit in a u32").into(),
        )
    })
}

fn boolean(value: &str, field: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "yes" | "true" => Ok(true),
        "0" | "no" | "false" => Ok(false),
        _ => Err(DrizzleError::ConversionError(
            format!("MySQL returned unsupported boolean value '{value}' for {field}").into(),
        )),
    }
}

fn required_boolean(row: &Row, index: usize, field: &str) -> Result<bool> {
    boolean(&required_string(row, index, field)?, field)
}

fn optional_boolean(row: &Row, index: usize, field: &str) -> Result<Option<bool>> {
    optional_string(row, index, field)?
        .map(|value| boolean(&value, field))
        .transpose()
}

pub(super) fn database(rows: Vec<Row>) -> Result<RawDatabaseInfo> {
    let mut rows = rows.into_iter();
    let row = rows
        .next()
        .ok_or_else(|| DrizzleError::Other("MySQL connection has no selected database".into()))?;
    if rows.next().is_some() {
        return Err(DrizzleError::ConversionError(
            "MySQL catalog query for the selected database returned multiple rows".into(),
        ));
    }
    Ok(RawDatabaseInfo {
        name: required_string(&row, 0, "schema name")?,
        default_engine: optional_string(&row, 1, "default storage engine")?,
        default_charset: optional_string(&row, 2, "default character set")?,
        default_collation: optional_string(&row, 3, "default collation")?,
    })
}

pub(super) fn tables(rows: Vec<Row>) -> Result<Vec<RawTableInfo>> {
    rows.iter()
        .map(|row| {
            Ok(RawTableInfo {
                database: required_string(row, 0, "TABLES.TABLE_SCHEMA")?,
                name: required_string(row, 1, "TABLES.TABLE_NAME")?,
                engine: optional_string(row, 2, "TABLES.ENGINE")?,
                charset: optional_string(row, 3, "TABLES.CHARACTER_SET_NAME")?,
                collation: optional_string(row, 4, "TABLES.TABLE_COLLATION")?,
                comment: optional_string(row, 5, "TABLES.TABLE_COMMENT")?,
            })
        })
        .collect()
}

pub(super) fn columns(rows: Vec<Row>) -> Result<Vec<RawColumnInfo>> {
    rows.iter()
        .map(|row| {
            Ok(RawColumnInfo {
                database: required_string(row, 0, "COLUMNS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "COLUMNS.TABLE_NAME")?,
                name: required_string(row, 2, "COLUMNS.COLUMN_NAME")?,
                column_type: required_string(row, 3, "COLUMNS.COLUMN_TYPE")?,
                nullable: required_boolean(row, 4, "COLUMNS.IS_NULLABLE")?,
                default_value: optional_string(row, 5, "COLUMNS.COLUMN_DEFAULT")?,
                extra: required_string(row, 6, "COLUMNS.EXTRA")?,
                generation_expression: optional_string(row, 7, "COLUMNS.GENERATION_EXPRESSION")?,
                charset: optional_string(row, 8, "COLUMNS.CHARACTER_SET_NAME")?,
                collation: optional_string(row, 9, "COLUMNS.COLLATION_NAME")?,
                comment: optional_string(row, 10, "COLUMNS.COLUMN_COMMENT")?,
                ordinal_position: required_u32(row, 11, "COLUMNS.ORDINAL_POSITION")?,
            })
        })
        .collect()
}

pub(super) fn indexes(rows: Vec<Row>) -> Result<Vec<RawIndexPart>> {
    rows.iter()
        .map(|row| {
            Ok(RawIndexPart {
                database: required_string(row, 0, "STATISTICS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "STATISTICS.TABLE_NAME")?,
                name: required_string(row, 2, "STATISTICS.INDEX_NAME")?,
                non_unique: value::<u64>(row, 3, "STATISTICS.NON_UNIQUE")? != 0,
                sequence: required_u32(row, 4, "STATISTICS.SEQ_IN_INDEX")?,
                column_name: optional_string(row, 5, "STATISTICS.COLUMN_NAME")?,
                expression: optional_string(row, 6, "STATISTICS.EXPRESSION")?,
                prefix_length: value::<Option<u64>>(row, 7, "STATISTICS.SUB_PART")?
                    .map(|value| {
                        u32::try_from(value).map_err(|_| {
                            DrizzleError::ConversionError(
                                format!(
                                    "MySQL value {value} for STATISTICS.SUB_PART does not fit in a u32"
                                )
                                .into(),
                            )
                        })
                    })
                    .transpose()?,
                collation: optional_string(row, 8, "STATISTICS.COLLATION")?,
                index_type: optional_string(row, 9, "STATISTICS.INDEX_TYPE")?,
                comment: optional_string(row, 10, "STATISTICS.INDEX_COMMENT")?,
                visible: optional_boolean(row, 11, "STATISTICS.IS_VISIBLE")?,
            })
        })
        .collect()
}

pub(super) fn primary_keys(rows: Vec<Row>) -> Result<Vec<RawPrimaryKeyPart>> {
    rows.iter()
        .map(|row| {
            Ok(RawPrimaryKeyPart {
                database: required_string(row, 0, "PRIMARY_KEYS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "PRIMARY_KEYS.TABLE_NAME")?,
                constraint_name: required_string(row, 2, "PRIMARY_KEYS.CONSTRAINT_NAME")?,
                column: required_string(row, 3, "PRIMARY_KEYS.COLUMN_NAME")?,
                ordinal_position: required_u32(row, 4, "PRIMARY_KEYS.ORDINAL_POSITION")?,
            })
        })
        .collect()
}

pub(super) fn foreign_keys(rows: Vec<Row>) -> Result<Vec<RawForeignKeyPart>> {
    rows.iter()
        .map(|row| {
            Ok(RawForeignKeyPart {
                database: required_string(row, 0, "FOREIGN_KEYS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "FOREIGN_KEYS.TABLE_NAME")?,
                name: required_string(row, 2, "FOREIGN_KEYS.CONSTRAINT_NAME")?,
                column: required_string(row, 3, "FOREIGN_KEYS.COLUMN_NAME")?,
                ordinal_position: required_u32(row, 4, "FOREIGN_KEYS.ORDINAL_POSITION")?,
                foreign_database: required_string(row, 5, "FOREIGN_KEYS.REFERENCED_TABLE_SCHEMA")?,
                foreign_table: required_string(row, 6, "FOREIGN_KEYS.REFERENCED_TABLE_NAME")?,
                foreign_column: required_string(row, 7, "FOREIGN_KEYS.REFERENCED_COLUMN_NAME")?,
                on_update: required_string(row, 8, "FOREIGN_KEYS.UPDATE_RULE")?,
                on_delete: required_string(row, 9, "FOREIGN_KEYS.DELETE_RULE")?,
            })
        })
        .collect()
}

pub(super) fn checks(rows: Vec<Row>) -> Result<Vec<RawCheckInfo>> {
    rows.iter()
        .map(|row| {
            Ok(RawCheckInfo {
                database: required_string(row, 0, "CHECKS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "CHECKS.TABLE_NAME")?,
                name: required_string(row, 2, "CHECKS.CONSTRAINT_NAME")?,
                expression: required_string(row, 3, "CHECKS.CHECK_CLAUSE")?,
                enforced: optional_boolean(row, 4, "CHECKS.ENFORCED")?,
            })
        })
        .collect()
}

pub(super) fn views(rows: Vec<Row>) -> Result<Vec<RawViewInfo>> {
    rows.iter()
        .map(|row| {
            Ok(RawViewInfo {
                database: required_string(row, 0, "VIEWS.TABLE_SCHEMA")?,
                name: required_string(row, 1, "VIEWS.TABLE_NAME")?,
                definition: required_string(row, 2, "VIEWS.VIEW_DEFINITION")?,
                algorithm: None,
                definer: optional_string(row, 3, "VIEWS.DEFINER")?,
                sql_security: optional_string(row, 4, "VIEWS.SECURITY_TYPE")?,
                check_option: optional_string(row, 5, "VIEWS.CHECK_OPTION")?,
                charset: optional_string(row, 6, "VIEWS.CHARACTER_SET_CLIENT")?,
                collation: optional_string(row, 7, "VIEWS.COLLATION_CONNECTION")?,
            })
        })
        .collect()
}

pub(super) fn view_sql(database: &str, view: &str) -> String {
    format!(
        "SHOW CREATE VIEW `{}`.`{}`",
        database.replace('`', "``"),
        view.replace('`', "``")
    )
}

pub(super) fn view_statement(rows: Vec<Row>, view: &str) -> Result<String> {
    let mut rows = rows.into_iter();
    let row = rows.next().ok_or_else(|| {
        DrizzleError::ConversionError(
            format!("SHOW CREATE VIEW returned no row for `{view}`").into(),
        )
    })?;
    if rows.next().is_some() {
        return Err(DrizzleError::ConversionError(
            format!("SHOW CREATE VIEW returned multiple rows for `{view}`").into(),
        ));
    }
    required_string(&row, 1, "SHOW CREATE VIEW statement")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysql_common::{constants::ColumnType, packets::Column, row::new_row, value::Value};
    use std::sync::Arc;

    fn row(values: Vec<Value>) -> Row {
        let columns = (0..values.len())
            .map(|_| Column::new(ColumnType::MYSQL_TYPE_VAR_STRING))
            .collect::<Vec<_>>();
        new_row(values, Arc::from(columns))
    }

    #[test]
    fn decoder_reports_missing_and_null_required_columns() {
        assert!(matches!(
            database(vec![row(Vec::new())]),
            Err(DrizzleError::ConversionError(_))
        ));
        assert!(matches!(
            database(vec![row(vec![
                Value::NULL,
                Value::NULL,
                Value::NULL,
                Value::NULL,
            ])]),
            Err(DrizzleError::ConversionError(_))
        ));
    }

    #[test]
    fn decoder_rejects_invalid_numeric_metadata() {
        assert!(matches!(
            required_u32(
                &row(vec![Value::Bytes(b"not-a-number".to_vec())]),
                0,
                "ordinal position",
            ),
            Err(DrizzleError::ConversionError(_))
        ));
    }
}
