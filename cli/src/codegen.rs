//! Rust schema source from a snapshot, shared by `introspect` / `pull` and
//! `import`.

use crate::config::IntrospectCasing;
use crate::error::CliError;
use drizzle_migrations::schema::Snapshot;

/// Generated schema source plus counts for the command summary.
#[derive(Debug, Clone, Default)]
pub struct SchemaCode {
    /// The Rust source.
    pub code: String,
    /// Number of tables written.
    pub table_count: usize,
    /// Number of indexes written.
    pub index_count: usize,
    /// Number of views written.
    pub view_count: usize,
    /// Objects the schema could not express, and similar notes.
    pub warnings: Vec<String>,
}

/// Options for [`schema_code`].
#[derive(Debug, Clone)]
pub struct SchemaCodeOptions<'a> {
    /// Field casing; `None` writes snake_case fields.
    pub casing: Option<IntrospectCasing>,
    /// Line added to the module doc comment.
    pub module_doc: &'a str,
    /// Name of the generated `#[derive(...Schema)]` struct.
    pub schema_name: &'a str,
}

/// Generates the Rust schema for `snapshot` with the dialect's codegen.
///
/// # Errors
///
/// Returns [`CliError::Other`] if the MySQL codegen refuses a snapshot it
/// cannot write without losing information.
pub fn schema_code(
    snapshot: &Snapshot,
    options: &SchemaCodeOptions<'_>,
) -> Result<SchemaCode, CliError> {
    // Each dialect's codegen has its own (identical) `FieldCasing` enum.
    macro_rules! field_casing {
        ($casing:ty) => {
            match options.casing {
                Some(IntrospectCasing::Camel) => <$casing>::Camel,
                Some(IntrospectCasing::Preserve) => <$casing>::Preserve,
                None => <$casing>::Snake,
            }
        };
    }

    match snapshot {
        Snapshot::Sqlite(snap) => {
            use drizzle_migrations::sqlite::SQLiteDDL;
            use drizzle_migrations::sqlite::codegen::{CodegenOptions, generate_rust_schema};

            let ddl = SQLiteDDL::from_entities(snap.ddl.clone());
            let generated = generate_rust_schema(
                &ddl,
                &CodegenOptions {
                    module_doc: Some(options.module_doc.into()),
                    include_schema: true,
                    schema_name: options.schema_name.into(),
                    use_pub: true,
                    field_casing: field_casing!(drizzle_migrations::sqlite::codegen::FieldCasing),
                },
            );
            Ok(SchemaCode {
                code: generated.code,
                table_count: generated.tables.len(),
                index_count: generated.indexes.len(),
                view_count: ddl.views.list().len(),
                warnings: generated.warnings,
            })
        }
        Snapshot::Postgres(snap) => {
            use drizzle_migrations::postgres::PostgresDDL;
            use drizzle_migrations::postgres::codegen::{CodegenOptions, generate_rust_schema};

            let ddl = PostgresDDL::from_entities(snap.ddl.clone());
            let generated = generate_rust_schema(
                &ddl,
                &CodegenOptions {
                    module_doc: Some(options.module_doc.into()),
                    include_schema: true,
                    schema_name: options.schema_name.into(),
                    use_pub: true,
                    field_casing: field_casing!(drizzle_migrations::postgres::codegen::FieldCasing),
                },
            );
            Ok(SchemaCode {
                code: generated.code,
                table_count: generated.tables.len(),
                index_count: generated.indexes.len(),
                view_count: generated.views.len(),
                warnings: generated.warnings,
            })
        }
        Snapshot::MySQL(snap) => {
            use drizzle_migrations::mysql::MySQLDDL;
            use drizzle_migrations::mysql::codegen::{CodegenOptions, generate_rust_schema};

            let ddl = MySQLDDL::from_entities(snap.ddl.clone());
            let generated = generate_rust_schema(
                &ddl,
                &CodegenOptions {
                    module_doc: Some(options.module_doc.into()),
                    include_schema: true,
                    schema_name: options.schema_name.into(),
                    use_pub: true,
                    field_casing: field_casing!(drizzle_migrations::mysql::codegen::FieldCasing),
                },
            )
            .map_err(|e| CliError::Other(format!("Cannot generate the MySQL schema: {e}")))?;
            Ok(SchemaCode {
                code: generated.code,
                table_count: generated.tables.len(),
                index_count: generated.indexes.len(),
                view_count: generated.views.len(),
                warnings: generated.warnings,
            })
        }
    }
}
