//! Shared generation of `TABLE_REF` const for both `SQLite` and `PostgreSQL`.

use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::quote;

/// Column-ref flag bits (matches `ColumnFlags::from_bits` in core).
#[derive(Clone, Copy, Default)]
pub struct ColumnRefFlags(u8);

impl ColumnRefFlags {
    pub const NOT_NULL: u8 = 1 << 0;
    pub const PRIMARY_KEY: u8 = 1 << 1;
    pub const UNIQUE: u8 = 1 << 2;
    pub const HAS_DEFAULT: u8 = 1 << 3;

    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn with(mut self, flag: u8, set: bool) -> Self {
        if set {
            self.0 |= flag;
        }
        self
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// Column metadata for `TABLE_REF` generation. Dialect-specific code creates
/// these from their own `FieldInfo` types.
pub struct ColumnRefInput {
    pub column_name: String,
    pub sql_type: String,
    pub flags: ColumnRefFlags,
    /// The `ColumnDialect` variant as a `TokenStream`.
    pub dialect: TokenStream,
}

/// Foreign key metadata for `TABLE_REF` generation.
pub struct ForeignKeyRefInput {
    /// Source columns in this table.
    pub source_columns: Vec<String>,
    /// Target table name expression (may use associated const).
    pub target_table: TokenStream,
    /// Target columns in the referenced table.
    pub target_columns: Vec<String>,
}

/// Constraint metadata for `TABLE_REF` generation.
pub struct ConstraintRefInput {
    pub name: Option<String>,
    pub kind: TokenStream,
    pub columns: Vec<String>,
    pub check_expression: Option<String>,
}

/// Generates the `const TABLE_REF: TableRef = ...;` body for a `DrizzleTable` impl.
#[allow(clippy::too_many_arguments)]
pub fn generate_table_ref_const(
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
    let column_flags = core_paths::column_flags();
    let primary_key_ref = core_paths::primary_key_ref();
    let foreign_key_ref = core_paths::foreign_key_ref();
    let constraint_ref = core_paths::constraint_ref();

    // Generate column ref literals
    let column_ref_literals: Vec<TokenStream> = columns
        .iter()
        .map(|col| {
            let col_name = &col.column_name;
            let sql_type = &col.sql_type;
            let flag_bits = col.flags.bits();
            let dialect = &col.dialect;

            quote! {
                #column_ref {
                    table: #table_name_expr,
                    name: #col_name,
                    sql_type: #sql_type,
                    flags: #column_flags::from_bits(#flag_bits),
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
            let name_expr = c.name.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |n| quote! { ::core::option::Option::Some(#n) },
            );
            let kind = &c.kind;
            let columns = &c.columns;
            let check_expr = c.check_expression.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |e| quote! { ::core::option::Option::Some(#e) },
            );
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
