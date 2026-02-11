use crate::paths::{core as core_paths, migrations as mig_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the SQLite Schema derive implementation
pub fn generate_sqlite_schema_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_schema_impl = core_paths::sql_schema_impl();
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
    let mig_sqlite_primary_key = mig_paths::sqlite::primary_key();
    let mig_sqlite_unique_constraint = mig_paths::sqlite::unique_constraint();
    let mig_sqlite_view = mig_paths::sqlite::view();

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
                #mig_dialect::SQLite
            }

            fn to_snapshot(&self) -> #mig_snapshot {
                // Use type aliases to avoid name collisions with user types
                type MigSnapshot = #mig_sqlite_snapshot;
                type MigEntity = #mig_sqlite_entity;
                type MigTable = #mig_sqlite_table;
                type MigColumn = #mig_sqlite_column;
                type MigIndex = #mig_sqlite_index;
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
                                    .map(|c| c.to_string())
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
        let mut tables: ::std::vec::Vec<(&str, ::std::string::String, &dyn #sql_table_info)> = ::std::vec::Vec::new();
        let mut indexes: ::std::collections::HashMap<&str, ::std::vec::Vec<::std::string::String>> = ::std::collections::HashMap::new();
        let mut views: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();

        // Collect all tables and indexes
        #(
            match <#field_types as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE {
                #sqlite_schema_type::Table(table_info) => {
                    let table_name = #sql_table_info::name(table_info);
                    let table_sql = <_ as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::sql(&self.#field_names).sql();
                    tables.push((table_name, table_sql, table_info));
                }
                #sqlite_schema_type::Index(index_info) => {
                    let index_sql = <_ as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::sql(&self.#field_names).sql();
                    let table_name = #sql_table_info::name(#sql_index_info::table(index_info));
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
        tables.sort_by(|a, b| a.0.cmp(b.0));
        let table_names: ::std::collections::HashSet<&str> =
            tables.iter().map(|(name, _, _)| *name).collect();

        let mut indegree: ::std::collections::HashMap<&str, usize> =
            ::std::collections::HashMap::with_capacity(tables.len());
        let mut reverse_edges: ::std::collections::HashMap<&str, ::std::vec::Vec<&str>> =
            ::std::collections::HashMap::new();

        for (table_name, _, table_info) in &tables {
            indegree.entry(*table_name).or_insert(0);

            for dep_name in #sql_table_info::dependencies(*table_info)
                .iter()
                .map(|dep| #sql_table_info::name(*dep))
                .filter(|dep_name| table_names.contains(dep_name))
            {
                *indegree
                    .get_mut(table_name)
                    .expect("indegree is initialized for each table") += 1;
                reverse_edges
                    .entry(dep_name)
                    .or_insert_with(::std::vec::Vec::new)
                    .push(*table_name);
            }
        }

        let mut ready: ::std::collections::BTreeSet<&str> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(name, _)| *name)
            .collect();
        let mut ordered_names: ::std::vec::Vec<&str> = ::std::vec::Vec::with_capacity(tables.len());

        while let ::std::option::Option::Some(next) = ready.pop_first() {
            ordered_names.push(next);

            if let ::std::option::Option::Some(children) = reverse_edges.get(next) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("child table must exist in indegree map");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(*child);
                    }
                }
            }
        }

        if ordered_names.len() != tables.len() {
            let mut remaining: ::std::vec::Vec<&str> = indegree
                .iter()
                .filter(|(_, degree)| **degree > 0)
                .map(|(name, _)| *name)
                .collect();
            remaining.sort_unstable();
            panic!(
                "Cyclic table dependency detected in SQLiteSchema: {}",
                remaining.join(", ")
            );
        }

        let mut table_by_name: ::std::collections::HashMap<
            &str,
            (::std::string::String, &dyn #sql_table_info),
        > = ::std::collections::HashMap::with_capacity(tables.len());
        for (table_name, table_sql, table_info) in tables {
            table_by_name.insert(table_name, (table_sql, table_info));
        }

        // Build final SQL statements: tables in dependency order, then their indexes
        let mut sql_statements = ::std::vec::Vec::<::std::string::String>::new();
        for table_name in ordered_names {
            let (table_sql, _) = table_by_name
                .remove(table_name)
                .expect("table exists after topological ordering");
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
