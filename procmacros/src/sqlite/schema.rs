use crate::paths::{core as core_paths, migrations as mig_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Result};

/// Generates the `SQLite` Schema derive implementation
pub fn generate_sqlite_schema_derive_impl(input: &DeriveInput) -> Result<TokenStream> {
    crate::common::reject_schema_trait_derives(input, "SQLiteSchema")?;
    let struct_name = &input.ident;

    // Get paths for fully-qualified types
    let sql_schema = core_paths::sql_schema();
    let sql_schema_impl = core_paths::sql_schema_impl();
    let validate_schema_item_foreign_keys = core_paths::validate_schema_item_foreign_keys();
    let sql_table_info = core_paths::sql_table_info();
    let sql_index_info = core_paths::sql_index_info();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();

    // Extract fields from the struct
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named_fields) => &named_fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[derive(SQLiteSchema)] requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(SQLiteSchema)] can only be applied to structs",
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
                    syn::Error::new_spanned(field, "#[derive(SQLiteSchema)] fields must have names")
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
    let mig_sqlite_snapshot = mig_paths::sqlite::snapshot();
    let mig_sqlite_entity = mig_paths::sqlite::entity();
    let mig_sqlite_table = mig_paths::sqlite::table();
    let mig_sqlite_column = mig_paths::sqlite::column();
    let mig_sqlite_index = mig_paths::sqlite::index();
    let mig_sqlite_index_column = mig_paths::sqlite::index_column();
    let mig_sqlite_primary_key = mig_paths::sqlite::primary_key();
    let mig_sqlite_unique_constraint = mig_paths::sqlite::unique_constraint();
    let mig_sqlite_check_constraint = mig_paths::sqlite::check_constraint();
    let mig_sqlite_foreign_key = mig_paths::sqlite::foreign_key();
    let mig_sqlite_generated = quote! { drizzle::migrations::sqlite::Generated };
    let mig_sqlite_generated_type = quote! { drizzle::migrations::sqlite::GeneratedType };
    let mig_sqlite_view = mig_paths::sqlite::view();

    let schema_table_refs_method = generate_schema_table_refs_method(&all_fields);
    let schema_has_table_impls = generate_schema_has_table_impls(struct_name, &all_fields);
    let schema_name_check = crate::common::schema_name_check(
        struct_name,
        &all_fields,
        &crate::common::constraints::DialectTypes::sqlite(),
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

            /// Get all schema items (tables and indexes) in field order
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
                    let empty = drizzle::migrations::Snapshot::empty(drizzle::Dialect::SQLite);
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
                #mig_dialect::SQLite
            }

            fn to_snapshot(&self) -> #mig_snapshot {
                // Use type aliases to avoid name collisions with user types
                type MigSnapshot = #mig_sqlite_snapshot;
                type MigEntity = #mig_sqlite_entity;
                type MigTable = #mig_sqlite_table;
                type MigColumn = #mig_sqlite_column;
                type MigForeignKey = #mig_sqlite_foreign_key;
                type MigIndex = #mig_sqlite_index;
                type MigIndexColumn = #mig_sqlite_index_column;
                type MigPrimaryKey = #mig_sqlite_primary_key;
                type MigUniqueConstraint = #mig_sqlite_unique_constraint;
                type MigCheckConstraint = #mig_sqlite_check_constraint;
                type MigView = #mig_sqlite_view;

                let mut snapshot = MigSnapshot::new();

                // Iterate through all schema fields and add DDL entities
                #(
                    match <#field_types_for_snapshot as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE {
                        #sqlite_schema_type::Table(_table_info) => {
                            // Use const TABLE_REF for column metadata instead of dyn traits
                            let table_ref = <#field_types_for_snapshot as drizzle::core::SchemaItemTables>::TABLE_REF_CONST
                                .expect("table must have TABLE_REF_CONST");
                            let table_name = table_ref.name;
                            let mut mig_table = MigTable::new(table_name);
                            if let drizzle::core::TableDialect::SQLite { without_rowid, strict } = table_ref.dialect {
                                mig_table.strict = strict;
                                mig_table.without_rowid = without_rowid;
                            }
                            snapshot.add_entity(MigEntity::Table(mig_table));

                            // Primary-key columns are collected across the column
                            // loop and emitted as ONE PrimaryKey entity below
                            // (per-column entities would mangle composite PKs).
                            let mut pk_columns: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();

                            // Add column entities from TABLE_REF
                            for col in table_ref.columns {
                                let (
                                    autoincrement,
                                    default,
                                    generated_expression,
                                    generated_stored,
                                    collate,
                                ) = match col.dialect {
                                    drizzle::core::ColumnDialect::SQLite {
                                        autoincrement,
                                        default,
                                        generated_expression,
                                        generated_stored,
                                        collate,
                                        ..
                                    } => (
                                        autoincrement,
                                        default,
                                        generated_expression,
                                        generated_stored,
                                        collate,
                                    ),
                                    _ => (false, ::core::option::Option::None, ::core::option::Option::None, false, ::core::option::Option::None),
                                };

                                let mut column = MigColumn::new(
                                    table_name,
                                    col.name,
                                    col.sql_type,
                                );
                                if col.not_null() {
                                    column = column.not_null();
                                }
                                if autoincrement {
                                    column = column.autoincrement();
                                }
                                if let ::core::option::Option::Some(default) = default {
                                    column = column.default_value(default);
                                }
                                if let ::core::option::Option::Some(expression) = generated_expression {
                                    column.generated = ::core::option::Option::Some(#mig_sqlite_generated {
                                        expression: ::std::borrow::Cow::Borrowed(expression),
                                        gen_type: if generated_stored {
                                            #mig_sqlite_generated_type::Stored
                                        } else {
                                            #mig_sqlite_generated_type::Virtual
                                        },
                                    });
                                }
                                if let ::core::option::Option::Some(collate) = collate {
                                    column.collate = ::core::option::Option::Some(::std::borrow::Cow::Borrowed(collate));
                                }
                                snapshot.add_entity(MigEntity::Column(column));

                                // Collect primary key columns (declaration order)
                                if col.primary_key() {
                                    pk_columns.push(col.name.to_string());
                                }

                                // Add unique constraint entity if this column is unique
                                if col.unique() {
                                    snapshot.add_entity(MigEntity::UniqueConstraint(MigUniqueConstraint::from_strings(
                                        table_name.to_string(),
                                        ::std::format!("{}_{}_unique", table_name, col.name),
                                        ::std::vec![col.name.to_string()],
                                    )));
                                }
                            }

                            // Emit ONE PrimaryKey entity covering all PK columns
                            // (single-column and composite alike).
                            if !pk_columns.is_empty() {
                                snapshot.add_entity(MigEntity::PrimaryKey(MigPrimaryKey::from_strings(
                                    table_name.to_string(),
                                    ::std::format!("{}_pk", table_name),
                                    pk_columns,
                                )));
                            }

                            // Emit ForeignKey entities from the table's FK refs
                            // (covers both column-level `references = ...` and
                            // table-level composite FOREIGN_KEY attributes).
                            for fk in table_ref.foreign_keys {
                                let mut mig_fk = MigForeignKey::from_strings(
                                    table_name.to_string(),
                                    fk.name.to_string(),
                                    fk.source_columns.iter().map(|c| c.to_string()).collect(),
                                    fk.target_table.to_string(),
                                    fk.target_columns.iter().map(|c| c.to_string()).collect(),
                                );
                                if let ::core::option::Option::Some(action) = fk.on_delete {
                                    mig_fk = mig_fk.on_delete(action.to_string());
                                }
                                if let ::core::option::Option::Some(action) = fk.on_update {
                                    mig_fk = mig_fk.on_update(action.to_string());
                                }
                                mig_fk.name_explicit = fk.name_explicit;
                                snapshot.add_entity(MigEntity::ForeignKey(mig_fk));
                            }

                            for constraint in table_ref.constraints {
                                match constraint.kind {
                                    drizzle::core::SQLConstraintKind::Unique => {
                                        let unique_name = constraint.name.unwrap_or("unique");
                                        let mut unique = MigUniqueConstraint::from_strings(
                                            table_name.to_string(),
                                            unique_name.to_string(),
                                            constraint.columns.iter().map(|col| col.to_string()).collect(),
                                        );
                                        unique.name_explicit = constraint.name_explicit;
                                        snapshot.add_entity(MigEntity::UniqueConstraint(unique));
                                    }
                                    drizzle::core::SQLConstraintKind::Check => {
                                        if let ::core::option::Option::Some(check_expression) = constraint.check_expression {
                                            let check_name = constraint.name.unwrap_or("check");
                                            snapshot.add_entity(MigEntity::CheckConstraint(MigCheckConstraint::new(
                                                table_name,
                                                check_name,
                                                check_expression,
                                            )));
                                        }
                                    }
                                    // PK data comes from the per-column flags above;
                                    // FK data comes from table_ref.foreign_keys
                                    // (ConstraintRef carries no FK target info).
                                    drizzle::core::SQLConstraintKind::PrimaryKey
                                    | drizzle::core::SQLConstraintKind::ForeignKey => {}
                                }
                            }
                        }
                        #sqlite_schema_type::Index(index_info) => {
                            // Add index entity
                            let idx_table_ref = #sql_index_info::table(index_info);
                            let mut idx = MigIndex::new(
                                idx_table_ref.name,
                                #sql_index_info::name(index_info),
                                #sql_index_info::columns(index_info)
                                    .iter()
                                    .map(|c| MigIndexColumn::new(*c))
                                    .collect::<::std::vec::Vec<_>>(),
                            );
                            if #sql_index_info::is_unique(index_info) {
                                idx = idx.unique();
                            }
                            if let ::core::option::Option::Some(where_clause) =
                                #sql_index_info::where_clause(index_info)
                            {
                                idx.where_clause = ::core::option::Option::Some(
                                    ::std::borrow::Cow::Borrowed(where_clause),
                                );
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
