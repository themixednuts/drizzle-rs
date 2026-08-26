use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DeriveInput, Error, Expr, Meta, Result, Token, Type, parse::Parse};

/// Options accepted by `MySQLIndex`.
#[derive(Clone, Default)]
pub struct IndexAttributes {
    pub unique: bool,
}

impl Parse for IndexAttributes {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let mut attrs = Self::default();
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
                let keyword = input.parse::<Token![where]>()?;
                return Err(Error::new_spanned(
                    keyword,
                    "MySQL does not support partial indexes; remove `where = ...`",
                ));
            }

            let meta: Meta = input.parse()?;
            match meta {
                Meta::Path(path)
                    if path
                        .get_ident()
                        .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("unique")) =>
                {
                    if attrs.unique {
                        return Err(Error::new_spanned(
                            path,
                            "MySQLIndex accepts `unique` only once",
                        ));
                    }
                    attrs.unique = true;
                }
                Meta::Path(path)
                    if path.get_ident().is_some_and(|ident| {
                        matches!(
                            ident.to_string().to_ascii_lowercase().as_str(),
                            "concurrent" | "concurrently"
                        )
                    }) =>
                {
                    return Err(Error::new_spanned(
                        path,
                        "MySQL does not support `concurrent` indexes",
                    ));
                }
                unsupported => {
                    return Err(Error::new_spanned(
                        unsupported,
                        "unsupported MySQL index option; only `unique` is accepted",
                    ));
                }
            }
        }

        Ok(attrs)
    }
}

/// Generate the driver-neutral implementation for a MySQL index declaration.
pub fn mysql_index_attr_macro(attr: IndexAttributes, input: &DeriveInput) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let columns = index_columns(input)?;
    let table_type = extract_table_from_column(columns.first().ok_or_else(|| {
        Error::new_spanned(struct_ident, "MySQLIndex requires at least one column")
    })?)?;

    let expected_table = table_type.to_token_stream().to_string();
    for column in &columns {
        let column_table = extract_table_from_column(column)?;
        if column_table.to_token_stream().to_string() != expected_table {
            return Err(Error::new_spanned(
                column,
                "all columns in a MySQL index must belong to the same table",
            ));
        }
    }

    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_index = core_paths::sql_index();
    let drizzle_index = core_paths::drizzle_index();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let to_sql = core_paths::to_sql();
    let const_format = crate::common::paths::const_format();

    let mysql_value = quote!(drizzle::mysql::values::MySQLValue);
    let mysql_schema_type = quote!(drizzle::mysql::common::MySQLSchemaType);
    let mysql_index_column = quote!(drizzle::mysql::traits::MySQLIndexColumn);
    let mysql_column = quote!(drizzle::mysql::traits::MySQLColumn);
    let mysql_table = quote!(drizzle::mysql::traits::MySQLTable);
    let index_name = heck::AsSnakeCase(struct_ident.to_string()).to_string();
    let unique_keyword = if attr.unique { "UNIQUE " } else { "" };
    let create_prefix = format!("CREATE {unique_keyword}INDEX `{index_name}` ON ");

    let column_names: Vec<_> = columns
        .iter()
        .map(|column| {
            quote! {{
                const fn column_name<'a, C>(_: &C) -> &'a str
                where
                    C: #sql_schema<'a, &'static str, #mysql_value<'a>> + #mysql_index_column,
                {
                    C::NAME
                }
                column_name(&#column)
            }}
        })
        .collect();
    let column_ddl_names: Vec<_> = columns
        .iter()
        .map(|column| {
            quote! {{
                const fn column_ddl_name<C>(_: &C) -> &'static str
                where
                    C: #mysql_column<'static> + #mysql_index_column,
                {
                    C::DDL_NAME
                }
                column_ddl_name(&#column)
            }}
        })
        .collect();
    let column_sql_parts: Vec<_> = column_ddl_names
        .iter()
        .enumerate()
        .map(|(index, column_name)| {
            let prefix = if index == 0 { "" } else { ", " };
            quote!(#prefix, #column_name)
        })
        .collect();
    let is_unique = attr.unique;
    let const_sql = quote! {
        #const_format::concatcp!(
            #create_prefix,
            <#table_type as #mysql_table<'static>>::DDL_QUALIFIED_NAME,
            "(",
            #(#column_sql_parts,)*
            ");"
        )
    };

    Ok(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #struct_vis struct #struct_ident;

        impl #struct_ident {
            pub const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];
            pub const DDL_SQL: &'static str = #const_sql;

            pub const fn new() -> Self {
                Self
            }

            pub fn create_index_sql() -> ::std::string::String {
                Self::DDL_SQL.to_owned()
            }

            pub const fn ddl_sql() -> &'static str {
                Self::DDL_SQL
            }
        }

        impl ::core::default::Default for #struct_ident {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<'a> #sql_index<'a, #mysql_schema_type, #mysql_value<'a>> for #struct_ident {
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

        impl<'a> #sql_schema<'a, #mysql_schema_type, #mysql_value<'a>> for #struct_ident {
            const NAME: &'static str = #index_name;
            const TYPE: #mysql_schema_type = {
                static INDEX: #struct_ident = #struct_ident::new();
                #mysql_schema_type::Index(&INDEX)
            };
            const SQL: &'static str = Self::DDL_SQL;
        }

        impl<'a> #to_sql<'a, #mysql_value<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #mysql_value<'a>> {
                #sql::raw(Self::DDL_SQL)
            }
        }

        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }
    })
}

fn index_columns(input: &DeriveInput) -> Result<Vec<Expr>> {
    let syn::Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "MySQLIndex can only be applied to a tuple struct",
        ));
    };
    let syn::Fields::Unnamed(fields) = &data.fields else {
        return Err(Error::new_spanned(
            input,
            "MySQLIndex requires a tuple struct such as `struct UsersEmailIdx(Users::email);`",
        ));
    };

    fields
        .unnamed
        .iter()
        .map(|field| match &field.ty {
            Type::Path(path) if path.qself.is_none() => Ok(Expr::Path(syn::ExprPath {
                attrs: Vec::new(),
                qself: None,
                path: path.path.clone(),
            })),
            _ => Err(Error::new_spanned(
                &field.ty,
                "MySQL index columns must be paths such as `Users::email`",
            )),
        })
        .collect()
}

fn extract_table_from_column(column: &Expr) -> Result<Type> {
    let Expr::Path(column) = column else {
        return Err(Error::new_spanned(
            column,
            "MySQL index columns must be paths such as `Users::email`",
        ));
    };
    if column.path.segments.len() != 2 {
        return Err(Error::new_spanned(
            column,
            "MySQL index columns must use the form `Table::column`",
        ));
    }

    let table = &column.path.segments[0].ident;
    syn::parse_str(&table.to_string()).map_err(|_| Error::new_spanned(table, "invalid table name"))
}
