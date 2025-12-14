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
    let sql_column_info = core_paths::sql_column_info();
    let sql_index_info = core_paths::sql_index_info();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let postgres_table_info = postgres_paths::postgres_table_info();
    let postgres_column_info = postgres_paths::postgres_column_info();

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

                let mut snapshot = MigSnapshot::new();

                // Add public schema entity
                snapshot.add_entity(MigEntity::Schema(MigSchema {
                    name: "public".to_string(),
                }));

                // Iterate through all schema fields and add DDL entities
                #(
                    match <#field_types_for_snapshot as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::TYPE {
                        #postgres_schema_type::Table(table_info) => {
                            // Add table entity
                            let table_name = #sql_table_info::name(table_info);
                            snapshot.add_entity(MigEntity::Table(MigTable {
                                schema: "public".to_string(),
                                name: table_name.to_string(),
                                is_rls_enabled: ::std::option::Option::None,
                            }));

                            // Add column entities using PostgresTableInfo::postgres_columns
                            for col in #postgres_table_info::postgres_columns(table_info) {
                                // col is &dyn PostgresColumnInfo which extends SQLColumnInfo
                                // Use method syntax since trait object vtable includes supertrait methods
                                snapshot.add_entity(MigEntity::Column(MigColumn {
                                    schema: "public".to_string(),
                                    table: table_name.to_string(),
                                    name: col.name().to_string(),
                                    sql_type: col.postgres_type().to_string(),
                                    type_schema: ::std::option::Option::None,
                                    not_null: col.is_not_null(),
                                    default: ::std::option::Option::None, // Default value access would need additional trait method
                                    generated: ::std::option::Option::None,
                                    identity: if col.is_generated_identity() || col.is_serial() || col.is_bigserial() {
                                        ::std::option::Option::Some(MigIdentity {
                                            name: ::std::format!("{}_{}_seq", table_name, col.name()),
                                            schema: ::std::option::Option::Some("public".to_string()),
                                            type_: if col.is_generated_identity() { "always".to_string() } else { "byDefault".to_string() },
                                            increment: ::std::option::Option::None,
                                            min_value: ::std::option::Option::None,
                                            max_value: ::std::option::Option::None,
                                            start_with: ::std::option::Option::None,
                                            cache: ::std::option::Option::None,
                                            cycle: ::std::option::Option::None,
                                        })
                                    } else {
                                        ::std::option::Option::None
                                    },
                                    dimensions: ::std::option::Option::None,
                                }));

                                // Add primary key entity if this is a primary key column
                                if col.is_primary_key() {
                                    snapshot.add_entity(MigEntity::PrimaryKey(MigPrimaryKey {
                                        schema: "public".to_string(),
                                        table: table_name.to_string(),
                                        name: ::std::format!("{}_pkey", table_name),
                                        name_explicit: false,
                                        columns: ::std::vec![col.name().to_string()],
                                    }));
                                }

                                // Add unique constraint entity if this column is unique
                                if col.is_unique() {
                                    snapshot.add_entity(MigEntity::UniqueConstraint(MigUniqueConstraint {
                                        schema: "public".to_string(),
                                        table: table_name.to_string(),
                                        name: ::std::format!("{}_{}_key", table_name, col.name()),
                                        name_explicit: false,
                                        columns: ::std::vec![col.name().to_string()],
                                        nulls_not_distinct: false,
                                    }));
                                }
                            }
                        }
                        #postgres_schema_type::Index(index_info) => {
                            // Add index entity
                            // Note: SQLIndexInfo doesn't expose column info directly
                            let table = #sql_index_info::table(index_info);
                            snapshot.add_entity(MigEntity::Index(MigIndex {
                                schema: "public".to_string(),
                                table: #sql_table_info::name(table).to_string(),
                                name: #sql_index_info::name(index_info).to_string(),
                                columns: ::std::vec::Vec::new(), // Would need columns from index definition
                                is_unique: #sql_index_info::is_unique(index_info),
                                r#where: ::std::option::Option::None,
                                method: ::std::option::Option::None,
                                concurrently: false,
                                r#with: ::std::option::Option::None,
                            }));
                        }
                        #postgres_schema_type::Enum(enum_info) => {
                            // Add enum entity
                            snapshot.add_entity(MigEntity::Enum(MigEnum {
                                schema: "public".to_string(),
                                name: enum_info.name().to_string(),
                                values: enum_info.variants().iter().map(|v| v.to_string()).collect(),
                            }));
                        }
                        #postgres_schema_type::View => {
                            // Views not implemented yet
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
                #postgres_schema_type::View => {
                    // Views not implemented yet
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
