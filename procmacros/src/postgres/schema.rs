use crate::paths::{core as core_paths, migrations as mig_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the PostgresSchema derive implementation
pub fn generate_postgres_schema_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_schema_impl = core_paths::sql_schema_impl();
    let sql_table_info = core_paths::sql_table_info();
    let _sql_column_info = core_paths::sql_column_info();
    let sql_index_info = core_paths::sql_index_info();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let postgres_table_info = postgres_paths::postgres_table_info();
    let _postgres_column_info = postgres_paths::postgres_column_info();

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

    // For Schema trait to_snapshot
    let field_types_for_snapshot: Vec<_> = all_fields.iter().map(|(_, ty)| *ty).collect();

    // Get migrations paths
    let mig_schema = mig_paths::schema();
    let mig_dialect = mig_paths::dialect();
    let mig_snapshot = mig_paths::snapshot();
    let mig_pg_snapshot = mig_paths::postgres::snapshot();
    let mig_pg_entity = mig_paths::postgres::entity();
    let mig_pg_schema_entity = mig_paths::postgres::schema_entity();
    let mig_pg_table = mig_paths::postgres::table();
    let mig_pg_column = mig_paths::postgres::column();
    let mig_pg_identity = mig_paths::postgres::identity();
    let mig_pg_index = mig_paths::postgres::index();
    let mig_pg_primary_key = mig_paths::postgres::primary_key();
    let mig_pg_unique_constraint = mig_paths::postgres::unique_constraint();
    let mig_pg_enum = mig_paths::postgres::enum_type();
    let mig_pg_view = mig_paths::postgres::view();

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
        impl #sql_schema_impl for #struct_name {
            fn create_statements(&self) -> ::std::vec::Vec<::std::string::String> {
                #create_statements_impl
            }
        }

        // Implement tuple destructuring support
        impl ::std::convert::From<#struct_name> for (#(#all_field_types,)*) {
            fn from(schema: #struct_name) -> Self {
                (#(schema.#all_field_names,)*)
            }
        }

        // Implement migrations Schema trait for migration config
        impl #mig_schema for #struct_name {
            fn dialect(&self) -> #mig_dialect {
                #mig_dialect::PostgreSQL
            }

            fn to_snapshot(&self) -> #mig_snapshot {
                // Use type aliases to avoid name collisions with user types
                type MigSnapshot = #mig_pg_snapshot;
                type MigEntity = #mig_pg_entity;
                type MigSchema = #mig_pg_schema_entity;
                type MigTable = #mig_pg_table;
                type MigColumn = #mig_pg_column;
                type MigIdentity = #mig_pg_identity;
                type MigIndex = #mig_pg_index;
                type MigPrimaryKey = #mig_pg_primary_key;
                type MigUniqueConstraint = #mig_pg_unique_constraint;
                type MigEnum = #mig_pg_enum;
                type MigView = #mig_pg_view;

                let mut snapshot = MigSnapshot::new();

                // Add public schema entity
                snapshot.add_entity(MigEntity::Schema(MigSchema::new("public")));

                // Iterate through all schema fields and add DDL entities
                #(
                    match <#field_types_for_snapshot as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::TYPE {
                        #postgres_schema_type::Table(table_info) => {
                            // Add table entity
                            let table_name = #sql_table_info::name(table_info);
                            snapshot.add_entity(MigEntity::Table(MigTable::new("public", table_name)));

                            // Add column entities using PostgresTableInfo::postgres_columns
                            for col in #postgres_table_info::postgres_columns(table_info) {
                                // col is &dyn PostgresColumnInfo which extends SQLColumnInfo
                                // Use method syntax since trait object vtable includes supertrait methods
                                let mut column = MigColumn::new(
                                    "public",
                                    table_name,
                                    col.name(),
                                    col.postgres_type(),
                                );

                                if col.is_not_null() {
                                    column = column.not_null();
                                }

                                // Handle identity/serial columns
                                if col.is_generated_identity() || col.is_serial() || col.is_bigserial() {
                                    let seq_name = ::std::format!("{}_{}_seq", table_name, col.name());
                                    let identity = if col.is_generated_identity() {
                                        MigIdentity::always(seq_name).schema("public")
                                    } else {
                                        MigIdentity::by_default(seq_name).schema("public")
                                    };
                                    column = column.identity(identity);
                                }

                                snapshot.add_entity(MigEntity::Column(column));

                                // Add primary key entity if this is a primary key column
                                if col.is_primary_key() {
                                    snapshot.add_entity(MigEntity::PrimaryKey(MigPrimaryKey::from_strings(
                                        "public".to_string(),
                                        table_name.to_string(),
                                        ::std::format!("{}_pkey", table_name),
                                        ::std::vec![col.name().to_string()],
                                    )));
                                }

                                // Add unique constraint entity if this column is unique
                                if col.is_unique() {
                                    snapshot.add_entity(MigEntity::UniqueConstraint(MigUniqueConstraint::from_strings(
                                        "public".to_string(),
                                        table_name.to_string(),
                                        ::std::format!("{}_{}_key", table_name, col.name()),
                                        ::std::vec![col.name().to_string()],
                                    )));
                                }
                            }
                        }
                        #postgres_schema_type::Index(index_info) => {
                            // Add index entity
                            // Note: SQLIndexInfo doesn't expose column info directly
                            let table = #sql_index_info::table(index_info);
                            let mut index = MigIndex::new(
                                "public",
                                #sql_table_info::name(table),
                                #sql_index_info::name(index_info),
                                ::std::vec::Vec::new(), // Would need columns from index definition
                            );
                            if #sql_index_info::is_unique(index_info) {
                                index = index.unique();
                            }
                            snapshot.add_entity(MigEntity::Index(index));
                        }
                        #postgres_schema_type::Enum(enum_info) => {
                            // Add enum entity
                            snapshot.add_entity(MigEntity::Enum(MigEnum::from_strings(
                                "public".to_string(),
                                enum_info.name().to_string(),
                                enum_info.variants().iter().map(|v| v.to_string()).collect(),
                            )));
                        }
                        #postgres_schema_type::View(view_info) => {
                            let mut view = MigView::new(view_info.schema(), #sql_table_info::name(view_info));
                            let definition = view_info.definition_sql();
                            if !definition.is_empty() {
                                view.definition = ::std::option::Option::Some(definition);
                            }
                            view.materialized = view_info.is_materialized();
                            if view_info.is_existing() {
                                view.is_existing = true;
                            }
                            view.with_no_data = view_info.with_no_data();
                            view.using = view_info.using_clause().map(::std::borrow::Cow::Borrowed);
                            view.tablespace = view_info.tablespace().map(::std::borrow::Cow::Borrowed);
                            snapshot.add_entity(MigEntity::View(view));
                        }
                        #postgres_schema_type::Trigger => {
                            // Triggers not implemented yet
                        }
                    }
                )*

                #mig_snapshot::Postgres(snapshot)
            }
        }
    })
}

