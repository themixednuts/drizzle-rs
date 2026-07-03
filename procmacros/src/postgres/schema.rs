use crate::paths::{core as core_paths, migrations as mig_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the `PostgresSchema` derive implementation
pub fn generate_postgres_schema_derive_impl(input: &DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_schema_impl = core_paths::sql_schema_impl();
    let validate_schema_item_foreign_keys = core_paths::validate_schema_item_foreign_keys();
    let sql_table_info = core_paths::sql_table_info();
    let sql_index_info = core_paths::sql_index_info();
    let sql_policy_info = core_paths::sql_policy_info();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    // Extract fields from the struct
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named_fields) => &named_fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[derive(PostgresSchema)] requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(PostgresSchema)] can only be applied to structs",
            ));
        }
    };

    // Collect all fields (we'll determine table vs index vs enum at runtime)
    let all_fields: Vec<_> = fields
        .iter()
        .map(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| (ident, &field.ty))
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        field,
                        "#[derive(PostgresSchema)] fields must have names",
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;

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
    let mig_pg_generated = quote! { drizzle::migrations::postgres::Generated };
    let mig_pg_generated_type = quote! { drizzle::migrations::postgres::GeneratedType };
    let mig_pg_sequence = mig_paths::postgres::sequence();
    let mig_pg_index = mig_paths::postgres::index();
    let mig_pg_index_column = mig_paths::postgres::index_column();
    let mig_pg_primary_key = mig_paths::postgres::primary_key();
    let mig_pg_foreign_key = mig_paths::postgres::foreign_key();
    let mig_pg_unique_constraint = mig_paths::postgres::unique_constraint();
    let mig_pg_check_constraint = mig_paths::postgres::check_constraint();
    let mig_pg_policy = mig_paths::postgres::policy();
    let mig_pg_enum = mig_paths::postgres::enum_type();
    let mig_pg_view = mig_paths::postgres::view();

    let schema_table_refs_method = generate_schema_table_refs_method(&all_fields);
    let schema_has_table_impls = generate_schema_has_table_impls(struct_name, &all_fields);
    let schema_fk_validation_asserts = generate_schema_fk_validation_asserts(
        &all_fields,
        struct_name,
        &validate_schema_item_foreign_keys,
    );

    Ok(quote! {
        impl ::core::marker::Copy for #struct_name {}
        impl ::core::clone::Clone for #struct_name {
            fn clone(&self) -> Self { *self }
        }
        impl ::core::fmt::Debug for #struct_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#struct_name))
                    #(.field(stringify!(#all_field_names), &self.#all_field_names))*
                    .finish()
            }
        }

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
            fn table_refs(&self) -> &'static [&'static drizzle::core::TableRef] {
                #schema_table_refs_method
            }

            fn create_statements(&self) -> ::std::result::Result<impl ::std::iter::Iterator<Item = ::std::string::String>, drizzle::error::DrizzleError> {
                let statements: ::std::vec::Vec<::std::string::String> = { #create_statements_impl };
                ::std::result::Result::Ok(statements.into_iter())
            }
        }

        // Implement tuple destructuring support
        impl ::std::convert::From<#struct_name> for (#(#all_field_types,)*) {
            fn from(schema: #struct_name) -> Self {
                (#(schema.#all_field_names,)*)
            }
        }

        #schema_has_table_impls

        #schema_fk_validation_asserts

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
                type MigIndexColumn = #mig_pg_index_column;
                type MigPrimaryKey = #mig_pg_primary_key;
                type MigForeignKey = #mig_pg_foreign_key;
                type MigUniqueConstraint = #mig_pg_unique_constraint;
                type MigCheckConstraint = #mig_pg_check_constraint;
                type MigPolicy = #mig_pg_policy;
                type MigEnum = #mig_pg_enum;
                type MigView = #mig_pg_view;
                type MigSequence = #mig_pg_sequence;

                let mut snapshot = MigSnapshot::new();
                let mut seen_schemas = ::std::collections::HashSet::new();

                // Iterate through all schema fields and add DDL entities
                #(
                    match <#field_types_for_snapshot as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::TYPE {
                        #postgres_schema_type::Table(_table_info) => {
                            // Use const TABLE_REF for column metadata instead of dyn traits
                            let table_ref = <#field_types_for_snapshot as drizzle::core::SchemaItemTables>::TABLE_REF_CONST
                                .expect("table must have TABLE_REF_CONST");
                            let table_name = table_ref.name;
                            let table_schema = table_ref.schema.unwrap_or("public");
                            // Add schema entity if not already added
                            if seen_schemas.insert(table_schema) {
                                snapshot.add_entity(MigEntity::Schema(MigSchema::new(table_schema)));
                            }
                            let mut table = MigTable::new(table_schema, table_name);
                            if let drizzle::core::TableDialect::PostgreSQL {
                                is_unlogged,
                                is_temporary,
                                inherits,
                                tablespace,
                                is_rls_enabled,
                                comment,
                            } = table_ref.dialect {
                                if is_unlogged {
                                    table = table.unlogged();
                                }
                                if is_temporary {
                                    table = table.temporary();
                                }
                                if let ::core::option::Option::Some(inherits) = inherits {
                                    table = table.inherits(inherits);
                                }
                                if let ::core::option::Option::Some(tablespace) = tablespace {
                                    table = table.tablespace(tablespace);
                                }
                                if is_rls_enabled {
                                    table = table.rls_enabled();
                                }
                                if let ::core::option::Option::Some(comment) = comment {
                                    table = table.comment(comment);
                                }
                            }
                            snapshot.add_entity(MigEntity::Table(table));

                            // Add column entities from TABLE_REF
                            for col in table_ref.columns {
                                let (
                                    pg_type,
                                    is_generated_identity,
                                    is_identity_always,
                                    default,
                                    generated_expression,
                                    generated_stored,
                                    collate,
                                    dimensions,
                                    comment,
                                ) = match col.dialect {
                                    drizzle::core::ColumnDialect::PostgreSQL {
                                        postgres_type,
                                        is_generated_identity,
                                        is_identity_always,
                                        default,
                                        generated_expression,
                                        generated_stored,
                                        collate,
                                        dimensions,
                                        comment,
                                        ..
                                    } => (
                                        postgres_type,
                                        is_generated_identity,
                                        is_identity_always,
                                        default,
                                        generated_expression,
                                        generated_stored,
                                        collate,
                                        dimensions,
                                        comment,
                                    ),
                                    _ => (col.sql_type, false, false, ::core::option::Option::None, ::core::option::Option::None, false, ::core::option::Option::None, ::core::option::Option::None, ::core::option::Option::None),
                                };

                                let mut column = MigColumn::new(
                                    table_schema,
                                    table_name,
                                    col.name,
                                    pg_type,
                                );

                                if matches!(
                                    drizzle::migrations::postgres::PgTypeCategory::from_sql_type(pg_type),
                                    drizzle::migrations::postgres::PgTypeCategory::Custom
                                ) {
                                    column.type_schema = ::core::option::Option::Some(::std::borrow::Cow::Borrowed(table_schema));
                                }

                                if col.not_null() {
                                    column = column.not_null();
                                }
                                if let ::core::option::Option::Some(default) = default {
                                    column = column.default_value(default);
                                }

                                // Handle identity columns (NOT serial — serial uses
                                // the SERIAL pseudo-type which implies its own sequence
                                // via DEFAULT nextval(...); combining it with GENERATED
                                // AS IDENTITY is invalid in PostgreSQL).
                                if is_generated_identity {
                                    let seq_name = ::std::format!("{}_{}_seq", table_name, col.name);
                                    let identity = if is_identity_always {
                                        MigIdentity::always(seq_name)
                                    } else {
                                        MigIdentity::by_default(seq_name)
                                    }.schema(table_schema);
                                    column = column.identity(identity);
                                }

                                if let ::core::option::Option::Some(expression) = generated_expression {
                                    column.generated = ::core::option::Option::Some(#mig_pg_generated {
                                        expression: ::std::borrow::Cow::Borrowed(expression),
                                        gen_type: if generated_stored {
                                            #mig_pg_generated_type::Stored
                                        } else {
                                            #mig_pg_generated_type::Virtual
                                        },
                                    });
                                }

                                if let ::core::option::Option::Some(collate) = collate {
                                    column.collate = ::core::option::Option::Some(::std::borrow::Cow::Borrowed(collate));
                                }
                                column.dimensions = dimensions;
                                if let ::core::option::Option::Some(comment) = comment {
                                    column.comment = ::core::option::Option::Some(::std::borrow::Cow::Borrowed(comment));
                                }

                                snapshot.add_entity(MigEntity::Column(column));

                                // Add primary key entity if this is a primary key column
                                if col.primary_key() {
                                    snapshot.add_entity(MigEntity::PrimaryKey(MigPrimaryKey::from_strings(
                                        table_schema.to_string(),
                                        table_name.to_string(),
                                        ::std::format!("{}_pkey", table_name),
                                        ::std::vec![col.name.to_string()],
                                    )));
                                }

                                // Add unique constraint entity if this column is unique
                                if col.unique() {
                                    snapshot.add_entity(MigEntity::UniqueConstraint(MigUniqueConstraint::from_strings(
                                        table_schema.to_string(),
                                        table_name.to_string(),
                                        ::std::format!("{}_{}_key", table_name, col.name),
                                        ::std::vec![col.name.to_string()],
                                    )));
                                }
                            }

                            for fk in table_ref.foreign_keys {
                                let mut foreign_key = MigForeignKey::from_strings(
                                    table_schema.to_string(),
                                    table_name.to_string(),
                                    fk.name.to_string(),
                                    fk.source_columns.iter().map(|col| col.to_string()).collect(),
                                    fk.target_schema.to_string(),
                                    fk.target_table.to_string(),
                                    fk.target_columns.iter().map(|col| col.to_string()).collect(),
                                );
                                foreign_key.name_explicit = fk.name_explicit;
                                if let ::core::option::Option::Some(on_delete) = fk.on_delete {
                                    foreign_key = foreign_key.on_delete(on_delete);
                                }
                                if let ::core::option::Option::Some(on_update) = fk.on_update {
                                    foreign_key = foreign_key.on_update(on_update);
                                }
                                if fk.deferrable {
                                    foreign_key = foreign_key.deferrable();
                                }
                                if fk.initially_deferred {
                                    foreign_key = foreign_key.initially_deferred();
                                }
                                snapshot.add_entity(MigEntity::ForeignKey(foreign_key));
                            }

                            for constraint in table_ref.constraints {
                                match constraint.kind {
                                    drizzle::core::SQLConstraintKind::Unique => {
                                        let unique_name = constraint.name.unwrap_or("unique");
                                        let mut unique = MigUniqueConstraint::from_strings(
                                            table_schema.to_string(),
                                            table_name.to_string(),
                                            unique_name.to_string(),
                                            constraint.columns.iter().map(|col| col.to_string()).collect(),
                                        );
                                        unique.name_explicit = constraint.name_explicit;
                                        if constraint.deferrable {
                                            unique = unique.deferrable();
                                        }
                                        if constraint.initially_deferred {
                                            unique = unique.initially_deferred();
                                        }
                                        snapshot.add_entity(MigEntity::UniqueConstraint(unique));
                                    }
                                    drizzle::core::SQLConstraintKind::Check => {
                                        if let ::core::option::Option::Some(check_expression) = constraint.check_expression {
                                            let check_name = constraint.name.unwrap_or("check");
                                            snapshot.add_entity(MigEntity::CheckConstraint(MigCheckConstraint::new(
                                                table_schema,
                                                table_name,
                                                check_name,
                                                check_expression,
                                            )));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        #postgres_schema_type::Index(index_info) => {
                            // Add index entity
                            let table_ref = #sql_index_info::table(index_info);
                            let table_schema = table_ref.schema.unwrap_or("public");
                            let mut index = MigIndex::new(
                                table_schema,
                                table_ref.name,
                                #sql_index_info::name(index_info),
                                #sql_index_info::columns(index_info)
                                    .iter()
                                    .map(|c| MigIndexColumn::new(*c))
                                    .collect::<::std::vec::Vec<_>>(),
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
                            let view_schema = #sql_table_info::schema(view_info).unwrap_or("public");
                            let mut view = MigView::new(view_schema, #sql_table_info::name(view_info));
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
                        #postgres_schema_type::Policy(policy_info) => {
                            let table_ref = #sql_policy_info::table(policy_info);
                            let table_schema = table_ref.schema.unwrap_or("public");
                            if seen_schemas.insert(table_schema) {
                                snapshot.add_entity(MigEntity::Schema(MigSchema::new(table_schema)));
                            }

                            let mut policy = MigPolicy::new(
                                table_schema,
                                table_ref.name,
                                #sql_policy_info::name(policy_info),
                            );
                            policy.as_clause = #sql_policy_info::as_clause(policy_info)
                                .map(::std::borrow::Cow::Borrowed);
                            policy.for_clause = #sql_policy_info::for_clause(policy_info)
                                .map(::std::borrow::Cow::Borrowed);
                            let roles = #sql_policy_info::to(policy_info);
                            if !roles.is_empty() {
                                policy.to = ::core::option::Option::Some(
                                    roles.iter().copied().map(::std::borrow::Cow::Borrowed).collect(),
                                );
                            }
                            policy.using = #sql_policy_info::using(policy_info)
                                .map(::std::borrow::Cow::Borrowed);
                            policy.with_check = #sql_policy_info::with_check(policy_info)
                                .map(::std::borrow::Cow::Borrowed);
                            snapshot.add_entity(MigEntity::Policy(policy));
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

fn generate_schema_fk_validation_asserts(
    fields: &[(&syn::Ident, &syn::Type)],
    struct_name: &syn::Ident,
    validate_schema_item_foreign_keys: &TokenStream,
) -> TokenStream {
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    quote! {
        const _: () = {
            const fn __assert_schema_item<Item>()
            where
                Item: #validate_schema_item_foreign_keys<#struct_name>,
            {
            }

            #(
                __assert_schema_item::<#field_types>();
            )*
        };
    }
}

fn generate_create_statements_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_index_info = core_paths::sql_index_info();
    let sql_policy_info = core_paths::sql_policy_info();
    let sql_table_info = core_paths::sql_table_info();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let schema_item_tables = core_paths::schema_item_tables();
    let policy_ddl = mig_paths::postgres::policy();

    // Extract field names and types for easier iteration
    #[allow(unused_variables)]
    let field_names: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    quote! {
        let mut tables: ::std::vec::Vec<(
            ::std::string::String,
            ::std::string::String,
            &'static drizzle::core::TableRef,
            ::std::vec::Vec<::std::string::String>,
            ::core::option::Option<::std::string::String>,
        )> = ::std::vec::Vec::new();
        let mut indexes: ::std::collections::HashMap<::std::string::String, ::std::vec::Vec<::std::string::String>> = ::std::collections::HashMap::new();
        let mut index_keys: ::std::collections::HashSet<::std::string::String> = ::std::collections::HashSet::new();
        let mut policies: ::std::collections::HashMap<::std::string::String, ::std::vec::Vec<::std::string::String>> = ::std::collections::HashMap::new();
        let mut policy_keys: ::std::collections::HashSet<::std::string::String> = ::std::collections::HashSet::new();
        let mut enums: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
        let mut views: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();

        // Collect all tables, indexes, and enums
        #(
            match <#field_types as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::TYPE {
                #postgres_schema_type::Table(_table_info) => {
                    let table_ref = <#field_types as #schema_item_tables>::TABLE_REF_CONST
                        .expect("table must have TABLE_REF_CONST");
                    let table_name = table_ref.qualified_name.to_string();
                    let table_sql = <#field_types as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL.to_string();
                    let schema = table_ref.schema.unwrap_or("public");
                    let quote_ident = |ident: &str| -> ::std::string::String {
                        ::std::format!("\"{}\"", ident.replace('"', "\"\""))
                    };
                    let quote_literal = |value: &str| -> ::std::string::String {
                        ::std::format!("'{}'", value.replace('\'', "''"))
                    };
                    let qualified = if schema == "public" {
                        quote_ident(table_ref.name)
                    } else {
                        ::std::format!("{}.{}", quote_ident(schema), quote_ident(table_ref.name))
                    };
                    let mut comment_sqls = ::std::vec::Vec::new();
                    if let drizzle::core::TableDialect::PostgreSQL { comment: ::core::option::Option::Some(comment), .. } = table_ref.dialect {
                        comment_sqls.push(::std::format!(
                            "COMMENT ON TABLE {} IS {};",
                            qualified,
                            quote_literal(comment)
                        ));
                    }
                    for column in table_ref.columns {
                        if let drizzle::core::ColumnDialect::PostgreSQL { comment: ::core::option::Option::Some(comment), .. } = column.dialect {
                            comment_sqls.push(::std::format!(
                                "COMMENT ON COLUMN {}.{} IS {};",
                                qualified,
                                quote_ident(column.name),
                                quote_literal(comment)
                            ));
                        }
                    }
                    let rls_sql = match table_ref.dialect {
                        drizzle::core::TableDialect::PostgreSQL { is_rls_enabled: true, .. } => {
                            ::core::option::Option::Some(
                                ::std::format!("ALTER TABLE {} ENABLE ROW LEVEL SECURITY;", qualified)
                            )
                        }
                        _ => ::core::option::Option::None,
                    };
                    tables.push((table_name, table_sql, table_ref, comment_sqls, rls_sql));
                }
                #postgres_schema_type::Index(index_info) => {
                    let index_sql = <#field_types as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL.to_string();
                    let idx_table_ref = #sql_index_info::table(index_info);
                    let table_name = idx_table_ref.qualified_name.to_string();
                    let index_name = #sql_index_info::name(index_info);
                    let index_key = ::std::format!("{}::{}", table_name, index_name);
                    if !index_keys.insert(index_key) {
                        return ::std::result::Result::Err(drizzle::error::DrizzleError::Statement(
                            ::std::format!("Duplicate index '{}' on table '{}' in PostgresSchema", index_name, table_name).into(),
                        ));
                    }
                    indexes
                        .entry(table_name)
                        .or_insert_with(::std::vec::Vec::new)
                        .push(index_sql);
                }
                #postgres_schema_type::Enum(_enum_info) => {
                    let enum_sql = <#field_types as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL.to_string();
                    enums.push(enum_sql);
                }
                #postgres_schema_type::View(view_info) => {
                    if !view_info.is_existing() {
                        let sql = <#field_types as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL;
                        let view_sql = if sql.is_empty() {
                            // Expression-based views have empty const SQL; reconstruct from view_info
                            let view_schema = #sql_table_info::schema(view_info).unwrap_or("public");
                            let view_name = #sql_table_info::name(view_info);
                            let definition = view_info.definition_sql();
                            let materialized_kw = if view_info.is_materialized() { "MATERIALIZED " } else { "" };
                            let schema_prefix = if view_schema != "public" {
                                ::std::format!("\"{}\".", view_schema)
                            } else {
                                ::std::string::String::new()
                            };
                            let mut view_sql = ::std::format!(
                                "CREATE {}VIEW {}\"{}\"",
                                materialized_kw, schema_prefix, view_name
                            );
                            if let ::std::option::Option::Some(using) = view_info.using_clause() {
                                view_sql.push_str(&::std::format!(" USING {}", using));
                            }
                            if let ::std::option::Option::Some(tablespace) = view_info.tablespace() {
                                view_sql.push_str(&::std::format!(" TABLESPACE \"{}\"", tablespace));
                            }
                            view_sql.push_str(" AS ");
                            view_sql.push_str(&definition);
                            if view_info.is_materialized() {
                                if let ::std::option::Option::Some(true) = view_info.with_no_data() {
                                    view_sql.push_str(" WITH NO DATA");
                                }
                            }
                            view_sql
                        } else {
                            sql.to_string()
                        };
                        views.push(view_sql);
                    }
                }
                #postgres_schema_type::Policy(policy_info) => {
                    let table_ref = #sql_policy_info::table(policy_info);
                    let table_name = table_ref.qualified_name.to_string();
                    let policy_name = #sql_policy_info::name(policy_info);
                    let policy_key = ::std::format!("{}::{}", table_name, policy_name);
                    if !policy_keys.insert(policy_key) {
                        return ::std::result::Result::Err(drizzle::error::DrizzleError::Statement(
                            ::std::format!("Duplicate policy '{}' on table '{}' in PostgresSchema", policy_name, table_name).into(),
                        ));
                    }
                    let mut policy = #policy_ddl::new(
                        table_ref.schema.unwrap_or("public"),
                        table_ref.name,
                        policy_name,
                    );
                    policy.as_clause = #sql_policy_info::as_clause(policy_info)
                        .map(::std::borrow::Cow::Borrowed);
                    policy.for_clause = #sql_policy_info::for_clause(policy_info)
                        .map(::std::borrow::Cow::Borrowed);
                    let roles = #sql_policy_info::to(policy_info);
                    if !roles.is_empty() {
                        policy.to = ::std::option::Option::Some(
                            roles.iter().copied().map(::std::borrow::Cow::Borrowed).collect()
                        );
                    }
                    policy.using = #sql_policy_info::using(policy_info)
                        .map(::std::borrow::Cow::Borrowed);
                    policy.with_check = #sql_policy_info::with_check(policy_info)
                        .map(::std::borrow::Cow::Borrowed);
                    let policy_sql = policy.create_policy_sql();
                    policies
                        .entry(table_name)
                        .or_insert_with(::std::vec::Vec::new)
                        .push(policy_sql);
                }
                #postgres_schema_type::Trigger => {
                    // Triggers not implemented yet
                }
            }
        )*

        // Deterministic topological ordering via Kahn's algorithm.
        // Guarantees dependency-safe order for DAGs in O(V + E), with
        // lexical tie-breaking for stable output.
        tables.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        let table_names: ::std::collections::HashSet<::std::string::String> =
            tables.iter().map(|(name, _, _, _, _)| name.clone()).collect();

        if table_names.len() != tables.len() {
            return ::std::result::Result::Err(drizzle::error::DrizzleError::Statement(
                "Duplicate table names detected in PostgresSchema".into(),
            ));
        }

        let mut indegree: ::std::collections::HashMap<::std::string::String, usize> =
            ::std::collections::HashMap::with_capacity(tables.len());
        let mut reverse_edges: ::std::collections::HashMap<::std::string::String, ::std::vec::Vec<::std::string::String>> =
            ::std::collections::HashMap::new();

        // Map unqualified name → qualified name for dependency resolution
        let name_to_qualified: ::std::collections::HashMap<::std::string::String, ::std::string::String> =
            tables.iter().map(|(qname, _, tref, _, _)| (tref.name.to_string(), qname.clone())).collect();

        for (table_name, _, table_ref, _, _) in &tables {
            indegree.entry(table_name.clone()).or_insert(0);

            for dep_name in table_ref.dependency_names
                .iter()
                .filter_map(|dep| name_to_qualified.get(*dep))
                .filter(|dep_name| *dep_name != table_name)
                .filter(|dep_name| table_names.contains(*dep_name))
                .cloned()
            {
                *indegree
                    .get_mut(table_name)
                    .expect("indegree is initialized for each table") += 1;
                reverse_edges
                    .entry(dep_name)
                    .or_insert_with(::std::vec::Vec::new)
                    .push(table_name.clone());
            }
        }

        let mut ready: ::std::collections::BTreeSet<::std::string::String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(name, _)| name.clone())
            .collect();
        let mut ordered_names: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::with_capacity(tables.len());

        while let ::std::option::Option::Some(next) = ready.pop_first() {
            ordered_names.push(next.clone());

            if let ::std::option::Option::Some(children) = reverse_edges.get(&next) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("child table must exist in indegree map");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }

        if ordered_names.len() != tables.len() {
            let mut remaining: ::std::vec::Vec<::std::string::String> = indegree
                .iter()
                .filter(|(_, degree)| **degree > 0)
                .map(|(name, _)| name.clone())
                .collect();
            remaining.sort_unstable();
            return ::std::result::Result::Err(drizzle::error::DrizzleError::Statement(
                ::std::format!(
                    "Cyclic table dependency detected in PostgresSchema: {}",
                    remaining.join(", ")
                )
                .into(),
            ));
        }

        let mut table_by_name: ::std::collections::HashMap<
            ::std::string::String,
            (::std::string::String, ::std::vec::Vec<::std::string::String>, ::core::option::Option<::std::string::String>),
        > = ::std::collections::HashMap::with_capacity(tables.len());
        for (table_name, table_sql, _table_ref, comment_sqls, rls_sql) in tables {
            table_by_name.insert(table_name, (table_sql, comment_sqls, rls_sql));
        }

        // Build final SQL statements: enums first, then tables in dependency order, then their indexes
        let mut sql_statements = ::std::vec::Vec::<::std::string::String>::new();

        // Add all enums first (they must be created before tables that use them)
        sql_statements.extend(enums);

        // Add tables and their indexes
        for table_name in ordered_names {
            let (table_sql, comment_sqls, rls_sql) = table_by_name
                .remove(&table_name)
                .expect("table exists after topological ordering");
            sql_statements.push(table_sql);
            sql_statements.extend(comment_sqls);

            // Add indexes for this table
            if let ::std::option::Option::Some(table_indexes) = indexes.get(&table_name) {
                for index_sql in table_indexes {
                    sql_statements.push(index_sql.clone());
                }
            }

            if let ::core::option::Option::Some(rls_sql) = rls_sql {
                sql_statements.push(rls_sql);
            }

            if let ::std::option::Option::Some(table_policies) = policies.get(&table_name) {
                for policy_sql in table_policies {
                    sql_statements.push(policy_sql.clone());
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

fn generate_schema_table_refs_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let table_ref = core_paths::table_ref();
    let schema_item_tables = core_paths::schema_item_tables();

    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();
    let n_total = fields.len();

    quote! {
        static TABLE_REF_OPTIONS: [::core::option::Option<&'static #table_ref>; #n_total] = [
            #(
                <#field_types as #schema_item_tables>::TABLE_REF_CONST,
            )*
        ];

        const TABLE_REF_COUNT: usize = {
            let mut count = 0usize;
            let mut i = 0usize;
            while i < #n_total {
                if TABLE_REF_OPTIONS[i].is_some() {
                    count += 1;
                }
                i += 1;
            }
            count
        };

        static TABLE_REFS: [&'static #table_ref; TABLE_REF_COUNT] = {
            let mut result: [::core::mem::MaybeUninit<&'static #table_ref>; TABLE_REF_COUNT] =
                [::core::mem::MaybeUninit::uninit(); TABLE_REF_COUNT];
            let mut out = 0usize;
            let mut i = 0usize;
            while i < #n_total {
                if let ::core::option::Option::Some(t) = TABLE_REF_OPTIONS[i] {
                    result[out] = ::core::mem::MaybeUninit::new(t);
                    out += 1;
                }
                i += 1;
            }
            // SAFETY: exactly TABLE_REF_COUNT elements are initialized
            unsafe { ::core::mem::transmute(result) }
        };

        &TABLE_REFS
    }
}

fn generate_schema_has_table_impls(
    struct_name: &syn::Ident,
    fields: &[(&syn::Ident, &syn::Type)],
) -> TokenStream {
    let schema_has_table = core_paths::schema_has_table();
    let mut unique_types = Vec::new();
    let mut seen = HashSet::new();
    for (_, ty) in fields {
        let key = quote!(#ty).to_string();
        if seen.insert(key) {
            unique_types.push(*ty);
        }
    }

    quote! {
        #(
            impl #schema_has_table<#unique_types> for #struct_name {}
        )*
    }
}
