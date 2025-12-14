use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Error, Expr, ExprPath, Ident, Meta, Result, Token, Type, parse::Parse};

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

    // Generate SQL for CREATE INDEX
    let unique_keyword = if is_unique { "UNIQUE " } else { "" };

    let zst_idents = columns
        .iter()
        .map(|col| match col {
            Expr::Path(p) => extract_zst_ident(p),
            _ => Err(syn::Error::new_spanned(
                col,
                "Expected column path like User::id",
            )),
        })
        .collect::<Result<Box<_>>>()?;

    // In the implementation, we'll need to use the actual column constants
    let column_name_exprs: Vec<_> = columns
        .iter()
        .map(|col| {
            quote! { #col.name() }
        })
        .collect();

    Ok(quote! {
        #[derive(Default, Debug, Clone, PartialEq)]
        #struct_vis struct #struct_ident(#(#zst_idents),*);

        impl #struct_ident {
            pub const fn new() -> Self {
                Self( #(#zst_idents::new(),)* )
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
        }

        impl<'a> #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>> for #struct_ident
        {
            const NAME: &'a str = #index_name;
            const TYPE: #sqlite_schema_type = {
                #[allow(non_upper_case_globals)]
                static INDEX_INSTANCE: #struct_ident = #struct_ident::new();
                #sqlite_schema_type::Index(&INDEX_INSTANCE)
            };
            const SQL: &'static str = "";

            fn sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #to_sql::to_sql(self)
            }
        }

        impl<'a> #to_sql<'a, #sqlite_value<'a>> for #struct_ident
        {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                let table_name = <#table_type as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME;
                let column_names = ::std::vec![#(#column_name_exprs),*];
                let column_list = column_names.join(", ");
                let sql = ::std::format!("CREATE {}INDEX \"{}\" ON \"{}\" ({})", #unique_keyword, #index_name, table_name, column_list);
                #sql::raw(sql)
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
fn extract_zst_ident(expr: &ExprPath) -> syn::Result<Ident> {
    let segments = &expr.path.segments;
    if segments.len() != 2 {
        return Err(syn::Error::new_spanned(
            &expr.path,
            "Expected column path like `User::id`",
        ));
    }

    let struct_ident = &segments[0].ident;
    let field_ident = &segments[1].ident;

    // Convert field to PascalCase
    let field_pascal_case = &field_ident.to_string().to_upper_camel_case();

    // Build ZST ident
    Ok(format_ident!("{}{}", struct_ident, field_pascal_case))
}
