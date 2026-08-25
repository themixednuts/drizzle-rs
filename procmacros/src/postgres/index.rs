use crate::paths::{core as core_paths, ddl::postgres as ddl_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Expr, ExprPath, Ident, Meta, Result, Token, Type, parse::Parse};

/// Attributes for the `PostgresIndex` attribute macro
/// Syntax: #[`PostgresIndex`] or #[PostgresIndex(unique)] or #[PostgresIndex(unique, method = "btree")]
#[derive(Default)]
pub struct IndexAttributes {
    pub unique: bool,
    pub concurrent: bool,
    /// Explicit `method = "..."` (btree, hash, gin, gist, spgist, brin).
    /// `None` = not written in source. PostgreSQL's implicit default is
    /// btree and the DDL renderer treats `None` as btree, so the default is
    /// never materialized into `Some("btree")` — only an explicit
    /// `method = "..."` produces `Some`.
    pub method: Option<String>,
    pub tablespace: Option<String>,
    pub where_clause: Option<String>,
}

fn create_index_prefix(unique: bool, concurrent: bool, index_name: &str) -> String {
    let unique_kw = if unique { "UNIQUE " } else { "" };
    let concurrent_kw = if concurrent { "CONCURRENTLY " } else { "" };
    format!("CREATE {unique_kw}INDEX {concurrent_kw}\"{index_name}\" ON \"")
}

