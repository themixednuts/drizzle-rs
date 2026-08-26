use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DeriveInput, Error, Expr, Meta, Result, Token, Type, parse::Parse};

/// Options accepted by `MySQLIndex`.
#[derive(Clone, Default)]
pub struct IndexAttributes {
    pub unique: bool,
    pub using: Option<String>,
    pub algorithm: Option<String>,
    pub lock: Option<String>,
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
                Meta::NameValue(name_value)
                    if name_value
                        .path
                        .get_ident()
                        .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("using")) =>
                {
                    set_option(&mut attrs.using, &name_value, "using", &["btree", "hash"])?;
                }
                Meta::NameValue(name_value)
                    if name_value.path.get_ident().is_some_and(|ident| {
                        ident.to_string().eq_ignore_ascii_case("algorithm")
                    }) =>
                {
                    set_option(
                        &mut attrs.algorithm,
                        &name_value,
                        "algorithm",
                        &["default", "inplace", "copy"],
                    )?;
                }
                Meta::NameValue(name_value)
                    if name_value
                        .path
                        .get_ident()
                        .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("lock")) =>
                {
                    set_option(
                        &mut attrs.lock,
                        &name_value,
                        "lock",
                        &["default", "none", "shared", "exclusive"],
                    )?;
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
                        "unsupported MySQL index option; accepted options are `unique`, `using`, `algorithm`, and `lock`",
                    ));
                }
            }
        }

        Ok(attrs)
    }
}

fn set_option(
    slot: &mut Option<String>,
    name_value: &syn::MetaNameValue,
    name: &str,
    accepted: &[&str],
) -> Result<()> {
    let syn::Expr::Lit(literal) = &name_value.value else {
        return Err(Error::new_spanned(
            &name_value.value,
            format!("MySQL index `{name}` expects a string literal"),
        ));
    };
    let syn::Lit::Str(value) = &literal.lit else {
        return Err(Error::new_spanned(
            &literal.lit,
            format!("MySQL index `{name}` expects a string literal"),
        ));
    };
    if slot.is_some() {
        return Err(Error::new_spanned(
            name_value,
            format!("MySQLIndex accepts `{name}` only once"),
        ));
    }
    let normalized = value.value().to_ascii_lowercase();
    if !accepted.contains(&normalized.as_str()) {
        return Err(Error::new_spanned(
            value,
            format!(
                "invalid MySQL index `{name}`; supported values: {}",
                accepted.join(", ")
            ),
        ));
    }
    *slot = Some(normalized);
    Ok(())
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
    let using_sql = attr
        .using
        .as_deref()
        .map(|using| format!(" USING {}", using.to_ascii_uppercase()))
        .unwrap_or_default();
    let create_prefix = format!("CREATE {unique_keyword}INDEX `{index_name}`{using_sql} ON ");
    let algorithm_sql = attr
        .algorithm
        .as_deref()
        .map(|algorithm| format!(" ALGORITHM={}", algorithm.to_ascii_uppercase()))
        .unwrap_or_default();
    let lock_sql = attr
        .lock
        .as_deref()
        .map(|lock| format!(" LOCK={}", lock.to_ascii_uppercase()))
        .unwrap_or_default();
    let ddl_suffix = format!("){algorithm_sql}{lock_sql};");

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
            #ddl_suffix
        )
    };

    let method = match attr.using.as_deref() {
        Some("btree") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexMethod::BTree
        )),
        Some("hash") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexMethod::Hash
        )),
        None => quote!(::core::option::Option::None),
        Some(_) => unreachable!("validated MySQL index method"),
    };
    let algorithm = match attr.algorithm.as_deref() {
        Some("default") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexAlgorithm::Default
        )),
        Some("inplace") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexAlgorithm::Inplace
        )),
        Some("copy") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexAlgorithm::Copy
        )),
        None => quote!(::core::option::Option::None),
        Some(_) => unreachable!("validated MySQL index algorithm"),
    };
    let lock = match attr.lock.as_deref() {
        Some("default") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexLock::Default
        )),
        Some("none") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexLock::None
        )),
        Some("shared") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexLock::Shared
        )),
        Some("exclusive") => quote!(::core::option::Option::Some(
            drizzle::mysql::index::MySQLIndexLock::Exclusive
        )),
        None => quote!(::core::option::Option::None),
        Some(_) => unreachable!("validated MySQL index lock"),
    };

    Ok(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #struct_vis struct #struct_ident;

        impl #struct_ident {
            pub const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];
            pub const DDL_SQL: &'static str = #const_sql;
            pub const METHOD: ::core::option::Option<drizzle::mysql::index::MySQLIndexMethod> = #method;
            pub const ALGORITHM: ::core::option::Option<drizzle::mysql::index::MySQLIndexAlgorithm> = #algorithm;
            pub const LOCK: ::core::option::Option<drizzle::mysql::index::MySQLIndexLock> = #lock;

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

        impl drizzle::mysql::index::MySQLIndexMetadata for #struct_ident {
            const METHOD: ::core::option::Option<drizzle::mysql::index::MySQLIndexMethod> = Self::METHOD;
            const ALGORITHM: ::core::option::Option<drizzle::mysql::index::MySQLIndexAlgorithm> = Self::ALGORITHM;
            const LOCK: ::core::option::Option<drizzle::mysql::index::MySQLIndexLock> = Self::LOCK;
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
