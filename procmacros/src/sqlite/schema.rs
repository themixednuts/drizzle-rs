use crate::paths::{core as core_paths, migrations as mig_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the SQLite Schema derive implementation
pub fn generate_sqlite_schema_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_schema_impl = core_paths::sql_schema_impl();
    let validate_schema_item_foreign_keys = core_paths::validate_schema_item_foreign_keys();
    let sql_table_info = core_paths::sql_table_info();
    let sql_index_info = core_paths::sql_index_info();
    let _sql_column_info = core_paths::sql_column_info();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_table_info = sqlite_paths::sqlite_table_info();
    let _sqlite_column_info = sqlite_paths::sqlite_column_info();

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
        .map(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| (ident, &field.ty))
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        field,
                        "SQLiteSchema can only be derived for structs with named fields",
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
    let mig_sqlite_snapshot = mig_paths::sqlite::snapshot();
    let mig_sqlite_entity = mig_paths::sqlite::entity();
    let mig_sqlite_table = mig_paths::sqlite::table();
    let mig_sqlite_column = mig_paths::sqlite::column();
    let mig_sqlite_index = mig_paths::sqlite::index();
    let mig_sqlite_index_column = mig_paths::sqlite::index_column();
    let mig_sqlite_primary_key = mig_paths::sqlite::primary_key();
    let mig_sqlite_unique_constraint = mig_paths::sqlite::unique_constraint();
    let mig_sqlite_view = mig_paths::sqlite::view();

    let schema_tables_method = generate_schema_tables_method(&all_fields);
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

        // Implement SQLSchemaImpl trait
        impl #sql_schema_impl for #struct_name {
            fn tables(&self) -> &'static [&'static dyn #sql_table_info] {
                #schema_tables_method
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
                #mig_dialect::SQLite
            }

            fn to_snapshot(&self) -> #mig_snapshot {
                // Use type aliases to avoid name collisions with user types
                type MigSnapshot = #mig_sqlite_snapshot;
                type MigEntity = #mig_sqlite_entity;
                type MigTable = #mig_sqlite_table;
                type MigColumn = #mig_sqlite_column;
                type MigIndex = #mig_sqlite_index;
                type MigIndexColumn = #mig_sqlite_index_column;
                type MigPrimaryKey = #mig_sqlite_primary_key;
                type MigUniqueConstraint = #mig_sqlite_unique_constraint;
                type MigView = #mig_sqlite_view;

                let mut snapshot = MigSnapshot::new();

                // Iterate through all schema fields and add DDL entities
                #(
                    match <#field_types_for_snapshot as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE {
                        #sqlite_schema_type::Table(table_info) => {
                            // Add table entity
                            let table_name = #sql_table_info::name(table_info);
                            snapshot.add_entity(MigEntity::Table(MigTable::new(table_name)));

                            // Add column entities
                            for col in #sqlite_table_info::sqlite_columns(table_info) {
                                // col is &dyn SQLiteColumnInfo which extends SQLColumnInfo
                                // Use method syntax since trait object vtable includes supertrait methods
                                let mut column = MigColumn::new(
                                    table_name,
                                    col.name(),
                                    col.r#type(),
                                );
                                if col.is_not_null() {
                                    column = column.not_null();
                                }
                                if col.is_autoincrement() {
                                    column = column.autoincrement();
                                }
                                // Note: primary_key and unique constraints are handled
                                // separately via PrimaryKey and UniqueConstraint entities
                                // rather than column-level flags in the DDL format
                                snapshot.add_entity(MigEntity::Column(column));

                                // Add primary key entity if this is a primary key column
                                if col.is_primary_key() {
                                    snapshot.add_entity(MigEntity::PrimaryKey(MigPrimaryKey::from_strings(
                                        table_name.to_string(),
                                        ::std::format!("{}_pk", table_name),
                                        ::std::vec![col.name().to_string()],
                                    )));
                                }

                                // Add unique constraint entity if this column is unique
                                if col.is_unique() {
                                    snapshot.add_entity(MigEntity::UniqueConstraint(MigUniqueConstraint::from_strings(
                                        table_name.to_string(),
                                        ::std::format!("{}_{}_unique", table_name, col.name()),
                                        ::std::vec![col.name().to_string()],
                                    )));
                                }
                            }
                        }
                        #sqlite_schema_type::Index(index_info) => {
                            // Add index entity
                            let table = #sql_index_info::table(index_info);
                            let mut idx = MigIndex::new(
                                #sql_table_info::name(table),
                                #sql_index_info::name(index_info),
                                #sql_index_info::columns(index_info)
                                    .iter()
                                    .map(|c| MigIndexColumn::new(*c))
                                    .collect::<::std::vec::Vec<_>>(),
                            );
                            if #sql_index_info::is_unique(index_info) {
                                idx = idx.unique();
                            }
                            snapshot.add_entity(MigEntity::Index(idx));
                        }
                        #sqlite_schema_type::View(view_info) => {
                            let mut view = MigView::new(#sql_table_info::name(view_info));
                            let definition = view_info.definition_sql();
                            if !definition.is_empty() {
                                view.definition = ::std::option::Option::Some(definition);
                            }
                            if view_info.is_existing() {
                                view.is_existing = true;
                            }
                            snapshot.add_entity(MigEntity::View(view));
                        }
                        #sqlite_schema_type::Trigger => {
                            // Triggers not implemented yet
                        }
                    }
                )*

                #mig_snapshot::Sqlite(snapshot)
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
    let sql_table_info = core_paths::sql_table_info();
    let sql_index_info = core_paths::sql_index_info();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();

    // Extract field names and types for easier iteration
    #[allow(unused_variables)]
    let field_names: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    #[allow(unused_variables)]
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    // Generate different implementations based on available features
    #[cfg(feature = "sqlite")]
    let impl_tokens = quote! {
        let mut tables: ::std::vec::Vec<(::std::string::String, ::std::string::String, &dyn #sql_table_info)> = ::std::vec::Vec::new();
        let mut indexes: ::std::collections::HashMap<::std::string::String, ::std::vec::Vec<::std::string::String>> = ::std::collections::HashMap::new();
        let mut index_keys: ::std::collections::HashSet<::std::string::String> = ::std::collections::HashSet::new();
        let mut views: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();

        // Collect all tables and indexes
        #(
            match <#field_types as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE {
                #sqlite_schema_type::Table(table_info) => {
                    let table_name = #sql_table_info::qualified_name(table_info).into_owned();
                    let table_sql = <_ as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::sql(&self.#field_names).sql();
                    tables.push((table_name, table_sql, table_info));
                }
                #sqlite_schema_type::Index(index_info) => {
                    let index_sql = <_ as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::sql(&self.#field_names).sql();
                    let table_name = #sql_table_info::qualified_name(#sql_index_info::table(index_info)).into_owned();
                    let index_name = #sql_index_info::name(index_info);
                    let index_key = ::std::format!("{}::{}", table_name, index_name);
                    if !index_keys.insert(index_key) {
                        return ::std::result::Result::Err(drizzle::error::DrizzleError::Statement(
                            ::std::format!("Duplicate index '{}' on table '{}' in SQLiteSchema", index_name, table_name).into(),
                        ));
                    }
                    indexes
                        .entry(table_name)
                        .or_insert_with(::std::vec::Vec::new)
                        .push(index_sql);
                }
                #sqlite_schema_type::View(view_info) => {
                    if !view_info.is_existing() {
                        let view_sql = <_ as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::sql(&self.#field_names).sql();
                        views.push(view_sql);
                    }
                }
                #sqlite_schema_type::Trigger => {
                    // Triggers not implemented yet
                }
            }
        )*

        // Deterministic topological ordering via Kahn's algorithm.
        // Guarantees dependency-safe order for DAGs in O(V + E), with
        // lexical tie-breaking for stable output.
        tables.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        let table_names: ::std::collections::HashSet<::std::string::String> =
            tables.iter().map(|(name, _, _)| name.clone()).collect();

        if table_names.len() != tables.len() {
            return ::std::result::Result::Err(drizzle::error::DrizzleError::Statement(
                "Duplicate table names detected in SQLiteSchema".into(),
            ));
        }

        let mut indegree: ::std::collections::HashMap<::std::string::String, usize> =
            ::std::collections::HashMap::with_capacity(tables.len());
        let mut reverse_edges: ::std::collections::HashMap<::std::string::String, ::std::vec::Vec<::std::string::String>> =
            ::std::collections::HashMap::new();

        for (table_name, _, table_info) in &tables {
            indegree.entry(table_name.clone()).or_insert(0);

            for dep_name in #sql_table_info::dependencies(*table_info)
                .iter()
                .map(|dep| #sql_table_info::qualified_name(*dep).into_owned())
                .filter(|dep_name| dep_name != table_name)
                .filter(|dep_name| table_names.contains(dep_name))
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
                    "Cyclic table dependency detected in SQLiteSchema: {}",
                    remaining.join(", ")
                )
                .into(),
            ));
        }

        let mut table_by_name: ::std::collections::HashMap<
            ::std::string::String,
            (::std::string::String, &dyn #sql_table_info),
        > = ::std::collections::HashMap::with_capacity(tables.len());
        for (table_name, table_sql, table_info) in tables {
            table_by_name.insert(table_name, (table_sql, table_info));
        }

        // Build final SQL statements: tables in dependency order, then their indexes
        let mut sql_statements = ::std::vec::Vec::<::std::string::String>::new();
        for table_name in ordered_names {
            let (table_sql, _) = table_by_name
                .remove(&table_name)
                .expect("table exists after topological ordering");
            sql_statements.push(table_sql);

            // Add indexes for this table
            if let ::std::option::Option::Some(table_indexes) = indexes.get(&table_name) {
                for index_sql in table_indexes {
                    sql_statements.push(index_sql.clone());
                }
            }
        }

        // Add views last (they depend on tables)
        sql_statements.extend(views);

        sql_statements
    };

    #[cfg(not(feature = "sqlite"))]
    let impl_tokens = quote! {
        ::std::vec::Vec::new()
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

fn generate_schema_tables_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql_table_info = core_paths::sql_table_info();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_value = sqlite_paths::sqlite_value();

    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    quote! {
        static TABLES: ::std::sync::LazyLock<::std::vec::Vec<&'static dyn #sql_table_info>> =
            ::std::sync::LazyLock::new(|| {
                let mut tables: ::std::vec::Vec<&'static dyn #sql_table_info> = ::std::vec::Vec::new();
                #(
                    if let #sqlite_schema_type::Table(table_info) =
                        <#field_types as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE
                    {
                        tables.push(table_info);
                    }
                )*
                tables
            });

        TABLES.as_slice()
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
