//! Shared generation of `TABLE_REF` const for both SQLite and PostgreSQL.

use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::quote;

/// Column metadata for TABLE_REF generation. Dialect-specific code creates
/// these from their own FieldInfo types.
pub(crate) struct ColumnRefInput {
    pub column_name: String,
    pub sql_type: String,
    pub not_null: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub has_default: bool,
    /// The `ColumnDialect` variant as a TokenStream.
    pub dialect: TokenStream,
}

/// Foreign key metadata for TABLE_REF generation.
pub(crate) struct ForeignKeyRefInput {
    /// Source columns in this table.
    pub source_columns: Vec<String>,
    /// Target table name expression (may use associated const).
    pub target_table: TokenStream,
    /// Target columns in the referenced table.
    pub target_columns: Vec<String>,
}

/// Constraint metadata for TABLE_REF generation.
pub(crate) struct ConstraintRefInput {
    pub name: Option<String>,
    pub kind: TokenStream,
    pub columns: Vec<String>,
    pub check_expression: Option<String>,
}

/// Generates the `const TABLE_REF: TableRef = ...;` body for a DrizzleTable impl.
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_table_ref_const(
    table_name_expr: &TokenStream,
    qualified_name_expr: &TokenStream,
    schema_expr: &TokenStream,
    column_names: &[&String],
    columns: &[ColumnRefInput],
    primary_key_columns: &[String],
    foreign_keys: &[ForeignKeyRefInput],
    constraints: &[ConstraintRefInput],
    dependency_names_expr: &TokenStream,
    table_dialect: &TokenStream,
) -> TokenStream {
    let table_ref = core_paths::table_ref();
    let column_ref = core_paths::column_ref();
    let primary_key_ref = core_paths::primary_key_ref();
    let foreign_key_ref = core_paths::foreign_key_ref();
    let constraint_ref = core_paths::constraint_ref();

    // Generate column ref literals
    let column_ref_literals: Vec<TokenStream> = columns
        .iter()
        .map(|col| {
            let col_name = &col.column_name;
            let sql_type = &col.sql_type;
            let not_null = col.not_null;
            let primary_key = col.primary_key;
            let unique = col.unique;
            let has_default = col.has_default;
            let dialect = &col.dialect;

            quote! {
                #column_ref {
                    table: #table_name_expr,
                    name: #col_name,
                    sql_type: #sql_type,
                    not_null: #not_null,
                    primary_key: #primary_key,
                    unique: #unique,
                    has_default: #has_default,
                    dialect: #dialect,
                }
            }
        })
        .collect();

    // Generate primary key
    let pk_expr = if primary_key_columns.is_empty() {
        quote! { ::core::option::Option::None }
    } else {
        quote! {
            ::core::option::Option::Some(#primary_key_ref {
                columns: &[#(#primary_key_columns),*],
            })
        }
    };

    // Generate foreign key refs
    let fk_ref_literals: Vec<TokenStream> = foreign_keys
        .iter()
        .map(|fk| {
            let target_table = &fk.target_table;
            let source_columns = &fk.source_columns;
            let target_columns = &fk.target_columns;
            quote! {
                #foreign_key_ref {
                    target_table: #target_table,
                    source_columns: &[#(#source_columns),*],
                    target_columns: &[#(#target_columns),*],
                }
            }
        })
        .collect();

    // Generate constraint refs
    let constraint_ref_literals: Vec<TokenStream> = constraints
        .iter()
        .map(|c| {
            let name_expr = match &c.name {
                Some(n) => quote! { ::core::option::Option::Some(#n) },
                None => quote! { ::core::option::Option::None },
            };
            let kind = &c.kind;
            let columns = &c.columns;
            let check_expr = match &c.check_expression {
                Some(e) => quote! { ::core::option::Option::Some(#e) },
                None => quote! { ::core::option::Option::None },
            };
            quote! {
                #constraint_ref {
                    name: #name_expr,
                    kind: #kind,
                    columns: &[#(#columns),*],
                    check_expression: #check_expr,
                }
            }
        })
        .collect();

    quote! {
        const TABLE_REF: #table_ref = #table_ref {
            name: #table_name_expr,
            column_names: &[#(#column_names),*],
            schema: #schema_expr,
            qualified_name: #qualified_name_expr,
            columns: &[#(#column_ref_literals),*],
            primary_key: #pk_expr,
            foreign_keys: &[#(#fk_ref_literals),*],
            constraints: &[#(#constraint_ref_literals),*],
            dependency_names: #dependency_names_expr,
            dialect: #table_dialect,
        };
    }
}
