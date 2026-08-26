use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Result};

/// Generate the runtime schema implementation for MySQL tables and indexes.
pub fn generate_mysql_schema_derive_impl(input: &DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[derive(MySQLSchema)] requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(MySQLSchema)] can only be applied to structs",
            ));
        }
    };

    let fields: Vec<_> = fields
        .iter()
        .map(|field| {
            field
                .ident
                .as_ref()
                .map(|name| (name, &field.ty))
                .ok_or_else(|| {
                    syn::Error::new_spanned(field, "#[derive(MySQLSchema)] fields must have names")
                })
        })
        .collect::<Result<_>>()?;
    let field_names: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    let sql_schema_impl = core_paths::sql_schema_impl();
    let validate_schema_item_foreign_keys = core_paths::validate_schema_item_foreign_keys();
    let create_statements = generate_create_statements_method(&fields);
    let table_refs = generate_schema_table_refs_method(&fields);
    let items = generate_items_method(&fields);
    let schema_has_table_impls = generate_schema_has_table_impls(struct_name, &fields);
    let foreign_key_assertions = generate_schema_fk_validation_asserts(
        &fields,
        struct_name,
        &validate_schema_item_foreign_keys,
    );

    Ok(quote! {
        impl ::core::marker::Copy for #struct_name {}

        impl ::core::clone::Clone for #struct_name {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl ::core::fmt::Debug for #struct_name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter
                    .debug_struct(stringify!(#struct_name))
                    #(.field(stringify!(#field_names), &self.#field_names))*
                    .finish()
            }
        }

        impl ::core::default::Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_names: ::core::default::Default::default(),)*
                }
            }
        }

        impl #struct_name {
            pub const fn new() -> Self {
                Self {
                    #(#field_names: #field_types::new(),)*
                }
            }

            #items
        }

        impl #sql_schema_impl for #struct_name {
            fn table_refs(&self) -> &'static [&'static drizzle::core::TableRef] {
                #table_refs
            }

            fn create_statements(
                &self,
            ) -> ::std::result::Result<
                impl ::std::iter::Iterator<Item = ::std::string::String>,
                drizzle::error::DrizzleError,
            > {
                let statements: ::std::vec::Vec<::std::string::String> = { #create_statements };
                ::std::result::Result::Ok(statements.into_iter())
            }
        }

        impl ::std::convert::From<#struct_name> for (#(#field_types,)*) {
            fn from(schema: #struct_name) -> Self {
                (#(schema.#field_names,)*)
            }
        }

        #schema_has_table_impls
        #foreign_key_assertions
    })
}

fn generate_create_statements_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql_index_info = core_paths::sql_index_info();
    let table_ref = core_paths::table_ref();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();
    let mysql_value = quote!(drizzle::mysql::values::MySQLValue);
    let mysql_schema_type = quote!(drizzle::mysql::common::MySQLSchemaType);

    quote! {
        let mut tables: ::std::vec::Vec<(
            ::std::string::String,
            ::std::string::String,
            &'static #table_ref,
        )> = ::std::vec::Vec::new();
        let mut indexes: ::std::collections::HashMap<
            ::std::string::String,
            ::std::vec::Vec<::std::string::String>,
        > = ::std::collections::HashMap::new();
        let mut index_keys = ::std::collections::HashSet::<::std::string::String>::new();

        #(
            match <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::TYPE {
                #mysql_schema_type::Table(table_ref) => {
                    tables.push((
                        table_ref.qualified_name.to_string(),
                        <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::SQL.to_string(),
                        table_ref,
                    ));
                }
                #mysql_schema_type::Index(index_info) => {
                    let table_name = #sql_index_info::table(index_info).qualified_name.to_string();
                    let index_name = #sql_index_info::name(index_info);
                    if !index_keys.insert(::std::format!("{table_name}::{index_name}")) {
                        return ::std::result::Result::Err(
                            drizzle::error::DrizzleError::Statement(
                                ::std::format!(
                                    "Duplicate index '{index_name}' on table '{table_name}' in MySQLSchema",
                                )
                                .into(),
                            ),
                        );
                    }
                    indexes
                        .entry(table_name)
                        .or_insert_with(::std::vec::Vec::new)
                        .push(
                            <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::SQL.to_string(),
                        );
                }
            }
        )*

        tables.sort_by(|left, right| left.0.cmp(&right.0));
        let table_names: ::std::collections::HashSet<::std::string::String> =
            tables.iter().map(|(name, _, _)| name.clone()).collect();
        if table_names.len() != tables.len() {
            return ::std::result::Result::Err(
                drizzle::error::DrizzleError::Statement(
                    "Duplicate table names detected in MySQLSchema".into(),
                ),
            );
        }

        if let ::std::option::Option::Some(orphan) =
            indexes.keys().find(|table_name| !table_names.contains(*table_name))
        {
            return ::std::result::Result::Err(
                drizzle::error::DrizzleError::Statement(
                    ::std::format!(
                        "MySQLSchema contains an index for table '{orphan}', but not the table itself",
                    )
                    .into(),
                ),
            );
        }

        let mut indegree = ::std::collections::HashMap::<::std::string::String, usize>::new();
        let mut reverse_edges = ::std::collections::HashMap::<
            ::std::string::String,
            ::std::vec::Vec<::std::string::String>,
        >::new();
        for (table_name, _, table_ref) in &tables {
            indegree.entry(table_name.clone()).or_insert(0);
            let mut table_dependencies = ::std::collections::HashSet::<::std::string::String>::new();
            for foreign_key in table_ref.foreign_keys {
                let dependency = if foreign_key.target_schema.is_empty() {
                    foreign_key.target_table.to_string()
                } else {
                    ::std::format!(
                        "{}.{}",
                        foreign_key.target_schema,
                        foreign_key.target_table,
                    )
                };
                if dependency == *table_name || !table_names.contains(&dependency) {
                    continue;
                }
                if !table_dependencies.insert(dependency.clone()) {
                    continue;
                }
                *indegree
                    .get_mut(table_name)
                    .expect("every MySQL table has an indegree entry") += 1;
                reverse_edges
                    .entry(dependency)
                    .or_insert_with(::std::vec::Vec::new)
                    .push(table_name.clone());
            }
        }

        let mut ready: ::std::collections::BTreeSet<::std::string::String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(name, _)| name.clone())
            .collect();
        let mut ordered_names = ::std::vec::Vec::<::std::string::String>::with_capacity(tables.len());
        while let ::std::option::Option::Some(next) = ready.pop_first() {
            ordered_names.push(next.clone());
            if let ::std::option::Option::Some(children) = reverse_edges.get(&next) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("every dependent MySQL table has an indegree entry");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }

        if ordered_names.len() != tables.len() {
            let mut remaining: ::std::vec::Vec<_> = indegree
                .iter()
                .filter(|(_, degree)| **degree > 0)
                .map(|(name, _)| name.clone())
                .collect();
            remaining.sort_unstable();
            return ::std::result::Result::Err(
                drizzle::error::DrizzleError::Statement(
                    ::std::format!(
                        "Cyclic table dependency detected in MySQLSchema: {}",
                        remaining.join(", "),
                    )
                    .into(),
                ),
            );
        }

        let mut table_sql = ::std::collections::HashMap::<
            ::std::string::String,
            ::std::string::String,
        >::with_capacity(tables.len());
        for (table_name, sql, _) in tables {
            table_sql.insert(table_name, sql);
        }

        let mut statements = ::std::vec::Vec::<::std::string::String>::new();
        for table_name in ordered_names {
            statements.push(
                table_sql
                    .remove(&table_name)
                    .expect("ordered MySQL table must have SQL"),
            );
            if let ::std::option::Option::Some(table_indexes) = indexes.get(&table_name) {
                statements.extend(table_indexes.iter().cloned());
            }
        }
        statements
    }
}

