use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the PostgresSchema derive implementation
pub fn generate_postgres_schema_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract fields from the struct
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named_fields) => &named_fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "PostgresSchema can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "PostgresSchema can only be derived for structs",
            ));
        }
    };

    // Collect all fields (we'll determine table vs index vs enum at runtime)
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

            /// Get all schema items (tables, indexes, and enums) in field order
            #items_method
        }

        // Implement SQLSchemaImpl trait
        impl SQLSchemaImpl for #struct_name {
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

    quote! {
        let mut tables: Vec<(&str, String, &dyn SQLTableInfo)> = Vec::new();
        let mut indexes: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
        let mut enums: Vec<String> = Vec::new();

        // Collect all tables, indexes, and enums
        #(
            match <#field_types as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::TYPE {
                PostgresSchemaType::Table(table_info) => {
                    let table_name = table_info.name();
                    let table_sql = <_ as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::sql(&self.#field_names).sql();
                    tables.push((table_name, table_sql, table_info));
                }
                PostgresSchemaType::Index(index_info) => {
                    let index_sql = <_ as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::sql(&self.#field_names).sql();
                    let table_name = index_info.table().name();
                    indexes
                        .entry(table_name)
                        .or_insert_with(Vec::new)
                        .push(index_sql);
                }
                PostgresSchemaType::Enum(enum_info) => {
                    let enum_sql = enum_info.create_type_sql();
                    enums.push(enum_sql);
                }
                PostgresSchemaType::View => {
                    // Views not implemented yet
                }
                PostgresSchemaType::Trigger => {
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

        // Build final SQL statements: enums first, then tables in dependency order, then their indexes
        let mut sql_statements = Vec::<String>::new();

        // Add all enums first (they must be created before tables that use them)
        sql_statements.extend(enums);

        // Add tables and their indexes
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
    }
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