fn generate_create_statements_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_table_info = core_paths::sql_table_info();
    let sql_index_info = core_paths::sql_index_info();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    // Extract field names and types for easier iteration
    let field_names: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    quote! {
        let mut tables: ::std::vec::Vec<(&str, ::std::string::String, &dyn #sql_table_info)> = ::std::vec::Vec::new();
        let mut indexes: ::std::collections::HashMap<&str, ::std::vec::Vec<::std::string::String>> = ::std::collections::HashMap::new();
        let mut enums: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
        let mut views: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();

        // Collect all tables, indexes, and enums
        #(
            match <#field_types as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::TYPE {
                #postgres_schema_type::Table(table_info) => {
                    let table_name = #sql_table_info::name(table_info);
                    let table_sql = <_ as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::sql(&self.#field_names).sql();
                    tables.push((table_name, table_sql, table_info));
                }
                #postgres_schema_type::Index(index_info) => {
                    let index_sql = <_ as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::sql(&self.#field_names).sql();
                    let table_name = #sql_table_info::name(#sql_index_info::table(index_info));
                    indexes
                        .entry(table_name)
                        .or_insert_with(::std::vec::Vec::new)
                        .push(index_sql);
                }
                #postgres_schema_type::Enum(enum_info) => {
                    let enum_sql = enum_info.create_type_sql();
                    enums.push(enum_sql);
                }
                #postgres_schema_type::View(view_info) => {
                    if !view_info.is_existing() {
                        let view_sql = <_ as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::sql(&self.#field_names).sql();
                        views.push(view_sql);
                    }
                }
                #postgres_schema_type::Trigger => {
                    // Triggers not implemented yet
                }
            }
        )*

        // Sort tables by dependencies (topological sort) then by name for deterministic ordering
        tables.sort_by(|a, b| {
            use ::std::cmp::Ordering;

            // Check if a depends on b
            let a_depends_on_b = #sql_table_info::dependencies(a.2).iter().any(|dep| #sql_table_info::name(*dep) == b.0);
            // Check if b depends on a
            let b_depends_on_a = #sql_table_info::dependencies(b.2).iter().any(|dep| #sql_table_info::name(*dep) == a.0);

            match (a_depends_on_b, b_depends_on_a) {
                (true, false) => Ordering::Greater,  // a comes after b
                (false, true) => Ordering::Less,     // a comes before b
                _ => a.0.cmp(b.0),                   // Same dependency level, sort by name
            }
        });

        // Build final SQL statements: enums first, then tables in dependency order, then their indexes
        let mut sql_statements = ::std::vec::Vec::<::std::string::String>::new();

        // Add all enums first (they must be created before tables that use them)
        sql_statements.extend(enums);

        // Add tables and their indexes
        for (table_name, table_sql, _) in tables {
            sql_statements.push(table_sql);

            // Add indexes for this table
            if let ::std::option::Option::Some(table_indexes) = indexes.get(table_name) {
                for index_sql in table_indexes {
                    sql_statements.push(index_sql.clone());
                }
            }
        }

        // Add views last (they depend on tables)
        sql_statements.extend(views);

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