fn generate_items_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let item_refs = fields.iter().map(|(name, _)| quote!(&self.#name));
    let item_types = fields.iter().map(|(_, ty)| quote!(&#ty));

    quote! {
        pub fn items(&self) -> (#(#item_types,)*) {
            (#(#item_refs,)*)
        }
    }
}

fn generate_schema_table_refs_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let table_ref = core_paths::table_ref();
    let schema_item_tables = core_paths::schema_item_tables();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();
    let field_count = fields.len();

    quote! {
        static TABLE_REF_OPTIONS: [::core::option::Option<&'static #table_ref>; #field_count] = [
            #(<#field_types as #schema_item_tables>::TABLE_REF_CONST,)*
        ];
        const TABLE_REF_COUNT: usize = {
            let mut count = 0;
            let mut index = 0;
            while index < #field_count {
                if TABLE_REF_OPTIONS[index].is_some() {
                    count += 1;
                }
                index += 1;
            }
            count
        };
        static TABLE_REFS: [&'static #table_ref; TABLE_REF_COUNT] = {
            static EMPTY_TABLE_REF: #table_ref = #table_ref::sql("", &[]);
            let mut result = [&EMPTY_TABLE_REF; TABLE_REF_COUNT];
            let mut input = 0;
            let mut output = 0;
            while input < #field_count {
                if let ::core::option::Option::Some(table_ref) = TABLE_REF_OPTIONS[input] {
                    result[output] = table_ref;
                    output += 1;
                }
                input += 1;
            }
            result
        };
        &TABLE_REFS
    }
}

fn generate_schema_has_table_impls(
    schema: &syn::Ident,
    fields: &[(&syn::Ident, &syn::Type)],
) -> TokenStream {
    let schema_has_table = core_paths::schema_has_table();
    let mut seen = HashSet::new();
    let unique_types: Vec<_> = fields
        .iter()
        .map(|(_, ty)| *ty)
        .filter(|ty| seen.insert(quote!(#ty).to_string()))
        .collect();

    quote! {
        #(impl #schema_has_table<#unique_types> for #schema {})*
    }
}

fn generate_schema_fk_validation_asserts(
    fields: &[(&syn::Ident, &syn::Type)],
    schema: &syn::Ident,
    validate_schema_item_foreign_keys: &TokenStream,
) -> TokenStream {
    let field_types = fields.iter().map(|(_, ty)| *ty);

    quote! {
        const _: () = {
            const fn assert_schema_item<Item>()
            where
                Item: #validate_schema_item_foreign_keys<#schema>,
            {
            }

            #(assert_schema_item::<#field_types>();)*
        };
    }
}
