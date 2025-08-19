use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result, Type};

/// Generates the Schema derive implementation
pub fn generate_schema_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let struct_vis = &input.vis;

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

    let mut all_fields = Vec::new();

    // Collect all fields (we'll determine table vs index at runtime)
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        all_fields.push((field_name, field_type));
    }

    // Generate field accessors for the schema struct
    let field_definitions = all_fields.iter().map(|(name, ty)| {
        quote! {
            pub #name: #ty
        }
    });

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

    // Generate methods that filter by SQLSchemaType
    let create_all_impl = generate_create_all_method(&all_fields);
    let tables_method = generate_tables_method(&all_fields);
    let indexes_method = generate_indexes_method(&all_fields);

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

    let create_signature = quote! {};

    // Precompute the create method signature based on feature flags
    #[cfg(feature = "rusqlite")]
    let create_signature = quote! {
        fn create(&self, conn: &::rusqlite::Connection) -> Result<(), ::drizzle_rs::error::DrizzleError>
    };

    #[cfg(feature = "libsql")]
    let create_signature = quote! {
        async fn create(&self, conn: &::libsql::Connection) -> Result<(), ::drizzle_rs::error::DrizzleError>
    };

    #[cfg(feature = "turso")]
    let create_signature = quote! {
        async fn create(&self, conn: &::turso::Connection) -> Result<(), ::drizzle_rs::error::DrizzleError>
    };

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

            /// Create all database objects (tables, indexes) in the correct order
            #create_signature {
                #create_all_impl
            }

            /// Get all table objects in field order
            #tables_method

            /// Get all index objects in field order
            #indexes_method
        }

        // Implement IsInSchema for all field types (tables and indexes)
        #(#is_in_schema_impls)*

        // Implement SQLSchemaImpl trait
        impl ::drizzle_rs::core::SQLSchemaImpl for #struct_name {
            #create_signature {
                #create_all_impl
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

fn generate_create_all_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let table_sql_collection = fields.iter().map(|(name, ty)| {
        quote! {
            if <#ty as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::TYPE == ::drizzle_rs::core::SQLSchemaType::Table {
                sql_statements.push(<#ty as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::sql(&self.#name).sql());
            }
        }
    });

    let index_sql_collection = fields.iter().map(|(name, ty)| {
        quote! {
            if <#ty as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::TYPE == ::drizzle_rs::core::SQLSchemaType::Index {
                sql_statements.push(<#ty as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::sql(&self.#name).sql());
            }
        }
    });

    #[cfg(feature = "rusqlite")]
    let execute_batch = quote! {
        if !sql_statements.is_empty() {
            let batch_sql = sql_statements.join(";");
            conn.execute_batch(&batch_sql)?;
        }
    };

    #[cfg(any(feature = "libsql", feature = "turso"))]
    let execute_batch = quote! {
        if !sql_statements.is_empty() {
            let batch_sql = sql_statements.join(";");
            conn.execute_batch(&batch_sql).await?;
        }
    };

    #[cfg(not(any(feature = "libsql", feature = "rusqlite", feature = "turso")))]
    let execute_batch = quote! {};

    quote! {
        let mut sql_statements = Vec::<String>::new();

        // Collect table SQL first (in field order)
        #(#table_sql_collection)*

        // Then collect index SQL (in field order)
        #(#index_sql_collection)*

        // Execute all statements as a batch
        #execute_batch

        Ok(())
    }
}

fn generate_tables_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let table_refs: Vec<_> = fields
        .iter()
        .map(|(_, ty)| {
            quote! { #ty::new() }
        })
        .collect();

    let table_types: Vec<_> = fields
        .iter()
        .map(|(_, ty)| {
            quote! { #ty }
        })
        .collect();

    quote! {
        pub fn tables(&self) -> (#(#table_types,)*) {
            (#(#table_refs,)*)
        }
    }
}

fn generate_indexes_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let index_refs: Vec<_> = fields
        .iter()
        .map(|(name, _)| {
            quote! { &self.#name }
        })
        .collect();

    let index_types: Vec<_> = fields
        .iter()
        .map(|(_, ty)| {
            quote! { &#ty }
        })
        .collect();

    quote! {
        pub fn indexes(&self) -> (#(#index_types,)*) {
            (#(#index_refs,)*)
        }
    }
}
