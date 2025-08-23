use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the Schema derive implementation
pub fn generate_schema_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract fields from the struct
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named_fields) => &named_fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "Schema can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "Schema can only be derived for structs",
            ));
        }
    };

    // Collect all fields (we'll determine table vs index at runtime)
    let all_fields: Vec<_> = fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
        .collect();

    let fields_new = all_fields.iter().map(|(name, ty)| {
        quote! {
            #name: #ty::new()
        }
    });

    // Generate Default implementation
    let field_defaults = all_fields.iter().map(|(name, _)| {
        quote! {
            #name: Default::default()
        }
    });

    let items_method = generate_items_method(&all_fields);

    // Generate IsInSchema implementations for all fields
    let is_in_schema_impls = all_fields.iter().map(|(_, ty)| {
        quote! {
            #[allow(non_local_definitions)]
            impl ::drizzle_rs::core::IsInSchema<#struct_name> for #ty {}
            #[allow(non_local_definitions)]
            impl ::drizzle_rs::core::IsInSchema<#struct_name> for &#ty {}
        }
    });

    // Collect field names and types for tuple destructuring
    let all_field_names: Box<_> = all_fields.iter().map(|(name, _)| *name).collect();
    let all_field_types: Box<_> = all_fields.iter().map(|(_, ty)| *ty).collect();

    let create_statements_impl = generate_create_statements_method(&all_fields);

    Ok(quote! {
        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_defaults,)*
                }
            }
        }

        impl #struct_name {
            pub const fn new() -> Self {
                Self {
                    #(#fields_new,)*
                }
            }

            /// Get all schema items (tables and indexes) in field order
            #items_method
        }

        // Implement IsInSchema for all field types (tables and indexes)
        #(#is_in_schema_impls)*

        // Implement SQLSchemaImpl trait
        impl ::drizzle_rs::core::SQLSchemaImpl for #struct_name {
            fn create_statements(&self) -> Vec<String> {
                #create_statements_impl
            }
        }

        // Implement tuple destructuring support
        impl From<#struct_name> for (#(#all_field_types,)*) {
            fn from(schema: #struct_name) -> Self {
                (#(schema.#all_field_names,)*)
            }
        }
    })
}

fn generate_create_statements_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    // Extract field names and types for easier iteration
    let field_names: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    // Generate different implementations based on available features
    #[cfg(feature = "sqlite")]
    let impl_tokens = quote! {
        let mut tables: Vec<(&str, String, &dyn ::drizzle_rs::core::SQLTableInfo)> = Vec::new();
        let mut indexes: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();

        // Collect all tables and indexes
        #(
            match <#field_types as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::TYPE {
                ::drizzle_rs::core::SQLSchemaType::Table(table_info) => {
                    let table_name = table_info.name();
                    let table_sql = <_ as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::sql(&self.#field_names).sql();
                    tables.push((table_name, table_sql, table_info));
                }
                ::drizzle_rs::core::SQLSchemaType::Index(index_info) => {
                    let index_sql = <_ as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::sql(&self.#field_names).sql();
                    let table_name = index_info.table().name();
                    indexes
                        .entry(table_name)
                        .or_insert_with(Vec::new)
                        .push(index_sql);
                }
                ::drizzle_rs::core::SQLSchemaType::View => {
                    // Views not implemented yet
                }
                ::drizzle_rs::core::SQLSchemaType::Trigger => {
                    // Triggers not implemented yet
                }
            }
        )*

        // Sort tables by dependencies (topological sort) then by name for deterministic ordering
        tables.sort_by(|a, b| {
            use std::cmp::Ordering;

            // Check if a depends on b
            let a_depends_on_b = a.2.dependencies().iter().any(|dep| dep.name() == b.0);
            // Check if b depends on a
            let b_depends_on_a = b.2.dependencies().iter().any(|dep| dep.name() == a.0);

            match (a_depends_on_b, b_depends_on_a) {
                (true, false) => Ordering::Greater,  // a comes after b
                (false, true) => Ordering::Less,     // a comes before b
                _ => a.0.cmp(b.0),                   // Same dependency level, sort by name
            }
        });

        // Build final SQL statements: tables in dependency order, then their indexes
        let mut sql_statements = Vec::<String>::new();
        for (table_name, table_sql, _) in tables {
            sql_statements.push(table_sql);

            // Add indexes for this table
            if let Some(table_indexes) = indexes.get(table_name) {
                for index_sql in table_indexes {
                    sql_statements.push(index_sql.clone());
                }
            }
        }

        sql_statements
    };

    #[cfg(not(feature = "sqlite"))]
    let impl_tokens = quote! {
        Vec::new()
    };

    impl_tokens
}

fn generate_items_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let (item_refs, item_types): (Vec<_>, Vec<_>) = fields
        .iter()
        .map(|(name, ty)| (quote! { &self.#name }, quote! { &#ty }))
        .unzip();

    quote! {
        pub fn items(&self) -> (#(#item_types,)*) {
            (#(#item_refs,)*)
        }
    }
}
