use super::attributes::TableAttributes;
use crate::postgres::field::{FieldInfo, PostgreSQLFlag, PostgreSQLType};

/// Generate PostgreSQL CREATE TABLE SQL statement
pub(super) fn generate_create_table_sql(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    attrs: &TableAttributes,
) -> String {
    let mut sql = String::new();

    // Table creation clause with PostgreSQL-specific options
    sql.push_str("CREATE ");
    if attrs.unlogged {
        sql.push_str("UNLOGGED ");
    }
    if attrs.temporary {
        sql.push_str("TEMPORARY ");
    }
    sql.push_str("TABLE ");
    // Quote table name for PostgreSQL
    sql.push('"');
    sql.push_str(table_name);
    sql.push_str("\" (");

    // Column definitions
    let mut column_defs = Vec::new();
    let mut primary_key_columns = Vec::new();

    for field_info in field_infos {
        let mut column_def = String::new();

        // Column name (quoted for PostgreSQL)
        column_def.push('"');
        column_def.push_str(&field_info.ident.to_string());
        column_def.push_str("\" ");

        // Column type
        match &field_info.column_type {
            PostgreSQLType::Enum(enum_name) => {
                column_def.push_str(enum_name);
            }
            _ => {
                column_def.push_str(field_info.column_type.to_sql_type());
            }
        }

        // Constraints
        for flag in &field_info.flags {
            match flag {
                PostgreSQLFlag::Primary if !is_composite_pk => {
                    column_def.push_str(" PRIMARY KEY");
                }
                PostgreSQLFlag::Primary if is_composite_pk => {
                    primary_key_columns.push(format!("\"{}\"", field_info.ident));
                }
                PostgreSQLFlag::Unique => {
                    column_def.push_str(" UNIQUE");
                }
                PostgreSQLFlag::NotNull => {
                    column_def.push_str(" NOT NULL");
                }
                PostgreSQLFlag::GeneratedIdentity => {
                    column_def.push_str(" GENERATED ALWAYS AS IDENTITY");
                }
                PostgreSQLFlag::Check(constraint) => {
                    column_def.push_str(&format!(" CHECK ({})", constraint));
                }
                _ => {} // Other flags handled elsewhere
            }
        }

        // Default values
        if let Some(default) = &field_info.default {
            match default {
                crate::postgres::field::PostgreSQLDefault::Literal(lit) => {
                    column_def.push_str(&format!(" DEFAULT '{}'", lit));
                }
                crate::postgres::field::PostgreSQLDefault::Function(func) => {
                    column_def.push_str(&format!(" DEFAULT {}", func));
                }
                crate::postgres::field::PostgreSQLDefault::Expression(_) => {
                    // Expression defaults need to be handled at runtime
                    column_def.push_str(" DEFAULT NULL");
                }
            }
        }

        // NULL/NOT NULL based on field type
        if !field_info.is_nullable && !field_info.flags.contains(&PostgreSQLFlag::NotNull) {
            // Add NOT NULL if field is not nullable and not already specified
            if !field_info.is_serial {
                // SERIAL columns are automatically NOT NULL
                column_def.push_str(" NOT NULL");
            }
        }

        // Foreign key constraints (with quoted identifiers)
        if let Some(fk) = &field_info.foreign_key {
            column_def.push_str(&format!(" REFERENCES \"{}\"(\"{}\")", fk.table, fk.column));
            if let Some(on_delete) = &fk.on_delete {
                column_def.push_str(&format!(" ON DELETE {}", on_delete));
            }
            if let Some(on_update) = &fk.on_update {
                column_def.push_str(&format!(" ON UPDATE {}", on_update));
            }
        }

        column_defs.push(column_def);
    }

    // Add composite primary key constraint
    if is_composite_pk && !primary_key_columns.is_empty() {
        column_defs.push(format!("PRIMARY KEY ({})", primary_key_columns.join(", ")));
    }

    sql.push_str(&column_defs.join(", "));
    sql.push(')');

    // Table inheritance
    if let Some(parent) = &attrs.inherits {
        sql.push_str(&format!(" INHERITS ({})", parent));
    }

    // Tablespace
    if let Some(tablespace) = &attrs.tablespace {
        sql.push_str(&format!(" TABLESPACE {}", tablespace));
    }

    sql
}
