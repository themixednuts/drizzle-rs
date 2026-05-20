//! `PostgreSQL` const DDL generation.
//!
//! Builds the `CREATE TABLE` SQL stored on each table's `SQLSchema::SQL` const.
//! The generated SQL uses the same identifier quoting, constraint shape, and
//! referential-action ordering as the runtime DDL renderer.
//!
//! Foreign-key targets are assembled with `concatcp!(...)` so referenced table
//! names remain compile-time constants.

use super::context::MacroContext;
use crate::paths::ddl::postgres as ddl_paths;
use crate::paths::{core as core_paths, postgres as postgres_paths};
use crate::postgres::field::{FieldInfo, PostgreSQLDefault};
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Write;
use syn::Ident;

/// A single piece of a `concatcp!`-emitted CREATE TABLE statement.
///
/// `TableNameOf` expands to `<RefTable>::TABLE_NAME` so referenced-table
/// names resolve at compile time; everything else is a plain literal.
enum DdlPiece {
    Literal(String),
    TableNameOf(Ident),
}

impl DdlPiece {
    fn to_token(&self) -> TokenStream {
        match self {
            Self::Literal(s) => quote! { #s },
            Self::TableNameOf(ident) => quote! { <#ident>::TABLE_NAME },
        }
    }
}

/// Format a `"schema"."table"` prefix (empty when schema is `"public"`).
fn schema_prefix(schema: &str) -> String {
    if schema == "public" {
        String::new()
    } else {
        format!("\"{schema}\".")
    }
}

/// Generate a compile-time `const SQL: &'static str` value for `SQLSchema`.
///
/// Output mirrors `TableSql::create_table_sql()`: double-quoted identifiers,
/// schema prefix only when not `public`, named `CONSTRAINT` clauses for every
/// table-level constraint, `ON UPDATE` before `ON DELETE`, `NO ACTION`
/// skipped, trailing `;`.
pub fn generate_schema_sql_const(ctx: &MacroContext) -> TokenStream {
    let tokens: Vec<TokenStream> = build_create_table_pieces(ctx)
        .iter()
        .map(DdlPiece::to_token)
        .collect();
    let const_format = crate::common::paths::const_format();
    quote! {
        #const_format::concatcp!(#(#tokens),*)
    }
}

/// Build the CREATE TABLE statement as `DdlPiece`s ready for `concatcp!`.
///
/// Lines are joined by literal `",\n"` prefixes on every non-first line so
/// that the FK case can interleave `TableNameOf` pieces between literal
/// fragments without losing the column-separator commas.
fn build_create_table_pieces(ctx: &MacroContext) -> Vec<DdlPiece> {
    let table_name = &ctx.table_name;
    let schema_name = ctx.attrs.schema.as_deref().unwrap_or("public");
    let field_infos = ctx.field_infos;

    let mut pieces: Vec<DdlPiece> = Vec::new();
    pieces.push(DdlPiece::Literal(format!(
        "CREATE TABLE {prefix}\"{table_name}\" (\n",
        prefix = schema_prefix(schema_name)
    )));

    let mut lines: Vec<Vec<DdlPiece>> = Vec::new();

    // Columns
    for field in field_infos {
        lines.push(vec![DdlPiece::Literal(format!(
            "\t{}",
            column_to_sql(field)
        ))]);
    }

    // Primary key (always at table level for Postgres, matching TableSql).
    // The non-explicit (macro-generated) form omits the CONSTRAINT name.
    let pk_columns: Vec<&String> = field_infos
        .iter()
        .filter(|f| f.is_primary())
        .map(|f| &f.column_name)
        .collect();
    if !pk_columns.is_empty() {
        let cols = pk_columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(vec![DdlPiece::Literal(format!("\tPRIMARY KEY({cols})"))]);
    }

    // Single-column foreign keys
    for field in field_infos {
        if let Some(ref fk) = field.foreign_key {
            let fk_name = format!("{}_{}_fkey", table_name, field.column_name);
            let ref_column = fk.column.to_string();
            let mut line = Vec::new();
            line.push(DdlPiece::Literal(format!(
                "\tCONSTRAINT \"{}\" FOREIGN KEY (\"{}\") REFERENCES \"",
                fk_name, field.column_name
            )));
            line.push(DdlPiece::TableNameOf(fk.table.clone()));
            let mut suffix = format!("\"(\"{ref_column}\")");
            // Match TableSql ordering: ON UPDATE then ON DELETE, NO ACTION skipped.
            if let Some(ref on_update) = fk.on_update {
                let action = on_update.to_uppercase();
                if action != "NO ACTION" {
                    let _ = write!(suffix, " ON UPDATE {action}");
                }
            }
            if let Some(ref on_delete) = fk.on_delete {
                let action = on_delete.to_uppercase();
                if action != "NO ACTION" {
                    let _ = write!(suffix, " ON DELETE {action}");
                }
            }
            line.push(DdlPiece::Literal(suffix));
            lines.push(line);
        }
    }

    // Composite foreign keys
    for fk in &ctx.attrs.composite_foreign_keys {
        let source_cols: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| &f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let target_cols: Vec<String> = fk
            .target_columns
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let fk_name = format!("{}_{}_fkey", table_name, source_cols.join("_"));
        let src_str = source_cols
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let tgt_str = target_cols
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let mut line = Vec::new();
        line.push(DdlPiece::Literal(format!(
            "\tCONSTRAINT \"{fk_name}\" FOREIGN KEY ({src_str}) REFERENCES \""
        )));
        line.push(DdlPiece::TableNameOf(fk.target_table.clone()));
        let mut suffix = format!("\"({tgt_str})");
        if let Some(ref on_update) = fk.on_update {
            let action = on_update.to_uppercase();
            if action != "NO ACTION" {
                let _ = write!(suffix, " ON UPDATE {action}");
            }
        }
        if let Some(ref on_delete) = fk.on_delete {
            let action = on_delete.to_uppercase();
            if action != "NO ACTION" {
                let _ = write!(suffix, " ON DELETE {action}");
            }
        }
        line.push(DdlPiece::Literal(suffix));
        lines.push(line);
    }

    // Unique constraints (always at table level for Postgres)
    for field in field_infos.iter().filter(|f| f.is_unique()) {
        let uq_name = format!("{}_{}_unique", table_name, field.column_name);
        lines.push(vec![DdlPiece::Literal(format!(
            "\tCONSTRAINT \"{}\" UNIQUE(\"{}\")",
            uq_name, field.column_name
        ))]);
    }

    // Check constraints
    for field in field_infos {
        if let Some(ref check) = field.check_constraint {
            let chk_name = format!("{}_{}_check", table_name, field.column_name);
            lines.push(vec![DdlPiece::Literal(format!(
                "\tCONSTRAINT \"{chk_name}\" CHECK ({check})"
            ))]);
        }
    }

    // Join lines with ",\n" by prepending it to the first piece of each
    // non-first line. If the first piece happens to be a `TableNameOf`
    // (shouldn't happen for our current shape, but handle defensively),
    // we insert a standalone separator instead.
    for (i, mut line) in lines.into_iter().enumerate() {
        if i == 0 {
            pieces.append(&mut line);
            continue;
        }
        match line.first_mut() {
            Some(DdlPiece::Literal(s)) => *s = format!(",\n{s}"),
            _ => pieces.push(DdlPiece::Literal(",\n".to_string())),
        }
        pieces.append(&mut line);
    }

    pieces.push(DdlPiece::Literal("\n);".to_string()));

    pieces
}

/// Format one column's SQL fragment.
///
/// Mirrors `Column::to_column_sql` in `drizzle_types::postgres::ddl::sql`:
/// - identifier and type with double quotes,
/// - `DEFAULT val` only when no identity/generated/serial occupies that slot
///   (Postgres doesn't allow both),
/// - `NOT NULL` is implicit on `SERIAL`/`BIGSERIAL`/`SMALLSERIAL`, so we skip
///   it for those.
fn column_to_sql(field: &FieldInfo) -> String {
    let mut sql = format!(
        "\"{}\" {}",
        field.column_name,
        field.column_type.to_sql_type()
    );

    // COLLATE follows the type in the Postgres grammar. Collation names
    // are quoted identifiers (`COLLATE "en_US"`, `COLLATE "C"`).
    if let Some(ref name) = field.collate {
        let _ = write!(sql, " COLLATE \"{name}\"");
    }

    if field.is_generated_identity {
        let identity_type = if matches!(
            field.identity_mode,
            Some(crate::postgres::field::IdentityMode::ByDefault)
        ) {
            "BY DEFAULT"
        } else {
            "ALWAYS"
        };
        let _ = write!(sql, " GENERATED {identity_type} AS IDENTITY");
    }

    if let Some(ref generated) = field.generated_column
        && generated.stored
    {
        let _ = write!(
            sql,
            " GENERATED ALWAYS AS ({}) STORED",
            generated.expression
        );
    }

    // Default value (serial types carry an implicit DEFAULT nextval(seq))
    if !field.is_serial
        && !field.is_generated_identity
        && field.generated_column.is_none()
        && let Some(ref default) = field.default
    {
        let default_str = match default {
            PostgreSQLDefault::Literal(s) | PostgreSQLDefault::Function(s) => s.clone(),
            PostgreSQLDefault::Expression(ts) => ts.to_string(),
        };
        let _ = write!(sql, " DEFAULT {default_str}");
    }

    // NOT NULL — serial types are implicitly NOT NULL in Postgres
    if !field.is_nullable && !field.is_serial {
        sql.push_str(" NOT NULL");
    }

    sql
}

/// Convert a referential action string to the corresponding enum variant token
fn referential_action_token(action: &str, referential_action: &TokenStream) -> TokenStream {
    match action.to_uppercase().as_str() {
        "RESTRICT" => quote! { #referential_action::Restrict },
        "CASCADE" => quote! { #referential_action::Cascade },
        "SET NULL" => quote! { #referential_action::SetNull },
        "SET DEFAULT" => quote! { #referential_action::SetDefault },
        // "NO ACTION" and unknown values default to NoAction
        _ => quote! { #referential_action::NoAction },
    }
}

/// Generate const DDL entities for a `PostgreSQL` table
pub fn generate_const_ddl(ctx: &MacroContext, _column_zst_idents: &[Ident]) -> TokenStream {
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let schema_name = ctx.attrs.schema.as_deref().unwrap_or("public");

    // Get core type paths for SQLSchema reference
    let sql_schema = core_paths::sql_schema();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let postgres_value = postgres_paths::postgres_value();

    // Get DDL type paths
    let table_def = ddl_paths::table_def();
    let column_def = ddl_paths::column_def();
    let primary_key_def = ddl_paths::primary_key_def();
    let foreign_key_def = ddl_paths::foreign_key_def();
    let unique_constraint_def = ddl_paths::unique_constraint_def();
    let index_def = ddl_paths::index_def();
    let identity_def = ddl_paths::identity_def();
    let table_sql = ddl_paths::table_sql();
    let referential_action = ddl_paths::referential_action();

    // Generate column definitions
    let column_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .map(|field| {
            let column_name = &field.column_name;
            let sql_type = field.column_type.to_sql_type();

            let mut modifiers = Vec::new();

            if !field.is_nullable {
                modifiers.push(quote! { .not_null() });
            }
            // Note: Primary key is handled at table level via DDL_PRIMARY_KEY
            // PostgreSQL doesn't use column-level primary_key() in ColumnDef
            if field.is_unique() {
                // Unique constraints are also handled at table level via DDL_UNIQUE_CONSTRAINTS
            }
            // Note: Serial columns use the SERIAL pseudo-type which handles auto-increment
            // via a sequence + DEFAULT nextval(...). We do NOT set identity for serial columns.
            // The SERIAL type in the column definition is sufficient.
            // Only add default if not a serial column (SERIAL has implicit DEFAULT)
            if !field.is_serial
                && let Some(ref default) = field.default
            {
                let default_str = match default {
                    PostgreSQLDefault::Literal(s) | PostgreSQLDefault::Function(s) => s.clone(),
                    PostgreSQLDefault::Expression(ts) => ts.to_string(),
                };
                modifiers.push(quote! { .default_value(#default_str) });
            }
            if let Some(ref collate_name) = field.collate {
                modifiers.push(quote! { .collate(#collate_name) });
            }
            if let Some(ref generated) = field.generated_column
                && generated.stored
            {
                let expression = &generated.expression;
                modifiers.push(quote! { .generated_stored(#expression) });
            }
            if !field.is_serial && field.is_generated_identity {
                let seq_name = format!("{table_name}_{column_name}_seq");
                let identity_type = if matches!(
                    field.identity_mode,
                    Some(crate::postgres::field::IdentityMode::ByDefault)
                ) {
                    quote! { drizzle::ddl::postgres::ddl::IdentityType::ByDefault }
                } else {
                    quote! { drizzle::ddl::postgres::ddl::IdentityType::Always }
                };
                modifiers.push(quote! {
                    .identity(#identity_def::new(#seq_name, #identity_type).schema(#schema_name))
                });
            }

            quote! {
                #column_def::new(#schema_name, #table_name, #column_name, #sql_type)
                #(#modifiers)*
            }
        })
        .collect();

    // Build primary key DDL if there are primary key columns
    let pk_columns: Vec<&String> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_primary())
        .map(|f| &f.column_name)
        .collect();

    let pk_name = format!("{table_name}_pkey");
    let pk_def = if pk_columns.is_empty() {
        quote! {
            /// Primary key definition (none)
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> =
                ::std::option::Option::None;
        }
    } else {
        let pk_col_cows: Vec<TokenStream> = pk_columns
            .iter()
            .map(|col| quote! { ::std::borrow::Cow::Borrowed(#col) })
            .collect();
        quote! {
            /// Primary key definition
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> = {
                const PK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#pk_col_cows),*];
                ::std::option::Option::Some(#primary_key_def::new(#schema_name, #table_name, #pk_name).columns(PK_COLS))
            };
        }
    };

    // Build foreign key DDL definitions
    let mut fk_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            field.foreign_key.as_ref().map(|fk_ref| {
                let ref_table_ident = &fk_ref.table;
                let ref_column = fk_ref.column.to_string();
                let fk_name = format!(
                    "{}_{}_fkey",
                    table_name, field.column_name
                );
                let column_name = &field.column_name;

                let mut modifiers = Vec::new();
                if let Some(ref on_delete) = fk_ref.on_delete {
                    let action_token = referential_action_token(on_delete.as_str(), &referential_action);
                    modifiers.push(quote! { .on_delete(#action_token) });
                }
                if let Some(ref on_update) = fk_ref.on_update {
                    let action_token = referential_action_token(on_update.as_str(), &referential_action);
                    modifiers.push(quote! { .on_update(#action_token) });
                }

                quote! {
                    {
                        const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                        const FK_REF_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#ref_column)];
                        #foreign_key_def::new(#schema_name, #table_name, #fk_name)
                            .columns(FK_COLS)
                            .references(<#ref_table_ident>::DDL_TABLE.schema, <#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
                            #(#modifiers)*
                    }
                }
            })
        })
        .collect();

    for fk in &ctx.attrs.composite_foreign_keys {
        let ref_table_ident = &fk.target_table;
        let source_columns: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| &f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let target_columns: Vec<String> = fk
            .target_columns
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let fk_name = format!("{}_{}_fkey", table_name, source_columns.join("_"));
        let fk_cols: Vec<TokenStream> = source_columns
            .iter()
            .map(|c| quote! { ::std::borrow::Cow::Borrowed(#c) })
            .collect();
        let fk_ref_cols: Vec<TokenStream> = target_columns
            .iter()
            .map(|c| quote! { ::std::borrow::Cow::Borrowed(#c) })
            .collect();

        let mut modifiers = Vec::new();
        if let Some(ref on_delete) = fk.on_delete {
            let action_token = referential_action_token(on_delete.as_str(), &referential_action);
            modifiers.push(quote! { .on_delete(#action_token) });
        }
        if let Some(ref on_update) = fk.on_update {
            let action_token = referential_action_token(on_update.as_str(), &referential_action);
            modifiers.push(quote! { .on_update(#action_token) });
        }

        fk_defs.push(quote! {
            {
                const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#fk_cols),*];
                const FK_REF_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#fk_ref_cols),*];
                #foreign_key_def::new(#schema_name, #table_name, #fk_name)
                    .columns(FK_COLS)
                    .references(<#ref_table_ident>::DDL_TABLE.schema, <#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
                    #(#modifiers)*
            }
        });
    }

    // Build unique constraint DDL definitions (for non-primary unique columns)
    let unique_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_unique())
        .map(|field| {
            let unique_name = format!("{}_{}_unique", table_name, field.column_name);
            let column_name = &field.column_name;

            quote! {
                {
                    const UQ_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                    #unique_constraint_def::new(#schema_name, #table_name, #unique_name).columns(UQ_COLS)
                }
            }
        })
        .collect();

    quote! {
        impl #struct_ident {
            /// Const DDL table definition for compile-time schema metadata.
            pub const DDL_TABLE: #table_def =
                #table_def::new(#schema_name, #table_name);

            /// Const DDL column definitions for compile-time schema metadata.
            pub const DDL_COLUMNS: &'static [#column_def] = &[
                #(#column_defs),*
            ];

            #pk_def

            /// Foreign key definitions
            pub const DDL_FOREIGN_KEYS: &'static [#foreign_key_def] = &[
                #(#fk_defs),*
            ];

            /// Unique constraint definitions
            pub const DDL_UNIQUE_CONSTRAINTS: &'static [#unique_constraint_def] = &[
                #(#unique_defs),*
            ];

            /// Index definitions (defined via separate #[PostgresIndex] structs)
            pub const DDL_INDEXES: &'static [#index_def] = &[];

            /// Generate the CREATE TABLE SQL using the DDL definitions.
            ///
            /// This is the single source of truth for SQL generation, building
            /// the statement from the const DDL entities above.
            pub fn create_table_sql() -> ::std::string::String {
                let table = Self::DDL_TABLE.into_table();
                let columns: ::std::vec::Vec<_> = Self::DDL_COLUMNS.iter().map(|c| c.into_column()).collect();
                let pk = Self::DDL_PRIMARY_KEY.map(|p| p.into_primary_key());
                let fks: ::std::vec::Vec<_> = Self::DDL_FOREIGN_KEYS.iter().map(|f| f.into_foreign_key()).collect();
                let uniques: ::std::vec::Vec<_> = Self::DDL_UNIQUE_CONSTRAINTS.iter().map(|u| u.into_unique_constraint()).collect();

                #table_sql::new(&table)
                    .columns(&columns)
                    .primary_key(pk.as_ref())
                    .foreign_keys(&fks)
                    .unique_constraints(&uniques)
                    .create_table_sql()
            }

            /// Returns the DDL SQL for creating this table.
            pub fn ddl_sql() -> &'static str {
                <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_const_ddl;
    use crate::postgres::field::{
        FieldInfo, GeneratedColumn, IdentityMode, PostgreSQLDefault, PostgreSQLReference,
        PostgreSQLType,
    };
    use crate::postgres::table::{attributes::TableAttributes, context::MacroContext};
    use std::collections::HashSet;

    fn base_field(name: &str, column_type: PostgreSQLType) -> FieldInfo {
        let ident: syn::Ident = syn::parse_str(name).expect("valid ident");
        let vis: syn::Visibility = syn::parse_str("pub").expect("valid visibility");
        let field_type: syn::Type = syn::parse_str("i32").expect("valid type");

        FieldInfo {
            ident,
            vis,
            field_type: field_type.clone(),
            base_type: field_type,
            column_name: name.to_string(),
            sql_definition: format!("\"{name}\" {}", column_type.to_sql_type()),
            column_type,
            flags: HashSet::new(),
            is_nullable: false,
            is_enum: false,
            is_pgenum: false,
            is_json: false,
            is_jsonb: false,
            is_serial: false,
            is_custom_type: false,
            is_generated_identity: false,
            identity_mode: None,
            generated_column: None,
            default: None,
            default_fn: None,
            check_constraint: None,
            foreign_key: None,
            has_default: false,
            marker_exprs: Vec::new(),
            constraint: crate::common::Constraint::None,
            collate: None,
        }
    }

    #[test]
    fn generated_fk_uses_referenced_table_schema_constant() {
        let struct_ident: syn::Ident = syn::parse_str("Posts").expect("valid ident");
        let struct_vis: syn::Visibility = syn::parse_str("pub").expect("valid visibility");

        let id_ident: syn::Ident = syn::parse_str("user_id").expect("valid ident");
        let id_type: syn::Type = syn::parse_str("i32").expect("valid type");

        let field = FieldInfo {
            ident: id_ident,
            vis: struct_vis.clone(),
            field_type: id_type.clone(),
            base_type: id_type,
            column_name: "user_id".to_string(),
            sql_definition: "\"user_id\" integer".to_string(),
            column_type: PostgreSQLType::Integer,
            flags: HashSet::new(),
            is_nullable: false,
            is_enum: false,
            is_pgenum: false,
            is_json: false,
            is_jsonb: false,
            is_serial: false,
            is_custom_type: false,
            is_generated_identity: false,
            identity_mode: None,
            generated_column: None,
            default: None,
            default_fn: None,
            check_constraint: None,
            foreign_key: Some(PostgreSQLReference {
                table: syn::parse_str("Users").expect("valid ref table"),
                column: syn::parse_str("id").expect("valid ref column"),
                on_delete: None,
                on_update: None,
            }),
            has_default: false,
            marker_exprs: Vec::new(),
            constraint: crate::common::Constraint::None,
            collate: None,
        };

        let fields = vec![field];
        let attrs = TableAttributes {
            name: None,
            schema: Some("app".to_string()),
            unlogged: false,
            temporary: false,
            inherits: None,
            tablespace: None,
            composite_foreign_keys: Vec::new(),
            marker_exprs: Vec::new(),
        };

        let ctx = MacroContext {
            struct_ident: &struct_ident,
            struct_vis: &struct_vis,
            table_name: "posts".to_string(),
            field_infos: &fields,
            select_model_ident: syn::parse_str("PostsSelect").expect("valid ident"),
            select_model_partial_ident: syn::parse_str("PostsPartial").expect("valid ident"),
            insert_model_ident: syn::parse_str("PostsInsert").expect("valid ident"),
            update_model_ident: syn::parse_str("PostsUpdate").expect("valid ident"),
            is_composite_pk: false,
            attrs: &attrs,
        };

        let tokens = generate_const_ddl(&ctx, &[]).to_string();
        assert!(
            tokens.contains(":: DDL_TABLE . schema"),
            "expected FK references to use referenced table schema constant, got: {tokens}"
        );
    }

    #[test]
    fn postgres_column_sql_preserves_marker_metadata() {
        let mut identity = base_field("identity_id", PostgreSQLType::Integer);
        identity.is_generated_identity = true;
        identity.identity_mode = Some(IdentityMode::ByDefault);
        identity.default = Some(PostgreSQLDefault::Literal("1".to_string()));
        assert_eq!(
            super::column_to_sql(&identity),
            "\"identity_id\" INTEGER GENERATED BY DEFAULT AS IDENTITY NOT NULL"
        );

        let mut generated = base_field("full_name", PostgreSQLType::Text);
        generated.generated_column = Some(GeneratedColumn {
            expression: "first_name || ' ' || last_name".to_string(),
            stored: true,
        });
        generated.default = Some(PostgreSQLDefault::Literal("'ignored'".to_string()));
        assert_eq!(
            super::column_to_sql(&generated),
            "\"full_name\" TEXT GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED NOT NULL"
        );

        let mut collated = base_field("sortable", PostgreSQLType::Text);
        collated.collate = Some("C".to_string());
        assert_eq!(
            super::column_to_sql(&collated),
            "\"sortable\" TEXT COLLATE \"C\" NOT NULL"
        );
    }

    #[test]
    fn const_ddl_preserves_column_marker_metadata() {
        let struct_ident: syn::Ident = syn::parse_str("Users").expect("valid ident");
        let struct_vis: syn::Visibility = syn::parse_str("pub").expect("valid visibility");

        let mut identity = base_field("identity_id", PostgreSQLType::Integer);
        identity.is_generated_identity = true;
        identity.identity_mode = Some(IdentityMode::ByDefault);

        let mut generated = base_field("full_name", PostgreSQLType::Text);
        generated.generated_column = Some(GeneratedColumn {
            expression: "first_name || ' ' || last_name".to_string(),
            stored: true,
        });

        let mut collated = base_field("sortable", PostgreSQLType::Text);
        collated.collate = Some("C".to_string());

        let fields = vec![identity, generated, collated];
        let attrs = TableAttributes {
            name: None,
            schema: Some("app".to_string()),
            unlogged: false,
            temporary: false,
            inherits: None,
            tablespace: None,
            composite_foreign_keys: Vec::new(),
            marker_exprs: Vec::new(),
        };

        let ctx = MacroContext {
            struct_ident: &struct_ident,
            struct_vis: &struct_vis,
            table_name: "users".to_string(),
            field_infos: &fields,
            select_model_ident: syn::parse_str("UsersSelect").expect("valid ident"),
            select_model_partial_ident: syn::parse_str("UsersPartial").expect("valid ident"),
            insert_model_ident: syn::parse_str("UsersInsert").expect("valid ident"),
            update_model_ident: syn::parse_str("UsersUpdate").expect("valid ident"),
            is_composite_pk: false,
            attrs: &attrs,
        };

        let tokens = generate_const_ddl(&ctx, &[]).to_string();
        assert!(tokens.contains(". identity"));
        assert!(tokens.contains("IdentityType :: ByDefault"));
        assert!(tokens.contains(". generated_stored"));
        assert!(tokens.contains(". collate"));
    }
}
