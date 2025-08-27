use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Expr, ExprPath, Ident, Meta, Result, Token, Type, parse::Parse};

/// Attributes for the PostgresIndex attribute macro
/// Syntax: #[PostgresIndex] or #[PostgresIndex(unique)] or #[PostgresIndex(unique, method = "btree")]
pub struct IndexAttributes {
    pub unique: bool,
    pub concurrent: bool,
    pub if_not_exists: bool,
    pub method: Option<String>, // btree, hash, gin, gist, spgist, brin
    pub tablespace: Option<String>,
    pub where_clause: Option<String>,
}

impl Default for IndexAttributes {
    fn default() -> Self {
        Self {
            unique: false,
            concurrent: false,
            if_not_exists: false,
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
                Meta::Path(path) if path.is_ident("if_not_exists") => {
                    attrs.if_not_exists = true;
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
                         - if_not_exists: Add IF NOT EXISTS clause\n\
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

    // Parse column references
    let column_info = parse_column_references(&columns)?;

    // Extract table type from first column (similar to SQLite implementation)
    let table_type = if let Some(first_column) = columns.first() {
        extract_table_from_column(first_column)?
    } else {
        return Err(Error::new_spanned(
            struct_ident,
            "Index must have at least one column",
        ));
    };

    // Set unique flag based on attributes
    let unique_flag = attr.unique;

    // Generate index name if not provided
    let index_name = generate_index_name(struct_ident, &column_info);

    // Generate CREATE INDEX SQL
    let create_index_sql = generate_create_index_sql(&index_name, &column_info, &attr);

    // Generate the index struct and implementations
    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #struct_vis struct #struct_ident;

        impl #struct_ident {
            pub const fn new() -> Self {
                Self
            }

            pub fn index_name(&self) -> &'static str {
                #index_name
            }

            pub fn create_index_sql(&self) -> &'static str {
                #create_index_sql
            }
        }

        impl Default for #struct_ident {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<'a> ::drizzle::core::SQLIndex<'a, ::drizzle::postgres::common::PostgresSchemaType, ::drizzle::postgres::values::PostgresValue<'a>> for #struct_ident {
            type Table = #table_type;
        }

        impl ::drizzle::core::SQLIndexInfo for #struct_ident {
            fn table(&self) -> &dyn ::drizzle::core::SQLTableInfo {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #table_type = #table_type::new();
                &TABLE_INSTANCE
            }

            fn name(&self) -> &'static str {
                #index_name
            }

            fn is_unique(&self) -> bool {
                #unique_flag
            }
        }

        impl<'a> ::drizzle::core::SQLSchema<'a, ::drizzle::postgres::common::PostgresSchemaType, ::drizzle::postgres::values::PostgresValue<'a>> for #struct_ident {
            const NAME: &'a str = #index_name;
            const TYPE: ::drizzle::postgres::common::PostgresSchemaType = {
                #[allow(non_upper_case_globals)]
                static INDEX_INSTANCE: #struct_ident = #struct_ident::new();
                ::drizzle::postgres::common::PostgresSchemaType::Index(&INDEX_INSTANCE)
            };
            const SQL: ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> = ::drizzle::core::SQL::empty();

            fn sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> {
                self.to_sql()
            }
        }

        impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> {
                ::drizzle::core::SQL::text(#create_index_sql)
            }
        }
    };

    Ok(expanded)
}

/// Information about a column reference in an index
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
fn generate_index_name(struct_ident: &Ident, columns: &[ColumnReference]) -> String {
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

/// Generate CREATE INDEX SQL statement
fn generate_create_index_sql(
    index_name: &str,
    columns: &[ColumnReference],
    attr: &IndexAttributes,
) -> String {
    let mut sql = String::new();

    // CREATE INDEX clause
    sql.push_str("CREATE ");
    if attr.unique {
        sql.push_str("UNIQUE ");
    }
    sql.push_str("INDEX ");
    if attr.concurrent {
        sql.push_str("CONCURRENTLY ");
    }
    if attr.if_not_exists {
        sql.push_str("IF NOT EXISTS ");
    }
    sql.push_str(index_name);

    // Table name (assume all columns are from the same table)
    if let Some(first_column) = columns.first() {
        sql.push_str(&format!(" ON {}", first_column.table_name.to_lowercase()));
    }

    // Index method
    if let Some(method) = &attr.method {
        sql.push_str(&format!(" USING {}", method.to_uppercase()));
    }

    // Column list
    let column_names: Vec<String> = columns.iter().map(|col| col.column_name.clone()).collect();
    sql.push_str(&format!(" ({})", column_names.join(", ")));

    // Tablespace
    if let Some(tablespace) = &attr.tablespace {
        sql.push_str(&format!(" TABLESPACE {}", tablespace));
    }

    // WHERE clause for partial indexes
    if let Some(where_clause) = &attr.where_clause {
        sql.push_str(&format!(" WHERE {}", where_clause));
    }

    sql
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
