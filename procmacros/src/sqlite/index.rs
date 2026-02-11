use crate::paths::{core as core_paths, ddl::sqlite as ddl_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Expr, Meta, Result, Token, Type, parse::Parse};

/// Attributes for the SQLiteIndex attribute macro
/// Syntax: #[SQLiteIndex] or #[SQLiteIndex(unique)]
pub struct IndexAttributes {
    pub unique: bool,
}

impl Parse for IndexAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut unique = false;

        if input.is_empty() {
            return Ok(IndexAttributes { unique });
        }

        let metas = input.parse_terminated(Meta::parse, Token![,])?;

        for meta in metas {
            match meta {
                Meta::Path(path) if path.is_ident("unique") => {
                    unique = true;
                }
                _ => {
                    return Err(Error::new_spanned(
                        meta,
                        "Only 'unique' is supported in SQLiteIndex attribute",
                    ));
                }
            }
        }

        Ok(IndexAttributes { unique })
    }
}

/// Generates the SQLiteIndex implementation
pub fn sqlite_index_attr_macro(attr: IndexAttributes, input: DeriveInput) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let is_unique = attr.unique;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_index = core_paths::sql_index();
    let sql_index_info = core_paths::sql_index_info();
    let sql_table_info = core_paths::sql_table_info();
    let to_sql = core_paths::to_sql();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();

    // DDL type paths
    let index_def = ddl_paths::index_def();
    let index_column_def = ddl_paths::index_column_def();

    // Extract columns from tuple struct fields: struct UserEmailIdx(User::email);
    let columns = match &input.data {
        syn::Data::Struct(data_struct) => {
            match &data_struct.fields {
                syn::Fields::Unnamed(fields) => {
                    fields
                        .unnamed
                        .iter()
                        .map(|field| {
                            // Convert Type to Expr
                            match &field.ty {
                                Type::Path(type_path) => Ok(Expr::Path(syn::ExprPath {
                                    attrs: vec![],
                                    qself: type_path.qself.clone(),
                                    path: type_path.path.clone(),
                                })),
                                _ => Err(Error::new_spanned(
                                    &field.ty,
                                    "Column must be a path like User::email",
                                )),
                            }
                        })
                        .collect::<Result<Vec<_>>>()?
                }
                _ => {
                    return Err(Error::new_spanned(
                        &input,
                        "SQLiteIndex can only be applied to tuple structs like `struct UserEmailIdx(User::email);`",
                    ));
                }
            }
        }
        _ => {
            return Err(Error::new_spanned(
                &input,
                "SQLiteIndex can only be applied to structs",
            ));
        }
    };

    // Extract table type from first column
    let table_type = if let Some(first_column) = columns.first() {
        extract_table_from_column(first_column)?
    } else {
        return Err(Error::new_spanned(
            struct_ident,
            "Index must have at least one column",
        ));
    };

    // Validate all columns are from the same table
    for column in &columns {
        let column_table = extract_table_from_column(column)?;
        if quote::quote!(#table_type).to_string() != quote::quote!(#column_table).to_string() {
            return Err(Error::new_spanned(
                column,
                "All columns in an index must belong to the same table",
            ));
        }
    }

    // Generate index name from struct name (e.g., UserEmailIdx -> user_email_idx)
    let index_name =
        struct_ident
            .to_string()
            .chars()
            .enumerate()
            .fold(String::new(), |mut acc, (i, c)| {
                if i > 0 && c.is_uppercase() {
                    acc.push('_');
                }
                acc.push(c.to_lowercase().next().unwrap());
                acc
            });

    // Build IndexColumnDef array for DDL using the column's NAME const
    // Uses a const block to validate that the column path implements SQLSchema
    // and extracts its NAME - this ensures we use the actual database column name
    let column_defs: Vec<_> = columns
        .iter()
        .map(|col| {
            quote! {
                #index_column_def::new({
                    // Const validation that the column implements SQLSchema
                    const fn column_name<'a, C: #sql_schema<'a, &'static str, #sqlite_value<'a>>>(_: &C) -> &'a str {
                        C::NAME
                    }
                    column_name(&#col)
                })
            }
        })
        .collect();

    let column_names: Vec<_> = columns
        .iter()
        .map(|col| {
            quote! {
                {
                    const fn column_name<'a, C: #sql_schema<'a, &'static str, #sqlite_value<'a>>>(_: &C) -> &'a str {
                        C::NAME
                    }
                    column_name(&#col)
                }
            }
        })
        .collect();

    // Generate optional .unique() call
    let unique_modifier = if is_unique {
        quote! { .unique() }
    } else {
        quote! {}
    };

    Ok(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #struct_vis struct #struct_ident;

        impl #struct_ident {
            /// Const DDL column definitions for the index
            pub const DDL_COLUMNS: &'static [#index_column_def] = &[#(#column_defs),*];

            /// Column names for schema snapshot generation
            pub const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];

            /// Const DDL index definition - single source of truth
            pub const DDL_INDEX: #index_def = #index_def::new(
                <#table_type as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME,
                #index_name
            )
            .columns(Self::DDL_COLUMNS)
            #unique_modifier;

            pub const fn new() -> Self {
                Self
            }

            /// Generate CREATE INDEX SQL using the DDL definition
            pub fn create_index_sql() -> ::std::string::String {
                Self::DDL_INDEX.into_index().create_index_sql()
            }
        }

        impl Default for #struct_ident {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<'a> #sql_index<'a, #sqlite_schema_type, #sqlite_value<'a>> for #struct_ident
        {
            type Table = #table_type;
        }

        impl #sql_index_info for #struct_ident
        {
            fn table(&self) -> &dyn #sql_table_info {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #table_type = #table_type::new();
                &TABLE_INSTANCE
            }

            fn name(&self) -> &'static str {
                #index_name
            }

            fn is_unique(&self) -> bool {
                #is_unique
            }

            fn columns(&self) -> &'static [&'static str] {
                Self::COLUMN_NAMES
            }
        }

        impl<'a> #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>> for #struct_ident
        {
            const NAME: &'static str = #index_name;
            const TYPE: #sqlite_schema_type = {
                #[allow(non_upper_case_globals)]
                static INDEX_INSTANCE: #struct_ident = #struct_ident::new();
                #sqlite_schema_type::Index(&INDEX_INSTANCE)
            };
            const SQL: &'static str = "";

            fn sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #sql::raw(Self::create_index_sql())
            }
        }

        impl<'a> #to_sql<'a, #sqlite_value<'a>> for #struct_ident
        {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #sql::raw(Self::create_index_sql())
            }
        }
    })
}

fn extract_table_from_column(column: &Expr) -> Result<Type> {
    if let Expr::Path(expr_path) = column {
        let path = &expr_path.path;
        if path.segments.len() >= 2 {
            // Extract table name (first segment)
            let table_ident = &path.segments[0].ident;

            // Create table type
            let table_type = syn::parse_str::<Type>(&table_ident.to_string())
                .map_err(|_| Error::new_spanned(column, "Invalid table name"))?;

            Ok(table_type)
        } else {
            Err(Error::new_spanned(
                column,
                "Column must be in format Table::column",
            ))
        }
    } else {
        Err(Error::new_spanned(
            column,
            "Column must be a path expression",
        ))
    }
}
