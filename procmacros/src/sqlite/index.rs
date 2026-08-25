use crate::paths::{core as core_paths, ddl::sqlite as ddl_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Expr, Meta, Result, Token, Type, parse::Parse};

/// Attributes for the `SQLiteIndex` attribute macro.
/// Syntax: #[`SQLiteIndex`], #[SQLiteIndex(unique)], or
/// #[SQLiteIndex(unique, where = "deleted = 0")].
#[derive(Clone, Default)]
pub struct IndexAttributes {
    pub unique: bool,
    pub where_clause: Option<String>,
}

impl Parse for IndexAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = Self::default();

        if input.is_empty() {
            return Ok(attrs);
        }

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
                let predicate: syn::LitStr = input.parse()?;
                let value = predicate.value();
                if value.trim().is_empty() {
                    return Err(Error::new_spanned(
                        predicate,
                        "SQLite partial-index predicate cannot be empty",
                    ));
                }
                if attrs.where_clause.is_some() {
                    return Err(Error::new_spanned(
                        predicate,
                        "SQLite index accepts only one partial-index predicate",
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
                _ => {
                    return Err(Error::new_spanned(
                        meta,
                        "Unrecognized SQLite index attribute; supported: `unique`, `where = \"...\"`",
                    ));
                }
            }
        }

        Ok(attrs)
    }
}

/// Generates the `SQLiteIndex` implementation
pub fn sqlite_index_attr_macro(attr: IndexAttributes, input: &DeriveInput) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let is_unique = attr.unique;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_index = core_paths::sql_index();
    let drizzle_index = core_paths::drizzle_index();
    let _sql_table_info = core_paths::sql_table_info();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
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
                        input,
                        "SQLiteIndex can only be applied to tuple structs like `struct UserEmailIdx(User::email);`",
                    ));
                }
            }
        }
        _ => {
            return Err(Error::new_spanned(
                input,
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
    let where_modifier = attr.where_clause.as_ref().map_or_else(
        || quote! {},
        |predicate| quote! { .where_clause(#predicate) },
    );
    let where_clause = attr.where_clause.as_ref().map_or_else(
        || quote! { ::std::option::Option::None },
        |predicate| quote! { ::std::option::Option::Some(#predicate) },
    );

    // Build the const SQL using concatcp! to reference the table's TABLE_NAME
    let const_format = crate::common::paths::const_format();
    let unique_kw = if is_unique { "UNIQUE " } else { "" };
    let index_name_lit = &index_name;

    // Build the column list for the CREATE INDEX SQL
    // We need each column name from the column ZSTs
    let column_sql_parts: Vec<TokenStream> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let prefix = if i > 0 { ", \"" } else { "\"" };
            let suffix = "\"";
            quote! {
                #prefix,
                {
                    const fn column_name<'a, C: #sql_schema<'a, &'static str, #sqlite_value<'a>>>(_: &C) -> &'a str {
                        C::NAME
                    }
                    column_name(&#col)
                },
                #suffix
            }
        })
        .collect();

    let create_index_prefix = format!("CREATE {unique_kw}INDEX \"{index_name_lit}\" ON \"");
    let create_index_mid = "\" (";
    let create_index_suffix = attr.where_clause.as_ref().map_or_else(
        || quote! { ")" },
        |predicate| {
            let suffix = format!(") WHERE {predicate}");
            quote! { #suffix }
        },
    );
    let const_sql = quote! {
        #const_format::concatcp!(
            #create_index_prefix,
            <#table_type as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME,
            #create_index_mid,
            #(#column_sql_parts,)*
            #create_index_suffix
        )
    };

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
                <#table_type as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME,
                #index_name
            )
            .columns(Self::DDL_COLUMNS)
            #unique_modifier
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
                <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::SQL
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

        impl #drizzle_index for #struct_ident
        {
            const INDEX_NAME: &'static str = #index_name;
            const COLUMN_NAMES: &'static [&'static str] = Self::COLUMN_NAMES;
            const IS_UNIQUE: bool = #is_unique;
            const WHERE_CLAUSE: ::std::option::Option<&'static str> = #where_clause;

            fn table_ref() -> &'static drizzle::core::TableRef {
                &<#table_type as drizzle::core::DrizzleTable>::TABLE_REF
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
            const SQL: &'static str = #const_sql;
        }

        impl<'a> #to_sql<'a, #sqlite_value<'a>> for #struct_ident
        {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #sql::raw(Self::create_index_sql())
            }
        }

        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }

    };

    // Generate ConflictTarget for unique indexes. A standalone index is not a
    // named table constraint, so it must not implement NamedConstraint.
    if is_unique {
        let conflict_target = core_paths::conflict_target();
        expanded.extend(quote! {
            impl #conflict_target<#table_type> for #struct_ident {
                fn conflict_columns(&self) -> &'static [&'static str] { Self::COLUMN_NAMES }
                fn conflict_where_clause(&self) -> ::std::option::Option<&'static str> {
                    <Self as #drizzle_index>::WHERE_CLAUSE
                }
            }
        });
    }

    Ok(expanded)
}

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

#[cfg(test)]
mod tests {
    use super::{IndexAttributes, sqlite_index_attr_macro};
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn rejects_duplicate_raw_partial_predicates() {
        assert!(
            syn::parse_str::<IndexAttributes>("where = \"deleted = 0\", where = \"archived = 0\"")
                .is_err()
        );
    }

    #[test]
    fn rejects_empty_partial_index_predicate() {
        assert!(syn::parse_str::<IndexAttributes>("where = \"  \"").is_err());
    }

    #[test]
    fn unique_index_is_not_a_named_constraint() {
        let attrs = syn::parse_str::<IndexAttributes>("unique").unwrap();
        let input: DeriveInput = parse_quote! {
            pub struct UsersEmailIdx(Users::email);
        };
        let expanded = sqlite_index_attr_macro(attrs, &input).unwrap().to_string();

        assert!(expanded.contains("ConflictTarget"));
        assert!(!expanded.contains("NamedConstraint"));
    }
}
