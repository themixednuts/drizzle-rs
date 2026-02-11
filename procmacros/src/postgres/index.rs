use crate::paths::{core as core_paths, ddl::postgres as ddl_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Expr, ExprPath, Ident, Meta, Result, Token, Type, parse::Parse};

/// Attributes for the PostgresIndex attribute macro
/// Syntax: #[PostgresIndex] or #[PostgresIndex(unique)] or #[PostgresIndex(unique, method = "btree")]
pub struct IndexAttributes {
    pub unique: bool,
    pub concurrent: bool,
    pub method: Option<String>, // btree, hash, gin, gist, spgist, brin
    pub tablespace: Option<String>,
    pub where_clause: Option<String>,
}

impl Default for IndexAttributes {
    fn default() -> Self {
        Self {
            unique: false,
            concurrent: false,
            method: Some("btree".to_string()), // Default to btree
            tablespace: None,
            where_clause: None,
        }
    }
}

impl Parse for IndexAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = IndexAttributes::default();

        if input.is_empty() {
            return Ok(attrs);
        }

        let metas = input.parse_terminated(Meta::parse, Token![,])?;

        for meta in metas {
            match meta {
                Meta::Path(path) if path.is_ident("unique") => {
                    attrs.unique = true;
                }
                Meta::Path(path) if path.is_ident("concurrent") => {
                    attrs.concurrent = true;
                }
                Meta::NameValue(nv) if nv.path.is_ident("method") => {
                    if let syn::Expr::Lit(ref lit) = nv.value
                        && let syn::Lit::Str(str_lit) = &lit.lit
                    {
                        let method = str_lit.value();
                        // Validate PostgreSQL index methods
                        match method.as_str() {
                            "btree" | "hash" | "gin" | "gist" | "spgist" | "brin" => {
                                attrs.method = Some(method);
                            }
                            _ => {
                                return Err(Error::new_spanned(
                                    str_lit,
                                    "Invalid index method. Supported methods: btree, hash, gin, gist, spgist, brin",
                                ));
                            }
                        }
                    } else {
                        return Err(Error::new_spanned(
                            &nv,
                            "Expected string literal for method",
                        ));
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("tablespace") => {
                    if let syn::Expr::Lit(ref lit) = nv.value
                        && let syn::Lit::Str(str_lit) = &lit.lit
                    {
                        attrs.tablespace = Some(str_lit.value());
                    } else {
                        return Err(Error::new_spanned(
                            &nv,
                            "Expected string literal for tablespace",
                        ));
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("where") => {
                    if let syn::Expr::Lit(ref lit) = nv.value
                        && let syn::Lit::Str(str_lit) = &lit.lit
                    {
                        attrs.where_clause = Some(str_lit.value());
                    } else {
                        return Err(Error::new_spanned(
                            &nv,
                            "Expected string literal for where clause",
                        ));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(
                        meta,
                        "Unrecognized index attribute.\n\
                         Supported attributes:\n\
                         - unique: Create unique index\n\
                         - concurrent: Create index concurrently\n\
                         - method: Index method (btree, hash, gin, gist, spgist, brin)\n\
                         - tablespace: Specify tablespace\n\
                         - where: Partial index condition\n\
                         See: https://www.postgresql.org/docs/current/sql-createindex.html",
                    ));
                }
            }
        }

        Ok(attrs)
    }
}

/// Generates the PostgresIndex implementation
pub fn postgres_index_attr_macro(attr: IndexAttributes, input: DeriveInput) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_index = core_paths::sql_index();
    let sql_index_info = core_paths::sql_index_info();
    let sql_table_info = core_paths::sql_table_info();
    let to_sql = core_paths::to_sql();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

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
                                    qself: None,
                                    path: type_path.path.clone(),
                                })),
                                _ => Err(Error::new_spanned(
                                    field,
                                    "Index columns must be table column references (e.g., Users::email)",
                                )),
                            }
                        })
                        .collect::<Result<Vec<_>>>()?
                }
                _ => {
                    return Err(Error::new_spanned(
                        input,
                        "PostgresIndex must be applied to a tuple struct with column references",
                    ));
                }
            }
        }
        _ => {
            return Err(Error::new_spanned(
                input,
                "PostgresIndex can only be applied to tuple structs",
            ));
        }
    };

    // Parse column references (for index name generation)
    let column_info = parse_column_references(&columns)?;

    // Extract table type from first column
    let table_type = if let Some(first_column) = columns.first() {
        extract_table_from_column(first_column)?
    } else {
        return Err(Error::new_spanned(
            struct_ident,
            "Index must have at least one column",
        ));
    };

    // Generate index name from struct name
    let index_name = generate_index_name(struct_ident, &column_info);

    // Build IndexColumnDef array for DDL using the column's NAME const
    // Uses a const block to validate that the column path implements SQLSchema
    // and extracts its NAME - this ensures we use the actual database column name
    let column_defs: Vec<_> = columns
        .iter()
        .map(|col| {
            quote! {
                #index_column_def::new({
                    // Const validation that the column implements SQLSchema
                    const fn column_name<'a, C: #sql_schema<'a, &'static str, #postgres_value<'a>>>(_: &C) -> &'a str {
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
                    const fn column_name<'a, C: #sql_schema<'a, &'static str, #postgres_value<'a>>>(_: &C) -> &'a str {
                        C::NAME
                    }
                    column_name(&#col)
                }
            }
        })
        .collect();

    // Generate optional modifiers
    let unique_modifier = if attr.unique {
        quote! { .unique() }
    } else {
        quote! {}
    };

    let concurrent_modifier = if attr.concurrent {
        quote! { .concurrently() }
    } else {
        quote! {}
    };

    let method_modifier = if let Some(ref method) = attr.method {
        quote! { .method(#method) }
    } else {
        quote! {}
    };

    let where_modifier = if let Some(ref where_clause) = attr.where_clause {
        quote! { .where_clause(#where_clause) }
    } else {
        quote! {}
    };

    let is_unique = attr.unique;

    // Generate the index struct and implementations
    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #struct_vis struct #struct_ident;

        impl #struct_ident {
            /// Const DDL column definitions for the index
            pub const DDL_COLUMNS: &'static [#index_column_def] = &[#(#column_defs),*];

            /// Column names for schema snapshot generation
            pub const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];

            /// Const DDL index definition - single source of truth
            pub const DDL_INDEX: #index_def = #index_def::new(
                #table_type::DDL_TABLE.schema,
                #table_type::DDL_TABLE.name,
                #index_name,
                Self::DDL_COLUMNS
            )
            #unique_modifier
            #concurrent_modifier
            #method_modifier
            #where_modifier;

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

        impl<'a> #sql_index<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            type Table = #table_type;
        }

        impl #sql_index_info for #struct_ident {
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

        impl<'a> #sql_schema<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            const NAME: &'static str = #index_name;
            const TYPE: #postgres_schema_type = {
                #[allow(non_upper_case_globals)]
                static INDEX_INSTANCE: #struct_ident = #struct_ident::new();
                #postgres_schema_type::Index(&INDEX_INSTANCE)
            };
            const SQL: &'static str = "";

            fn sql(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::raw(Self::create_index_sql())
            }
        }

        impl<'a> #to_sql<'a, #postgres_value<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::raw(Self::create_index_sql())
            }
        }
    };

    Ok(expanded)
}