impl Parse for IndexAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = Self::default();

        if input.is_empty() {
            return Ok(attrs);
        }

        // `where = "..."` needs manual handling: `where` is a Rust keyword,
        // so `Meta::parse` rejects it as a path. Everything else goes
        // through `Meta` as before.
        let mut first = true;
        while !input.is_empty() {
            if !first {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
            }
            first = false;

            if input.peek(Token![where]) {
                input.parse::<Token![where]>()?;
                input.parse::<Token![=]>()?;
                let lit: syn::LitStr = input.parse()?;
                let value = lit.value();
                if value.trim().is_empty() {
                    return Err(Error::new_spanned(
                        lit,
                        "PostgreSQL partial-index predicate cannot be empty",
                    ));
                }
                if attrs.where_clause.is_some() {
                    return Err(Error::new_spanned(
                        lit,
                        "PostgreSQL index accepts only one partial-index predicate",
                    ));
                }
                attrs.where_clause = Some(value);
                continue;
            }

            let meta: Meta = input.parse()?;
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

/// Generates the `PostgresIndex` implementation
pub fn postgres_index_attr_macro(
    attr: &IndexAttributes,
    input: &DeriveInput,
) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_index = core_paths::sql_index();
    let drizzle_index = core_paths::drizzle_index();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let to_sql = core_paths::to_sql();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    // DDL type paths
    let index_def = ddl_paths::index_def();
    let index_column_def = ddl_paths::index_column_def();
    let postgres_item_ddl = ddl_paths::postgres_item_ddl();

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

    let method_modifier = attr
        .method
        .as_ref()
        .map_or_else(|| quote! {}, |method| quote! { .method(#method) });

    let where_modifier = attr.where_clause.as_ref().map_or_else(
        || quote! {},
        |where_clause| quote! { .where_clause(#where_clause) },
    );
    let conflict_where_clause = attr.where_clause.as_ref().map_or_else(
        || quote! {},
        |predicate| {
            quote! {
                fn conflict_where_clause(&self) -> ::std::option::Option<&'static str> {
                    ::std::option::Option::Some(#predicate)
                }
            }
        },
    );

    let is_unique = attr.unique;

    // Build compile-time SQL using concatcp! to reference the table's schema and name
    let create_prefix = create_index_prefix(attr.unique, attr.concurrent, &index_name);
    let dot_quote = "\".\"";
    let method_and_open = attr
        .method
        .as_ref()
        .map_or_else(|| "\"(".to_string(), |method| format!("\" USING {method}("));
    let column_sql_parts: Vec<TokenStream> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let prefix = if i > 0 { ", \"" } else { "\"" };
            let suffix = "\"";
            quote! {
                #prefix,
                {
                    const fn column_name<'a, C: #sql_schema<'a, &'static str, #postgres_value<'a>>>(_: &C) -> &'a str {
                        C::NAME
                    }
                    column_name(&#col)
                },
                #suffix
            }
        })
        .collect();
    let close = attr
        .where_clause
        .as_ref()
        .map_or_else(|| ")".to_string(), |wc| format!(") WHERE {wc}"));
    let const_format = crate::common::paths::const_format();
    let const_sql = quote! {
        #const_format::concatcp!(
            #create_prefix,
            <#table_type>::DDL_TABLE.schema,
            #dot_quote,
            <#table_type as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::NAME,
            #method_and_open,
            #(#column_sql_parts,)*
            #close
        )
    };

    // Generate the index struct and implementations
    let mut expanded = quote! {
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

            /// Returns the DDL SQL for creating this index.
            pub fn ddl_sql() -> &'static str {
                <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL
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

        impl #drizzle_index for #struct_ident {
            const INDEX_NAME: &'static str = #index_name;
            const COLUMN_NAMES: &'static [&'static str] = Self::COLUMN_NAMES;
            const IS_UNIQUE: bool = #is_unique;

            fn table_ref() -> &'static drizzle::core::TableRef {
                &<#table_type as drizzle::core::DrizzleTable>::TABLE_REF
            }
        }

        impl<'a> #sql_schema<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            const NAME: &'static str = #index_name;
            const TYPE: #postgres_schema_type = {
                #[allow(non_upper_case_globals)]
                static INDEX_INSTANCE: #struct_ident = #struct_ident::new();
                #postgres_schema_type::Index(&INDEX_INSTANCE)
            };
            const SQL: &'static str = #const_sql;
        }

        impl<'a> #to_sql<'a, #postgres_value<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::raw(Self::create_index_sql())
            }
        }

        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }

        // Snapshot DDL channel: exposes the full index definition (method /
        // where / concurrently) to `PostgresSchema::to_snapshot()`.
        impl #postgres_item_ddl for #struct_ident {
            const SNAPSHOT_INDEX: ::std::option::Option<#index_def> =
                ::std::option::Option::Some(Self::DDL_INDEX);
        }

    };

    // Generate ConflictTarget + NamedConstraint for unique indexes
    if attr.unique {
        let conflict_target = core_paths::conflict_target();
        let named_constraint = core_paths::named_constraint();
        expanded.extend(quote! {
            impl #conflict_target<#table_type> for #struct_ident {
                fn conflict_columns(&self) -> &'static [&'static str] { Self::COLUMN_NAMES }
                #conflict_where_clause
            }
            impl #named_constraint<#table_type> for #struct_ident {
                fn constraint_name(&self) -> &'static str { #index_name }
            }
        });
    }

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::{IndexAttributes, create_index_prefix};

    #[test]
    fn create_index_prefix_places_concurrently_after_index() {
        assert_eq!(
            create_index_prefix(true, true, "users_email_idx"),
            "CREATE UNIQUE INDEX CONCURRENTLY \"users_email_idx\" ON \""
        );
    }

    #[test]
    fn duplicate_partial_index_predicates_are_rejected() {
        assert!(
            syn::parse_str::<IndexAttributes>("where = \"active\", where = \"current\"").is_err()
        );
    }

    #[test]
    fn empty_partial_index_predicate_is_rejected() {
        assert!(syn::parse_str::<IndexAttributes>("where = \"  \"").is_err());
    }
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
        format!("{snake_case}_idx")
    }
}

/// Extract table type from column expression (similar to `SQLite` implementation)
fn extract_table_from_column(column: &Expr) -> Result<Type> {
    if let Expr::Path(expr_path) = column {
        let path = &expr_path.path;
        if path.segments.len() >= 2 {
            // Extract table name (first segment)
            let table_ident = &path.segments[0].ident;

            // Create table type
            let table_type = syn::parse_str::<Type>(&table_ident.to_string())
                .map_err(|_| Error::new_spanned(column, "invalid table name"))?;

            Ok(table_type)
        } else {
            Err(Error::new_spanned(
                column,
                "column must be in format Table::column",
            ))
        }
    } else {
        Err(Error::new_spanned(
            column,
            "Column must be a path expression",
        ))
    }
}
