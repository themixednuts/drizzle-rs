use crate::paths::{core as core_paths, migrations as mig_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the `PostgresSchema` derive implementation
pub fn generate_postgres_schema_derive_impl(input: &DeriveInput) -> Result<TokenStream> {
    crate::common::reject_schema_trait_derives(input, "PostgresSchema")?;
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
    let mig_pg_index = mig_paths::postgres::index();
    let mig_pg_index_column = mig_paths::postgres::index_column();
    let mig_pg_foreign_key = mig_paths::postgres::foreign_key();
    let mig_pg_check_constraint = mig_paths::postgres::check_constraint();
    let mig_pg_policy = mig_paths::postgres::policy();
    let mig_pg_enum = mig_paths::postgres::enum_type();
    let mig_pg_view = mig_paths::postgres::view();
    let postgres_item_ddl = crate::paths::ddl::postgres::postgres_item_ddl();

    let schema_table_refs_method = generate_schema_table_refs_method(&all_fields);
    let schema_has_table_impls = generate_schema_has_table_impls(struct_name, &all_fields);
    let schema_name_check = crate::common::schema_name_check(
        struct_name,
        &all_fields,
        &crate::common::constraints::DialectTypes::postgres(),
    );
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
                let statements: ::std::vec::Vec<::std::string::String> = {
                    // The same differ as `drizzle generate`, from an empty database:
                    // one source for DDL, and tables that reference each other get
                    // their foreign keys after both exist.
                    let empty = drizzle::migrations::Snapshot::empty(drizzle::Dialect::PostgreSQL);
                    let current = <Self as drizzle::migrations::Schema>::to_snapshot(self);
                    drizzle::migrations::diff(&empty, &current)
                        .map_err(|error| drizzle::error::DrizzleError::Statement(error.to_string().into()))?
                        .statements
                };
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
        #schema_name_check

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
                type MigIndex = #mig_pg_index;
                type MigIndexColumn = #mig_pg_index_column;
                type MigForeignKey = #mig_pg_foreign_key;
                type MigCheckConstraint = #mig_pg_check_constraint;
                type MigPolicy = #mig_pg_policy;
                type MigEnum = #mig_pg_enum;
                type MigView = #mig_pg_view;

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
                            if table_schema != "public" && seen_schemas.insert(table_schema) {
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

                            // Column entities come from the compile-time DDL
                            // consts (the same source `create_table_sql()`
                            // renders from), so identity sequence options and
                            // custom-type schemas survive into the snapshot.
                            for column_def in <#field_types_for_snapshot as #postgres_item_ddl>::SNAPSHOT_COLUMNS {
                                snapshot.add_entity(MigEntity::Column(column_def.into_column()));
                            }

                            // ONE PrimaryKey entity per table, covering all PK
                            // columns in declaration order (single-column and
                            // composite alike) — matches the compile-time
                            // `DDL_PRIMARY_KEY` shape.
                            if let ::core::option::Option::Some(pk) =
                                <#field_types_for_snapshot as #postgres_item_ddl>::SNAPSHOT_PRIMARY_KEY
                            {
                                snapshot.add_entity(MigEntity::PrimaryKey(pk.into_primary_key()));
                            }

                            // Unique constraints (column-level and table-level)
                            // from the DDL consts, preserving NULLS NOT
                            // DISTINCT / DEFERRABLE and explicit names.
                            for unique_def in <#field_types_for_snapshot as #postgres_item_ddl>::SNAPSHOT_UNIQUE_CONSTRAINTS {
                                snapshot.add_entity(MigEntity::UniqueConstraint(unique_def.into_unique_constraint()));
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
                                    // Unique constraints are emitted from the
                                    // DDL consts above (which carry NULLS NOT
                                    // DISTINCT; ConstraintRef does not).
                                    drizzle::core::SQLConstraintKind::Unique => {}
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
                            // Prefer the compile-time DDL definition, which
                            // carries method / where / concurrently. The
                            // SQLIndexInfo-based path stays as a fallback for
                            // index items without const DDL.
                            if let ::core::option::Option::Some(index_def) =
                                <#field_types_for_snapshot as #postgres_item_ddl>::SNAPSHOT_INDEX
                            {
                                snapshot.add_entity(MigEntity::Index(index_def.into_index()));
                            } else {
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
                                if let ::core::option::Option::Some(where_clause) =
                                    #sql_index_info::where_clause(index_info)
                                {
                                    index.where_clause = ::core::option::Option::Some(
                                        ::std::borrow::Cow::Borrowed(where_clause),
                                    );
                                }
                                snapshot.add_entity(MigEntity::Index(index));
                            }
                        }
                        #postgres_schema_type::Enum(enum_info) => {
                            // Add enum entity; the schema comes from the
                            // derive's `#[postgres_enum(schema = "...")]`
                            // (default `public`). Register the schema itself
                            // too — an enum may be the schema's only occupant
                            // and CREATE TYPE needs CREATE SCHEMA first.
                            let enum_schema = <#field_types_for_snapshot as #postgres_item_ddl>::ENUM_SCHEMA;
                            if enum_schema != "public" && seen_schemas.insert(enum_schema) {
                                snapshot.add_entity(MigEntity::Schema(MigSchema::new(enum_schema)));
                            }
                            snapshot.add_entity(MigEntity::Enum(MigEnum::from_strings(
                                enum_schema.to_string(),
                                enum_info.name().to_string(),
                                enum_info.variants().iter().map(|v| v.to_string()).collect(),
                            )));
                        }
                        #postgres_schema_type::View(view_info) => {
                            let view_schema = #sql_table_info::schema(view_info).unwrap_or("public");
                            if view_schema != "public" && seen_schemas.insert(view_schema) {
                                snapshot.add_entity(MigEntity::Schema(MigSchema::new(view_schema)));
                            }
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
                            if table_schema != "public" && seen_schemas.insert(table_schema) {
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
    let duplicates = crate::common::duplicate_schema_fields(fields);
    let mut unique_types = Vec::new();
    let mut seen = HashSet::new();
    for (_, ty) in fields {
        let key = quote!(#ty).to_string();
        if seen.insert(key) {
            unique_types.push(*ty);
        }
    }

    quote! {
        #duplicates
        #(
            impl #schema_has_table<#unique_types> for #struct_name {}
        )*
    }
}