/// Information about a column reference in an index
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ColumnReference {
    table_name: String,
    column_name: String,
}

/// Parse column references from expressions
fn parse_column_references(columns: &[Expr]) -> Result<Vec<ColumnReference>> {
    let mut column_refs = Vec::new();

    for column in columns {
        if let Expr::Path(ExprPath { path, .. }) = column {
            let segments: Vec<_> = path.segments.iter().collect();

            if segments.len() != 2 {
                return Err(Error::new_spanned(
                    column,
                    "Column references must be in the format Table::column",
                ));
            }

            let table_name = segments[0].ident.to_string();
            let column_name = segments[1].ident.to_string();

            column_refs.push(ColumnReference {
                table_name,
                column_name,
            });
        } else {
            return Err(Error::new_spanned(
                column,
                "Expected column reference in the format Table::column",
            ));
        }
    }

    Ok(column_refs)
}

/// Generate index name from struct name and columns
fn generate_index_name(struct_ident: &Ident, _columns: &[ColumnReference]) -> String {
    // Convert from CamelCase to snake_case
    let struct_name = struct_ident.to_string();
    let snake_case = heck::AsSnakeCase(struct_name).to_string();

    // If the name already looks like an index name, use it as is
    if snake_case.ends_with("_idx") || snake_case.ends_with("_index") {
        snake_case
    } else {
        // Otherwise append _idx
        format!("{}_idx", snake_case)
    }
}

/// Extract table type from column expression (similar to SQLite implementation)
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
