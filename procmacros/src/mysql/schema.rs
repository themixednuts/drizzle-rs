use crate::paths::{core as core_paths, migrations as mig_paths, mysql as mysql_paths};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Result};

/// Generate the runtime schema implementation for MySQL tables and indexes.
pub fn generate_mysql_schema_derive_impl(input: &DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[derive(MySQLSchema)] requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(MySQLSchema)] can only be applied to structs",
            ));
        }
    };

    let fields: Vec<_> = fields
        .iter()
        .map(|field| {
            field
                .ident
                .as_ref()
                .map(|name| (name, &field.ty))
                .ok_or_else(|| {
                    syn::Error::new_spanned(field, "#[derive(MySQLSchema)] fields must have names")
                })
        })
        .collect::<Result<_>>()?;
    let field_names: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();

    let sql_schema_impl = core_paths::sql_schema_impl();
    let validate_schema_item_foreign_keys = core_paths::validate_schema_item_foreign_keys();
    let mysql_value = mysql_paths::mysql_value();
    let mysql_schema_type = mysql_paths::mysql_schema_type();
    let create_statements = generate_create_statements_method(&fields);
    let table_refs = generate_schema_table_refs_method(&fields);
    let items = generate_items_method(&fields);
    let schema_has_table_impls = generate_schema_has_table_impls(struct_name, &fields);
    let foreign_key_assertions = generate_schema_fk_validation_asserts(
        &fields,
        struct_name,
        &validate_schema_item_foreign_keys,
    );
    let migration_schema_impl =
        generate_migration_schema_impl(struct_name, &field_types, &mysql_value, &mysql_schema_type);

    Ok(quote! {
        impl ::core::marker::Copy for #struct_name {}

        impl ::core::clone::Clone for #struct_name {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl ::core::fmt::Debug for #struct_name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter
                    .debug_struct(stringify!(#struct_name))
                    #(.field(stringify!(#field_names), &self.#field_names))*
                    .finish()
            }
        }

        impl ::core::default::Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_names: ::core::default::Default::default(),)*
                }
            }
        }

        impl #struct_name {
            pub const fn new() -> Self {
                Self {
                    #(#field_names: #field_types::new(),)*
                }
            }

            #items
        }

        impl #sql_schema_impl for #struct_name {
            fn table_refs(&self) -> &'static [&'static drizzle::core::TableRef] {
                #table_refs
            }

            fn create_statements(
                &self,
            ) -> ::std::result::Result<
                impl ::std::iter::Iterator<Item = ::std::string::String>,
                drizzle::error::DrizzleError,
            > {
                let statements: ::std::vec::Vec<::std::string::String> = { #create_statements };
                ::std::result::Result::Ok(statements.into_iter())
            }
        }

        impl ::std::convert::From<#struct_name> for (#(#field_types,)*) {
            fn from(schema: #struct_name) -> Self {
                (#(schema.#field_names,)*)
            }
        }

        #schema_has_table_impls
        #foreign_key_assertions
        #migration_schema_impl
    })
}

fn generate_create_statements_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql_index_info = core_paths::sql_index_info();
    let table_ref = core_paths::table_ref();
    let field_types: Vec<_> = fields.iter().map(|(_, ty)| *ty).collect();
    let mysql_value = mysql_paths::mysql_value();
    let mysql_schema_type = mysql_paths::mysql_schema_type();
    let mysql_view_info = mysql_paths::mysql_view_info();
    let order_schema_views = mysql_paths::order_schema_views();

    quote! {
        let mut tables: ::std::vec::Vec<(
            ::std::string::String,
            ::std::string::String,
            &'static #table_ref,
        )> = ::std::vec::Vec::new();
        let mut indexes: ::std::collections::HashMap<
            ::std::string::String,
            ::std::vec::Vec<::std::string::String>,
        > = ::std::collections::HashMap::new();
        let mut index_keys = ::std::collections::HashSet::<::std::string::String>::new();
        let mut views = ::std::vec::Vec::<&'static dyn #mysql_view_info>::new();

        #(
            match <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::TYPE {
                #mysql_schema_type::Table(table_ref) => {
                    tables.push((
                        table_ref.qualified_name.to_string(),
                        <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::SQL.to_string(),
                        table_ref,
                    ));
                }
                #mysql_schema_type::Index(index_info) => {
                    let table_name = #sql_index_info::table(index_info).qualified_name.to_string();
                    let index_name = #sql_index_info::name(index_info);
                    if !index_keys.insert(::std::format!("{table_name}::{index_name}")) {
                        return ::std::result::Result::Err(
                            drizzle::error::DrizzleError::Statement(
                                ::std::format!(
                                    "Duplicate index '{index_name}' on table '{table_name}' in MySQLSchema",
                                )
                                .into(),
                            ),
                        );
                    }
                    indexes
                        .entry(table_name)
                        .or_insert_with(::std::vec::Vec::new)
                        .push(
                            <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::SQL.to_string(),
                        );
                }
                #mysql_schema_type::View(view_info) => views.push(view_info),
            }
        )*

        tables.sort_by(|left, right| left.0.cmp(&right.0));
        let table_names: ::std::collections::HashSet<::std::string::String> =
            tables.iter().map(|(name, _, _)| name.clone()).collect();
        if table_names.len() != tables.len() {
            return ::std::result::Result::Err(
                drizzle::error::DrizzleError::Statement(
                    "Duplicate table names detected in MySQLSchema".into(),
                ),
            );
        }

        if let ::std::option::Option::Some(orphan) =
            indexes.keys().find(|table_name| !table_names.contains(*table_name))
        {
            return ::std::result::Result::Err(
                drizzle::error::DrizzleError::Statement(
                    ::std::format!(
                        "MySQLSchema contains an index for table '{orphan}', but not the table itself",
                    )
                    .into(),
                ),
            );
        }

        let mut indegree = ::std::collections::HashMap::<::std::string::String, usize>::new();
        let mut reverse_edges = ::std::collections::HashMap::<
            ::std::string::String,
            ::std::vec::Vec<::std::string::String>,
        >::new();
        for (table_name, _, table_ref) in &tables {
            indegree.entry(table_name.clone()).or_insert(0);
            let mut table_dependencies = ::std::collections::HashSet::<::std::string::String>::new();
            for foreign_key in table_ref.foreign_keys {
                let dependency = if foreign_key.target_schema.is_empty() {
                    foreign_key.target_table.to_string()
                } else {
                    ::std::format!(
                        "{}.{}",
                        foreign_key.target_schema,
                        foreign_key.target_table,
                    )
                };
                if dependency == *table_name || !table_names.contains(&dependency) {
                    continue;
                }
                if !table_dependencies.insert(dependency.clone()) {
                    continue;
                }
                *indegree
                    .get_mut(table_name)
                    .expect("every MySQL table has an indegree entry") += 1;
                reverse_edges
                    .entry(dependency)
                    .or_insert_with(::std::vec::Vec::new)
                    .push(table_name.clone());
            }
        }

        let mut ready: ::std::collections::BTreeSet<::std::string::String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(name, _)| name.clone())
            .collect();
        let mut ordered_names = ::std::vec::Vec::<::std::string::String>::with_capacity(tables.len());
        while let ::std::option::Option::Some(next) = ready.pop_first() {
            ordered_names.push(next.clone());
            if let ::std::option::Option::Some(children) = reverse_edges.get(&next) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("every dependent MySQL table has an indegree entry");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }

        if ordered_names.len() != tables.len() {
            let mut remaining: ::std::vec::Vec<_> = indegree
                .iter()
                .filter(|(_, degree)| **degree > 0)
                .map(|(name, _)| name.clone())
                .collect();
            remaining.sort_unstable();
            return ::std::result::Result::Err(
                drizzle::error::DrizzleError::Statement(
                    ::std::format!(
                        "Cyclic table dependency detected in MySQLSchema: {}",
                        remaining.join(", "),
                    )
                    .into(),
                ),
            );
        }

        let mut table_sql = ::std::collections::HashMap::<
            ::std::string::String,
            ::std::string::String,
        >::with_capacity(tables.len());
        for (table_name, sql, _) in tables {
            table_sql.insert(table_name, sql);
        }

        let mut statements = ::std::vec::Vec::<::std::string::String>::new();
        for table_name in ordered_names {
            statements.push(
                table_sql
                    .remove(&table_name)
                    .expect("ordered MySQL table must have SQL"),
            );
            if let ::std::option::Option::Some(table_indexes) = indexes.get(&table_name) {
                statements.extend(table_indexes.iter().cloned());
            }
        }
        statements.extend(#order_schema_views(&views)?);
        statements
    }
}

fn generate_items_method(fields: &[(&syn::Ident, &syn::Type)]) -> TokenStream {
    let item_refs = fields.iter().map(|(name, _)| quote!(&self.#name));
    let item_types = fields.iter().map(|(_, ty)| quote!(&#ty));

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
    let field_count = fields.len();

    quote! {
        static TABLE_REF_OPTIONS: [::core::option::Option<&'static #table_ref>; #field_count] = [
            #(<#field_types as #schema_item_tables>::TABLE_REF_CONST,)*
        ];
        const TABLE_REF_COUNT: usize = {
            let mut count = 0;
            let mut index = 0;
            while index < #field_count {
                if TABLE_REF_OPTIONS[index].is_some() {
                    count += 1;
                }
                index += 1;
            }
            count
        };
        static TABLE_REFS: [&'static #table_ref; TABLE_REF_COUNT] = {
            static EMPTY_TABLE_REF: #table_ref = #table_ref::sql("", &[]);
            let mut result = [&EMPTY_TABLE_REF; TABLE_REF_COUNT];
            let mut input = 0;
            let mut output = 0;
            while input < #field_count {
                if let ::core::option::Option::Some(table_ref) = TABLE_REF_OPTIONS[input] {
                    result[output] = table_ref;
                    output += 1;
                }
                input += 1;
            }
            result
        };
        &TABLE_REFS
    }
}

fn generate_schema_has_table_impls(
    schema: &syn::Ident,
    fields: &[(&syn::Ident, &syn::Type)],
) -> TokenStream {
    let schema_has_table = core_paths::schema_has_table();
    let mut seen = HashSet::new();
    let unique_types: Vec<_> = fields
        .iter()
        .map(|(_, ty)| *ty)
        .filter(|ty| seen.insert(quote!(#ty).to_string()))
        .collect();

    quote! {
        #(impl #schema_has_table<#unique_types> for #schema {})*
    }
}

fn generate_schema_fk_validation_asserts(
    fields: &[(&syn::Ident, &syn::Type)],
    schema: &syn::Ident,
    validate_schema_item_foreign_keys: &TokenStream,
) -> TokenStream {
    let field_types = fields.iter().map(|(_, ty)| *ty);

    quote! {
        const _: () = {
            const fn assert_schema_item<Item>()
            where
                Item: #validate_schema_item_foreign_keys<#schema>,
            {
            }

            #(assert_schema_item::<#field_types>();)*
        };
    }
}

/// Build the migration producer from generated structural metadata.  This is
/// intentionally separate from `create_statements`: snapshots retain values
/// such as generated expressions and online index options without parsing the
/// SQL that the normal schema path renders.
fn generate_migration_schema_impl(
    struct_name: &syn::Ident,
    field_types: &[&syn::Type],
    mysql_value: &TokenStream,
    mysql_schema_type: &TokenStream,
) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql_index_info = core_paths::sql_index_info();
    let sql_table_info = core_paths::sql_table_info();
    let schema_item_tables = core_paths::schema_item_tables();

    let mig_schema = mig_paths::schema();
    let mig_dialect = mig_paths::dialect();
    let mig_snapshot = mig_paths::snapshot();
    let mysql_snapshot = mig_paths::mysql::snapshot();
    let mysql_ddl = mig_paths::mysql::collection();
    let mysql_entity = mig_paths::mysql::entity();
    let mysql_table = mig_paths::mysql::table();
    let mysql_column = mig_paths::mysql::column();
    let mysql_index = mig_paths::mysql::index();
    let mysql_index_column = mig_paths::mysql::index_column();
    let mysql_primary_key = mig_paths::mysql::primary_key();
    let mysql_unique = mig_paths::mysql::unique_constraint();
    let mysql_foreign_key = mig_paths::mysql::foreign_key();
    let mysql_check = mig_paths::mysql::check_constraint();
    let mysql_generated = mig_paths::mysql::generated();
    let mysql_generated_type = mig_paths::mysql::generated_type();
    let mysql_inline_enum = mig_paths::mysql::inline_enum();
    let mysql_inline_type = mig_paths::mysql::inline_type();
    let mysql_action = mig_paths::mysql::referential_action();
    let mysql_index_method = mig_paths::mysql::index_method();
    let mysql_index_algorithm = mig_paths::mysql::index_algorithm();
    let mysql_index_lock = mig_paths::mysql::index_lock();
    let mysql_view = mig_paths::mysql::view();
    let mysql_view_algorithm = mig_paths::mysql::view_algorithm();
    let mysql_view_sql_security = mig_paths::mysql::view_sql_security();
    let mysql_view_check_option = mig_paths::mysql::view_check_option();

    quote! {
        impl #mig_schema for #struct_name {
            fn dialect(&self) -> #mig_dialect {
                #mig_dialect::MySQL
            }

            fn to_snapshot(&self) -> #mig_snapshot {
                type MigSnapshot = #mysql_snapshot;
                type MigDDL = #mysql_ddl;
                type MigEntity = #mysql_entity;
                type MigTable = #mysql_table;
                type MigColumn = #mysql_column;
                type MigIndex = #mysql_index;
                type MigIndexColumn = #mysql_index_column;
                type MigPrimaryKey = #mysql_primary_key;
                type MigUniqueConstraint = #mysql_unique;
                type MigForeignKey = #mysql_foreign_key;
                type MigCheckConstraint = #mysql_check;
                type MigGenerated = #mysql_generated;
                type MigGeneratedType = #mysql_generated_type;
                type MigInlineEnum = #mysql_inline_enum;
                type MigInlineType = #mysql_inline_type;
                type MigReferentialAction = #mysql_action;
                type MigIndexMethod = #mysql_index_method;
                type MigIndexAlgorithm = #mysql_index_algorithm;
                type MigIndexLock = #mysql_index_lock;
                type MigView = #mysql_view;
                type MigViewAlgorithm = #mysql_view_algorithm;
                type MigViewSqlSecurity = #mysql_view_sql_security;
                type MigViewCheckOption = #mysql_view_check_option;

                let mysql_action = |action: ::core::option::Option<&str>| -> ::core::option::Option<MigReferentialAction> {
                    match action {
                        ::core::option::Option::Some("CASCADE") => ::core::option::Option::Some(MigReferentialAction::Cascade),
                        ::core::option::Option::Some("SET NULL") => ::core::option::Option::Some(MigReferentialAction::SetNull),
                        ::core::option::Option::Some("RESTRICT") => ::core::option::Option::Some(MigReferentialAction::Restrict),
                        ::core::option::Option::Some("NO ACTION") => ::core::option::Option::Some(MigReferentialAction::NoAction),
                        _ => ::core::option::Option::None,
                    }
                };
                let mut snapshot = MigSnapshot::new();

                #(
                    match <#field_types as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::TYPE {
                        #mysql_schema_type::Table(_) => {
                            let table_ref = <#field_types as #schema_item_tables>::TABLE_REF_CONST
                                .expect("MySQL table schema item must have TABLE_REF_CONST");
                            let table_name = table_ref.name;
                            let database = table_ref.schema.map(::std::borrow::Cow::Borrowed);

                            let mut table = MigTable::new(table_name);
                            table.database = database.clone();
                            if let drizzle::core::TableDialect::MySQL {
                                is_temporary,
                                engine,
                                charset,
                                collate,
                                comment,
                            } = table_ref.dialect {
                                table.temporary = is_temporary;
                                table.engine = engine.map(::std::borrow::Cow::Borrowed);
                                table.charset = charset.map(::std::borrow::Cow::Borrowed);
                                table.collation = collate.map(::std::borrow::Cow::Borrowed);
                                table.comment = comment.map(::std::borrow::Cow::Borrowed);
                            }
                            snapshot.add_entity(MigEntity::Table(table));

                            let mut primary_columns = ::std::vec::Vec::<::std::borrow::Cow<'static, str>>::new();
                            for column_ref in table_ref.columns {
                                let (
                                    autoincrement,
                                    default,
                                    generated_expression,
                                    generated_stored,
                                    charset,
                                    collate,
                                    on_update,
                                ) = match column_ref.dialect {
                                    drizzle::core::ColumnDialect::MySQL {
                                        auto_increment,
                                        default,
                                        generated_expression,
                                        generated_stored,
                                        charset,
                                        collate,
                                        on_update,
                                    } => (
                                        auto_increment,
                                        default,
                                        generated_expression,
                                        generated_stored,
                                        charset,
                                        collate,
                                        on_update,
                                    ),
                                    _ => (
                                        false,
                                        ::core::option::Option::None,
                                        ::core::option::Option::None,
                                        false,
                                        ::core::option::Option::None,
                                        ::core::option::Option::None,
                                        ::core::option::Option::None,
                                    ),
                                };

                                let mut column = MigColumn::new(table_name, column_ref.name, column_ref.sql_type);
                                column.database = database.clone();
                                column.not_null = column_ref.not_null();
                                column.autoincrement = autoincrement;
                                column.primary_key = column_ref.primary_key();
                                column.unique = column_ref.unique() && !column_ref.primary_key();
                                column.default = default.map(::std::borrow::Cow::Borrowed);
                                column.on_update = on_update.map(::std::borrow::Cow::Borrowed);
                                column.generated = generated_expression.map(|expression| MigGenerated {
                                    expression: ::std::borrow::Cow::Borrowed(expression),
                                    generation_type: if generated_stored {
                                        MigGeneratedType::Stored
                                    } else {
                                        MigGeneratedType::Virtual
                                    },
                                });
                                column.inline_type = match <#field_types as drizzle::mysql::index::MySQLSchemaItemMetadata>::inline_type(column_ref.name) {
                                    ::core::option::Option::Some(drizzle::mysql::index::MySQLInlineTypeMetadata::Enum(values)) => {
                                        ::core::option::Option::Some(MigInlineType::Enum(MigInlineEnum {
                                            values: values.iter().copied().map(::std::borrow::Cow::Borrowed).collect(),
                                        }))
                                    }
                                    ::core::option::Option::Some(drizzle::mysql::index::MySQLInlineTypeMetadata::Set(values)) => {
                                        ::core::option::Option::Some(MigInlineType::Set(MigInlineEnum {
                                            values: values.iter().copied().map(::std::borrow::Cow::Borrowed).collect(),
                                        }))
                                    }
                                    ::core::option::Option::None => ::core::option::Option::None,
                                };
                                column.charset = charset.map(::std::borrow::Cow::Borrowed);
                                column.collation = collate.map(::std::borrow::Cow::Borrowed);
                                column.comment = <#field_types as drizzle::mysql::index::MySQLSchemaItemMetadata>::column_comment(column_ref.name)
                                    .map(::std::borrow::Cow::Borrowed);
                                snapshot.add_entity(MigEntity::Column(column));

                                if column_ref.primary_key() {
                                    primary_columns.push(::std::borrow::Cow::Borrowed(column_ref.name));
                                }
                            }

                            if !primary_columns.is_empty() {
                                snapshot.add_entity(MigEntity::PrimaryKey(MigPrimaryKey {
                                    database: database.clone(),
                                    table: ::std::borrow::Cow::Borrowed(table_name),
                                    name: ::core::option::Option::None,
                                    columns: primary_columns,
                                }));
                            }

                            for foreign_key_ref in table_ref.foreign_keys {
                                let foreign_database = if foreign_key_ref.target_schema.is_empty() {
                                    ::core::option::Option::None
                                } else {
                                    ::core::option::Option::Some(::std::borrow::Cow::Borrowed(foreign_key_ref.target_schema))
                                };
                                snapshot.add_entity(MigEntity::ForeignKey(MigForeignKey {
                                    database: database.clone(),
                                    table: ::std::borrow::Cow::Borrowed(table_name),
                                    name: ::std::borrow::Cow::Borrowed(foreign_key_ref.name),
                                    columns: foreign_key_ref.source_columns.iter().copied().map(::std::borrow::Cow::Borrowed).collect(),
                                    foreign_database,
                                    foreign_table: ::std::borrow::Cow::Borrowed(foreign_key_ref.target_table),
                                    foreign_columns: foreign_key_ref.target_columns.iter().copied().map(::std::borrow::Cow::Borrowed).collect(),
                                    on_delete: mysql_action(foreign_key_ref.on_delete),
                                    on_update: mysql_action(foreign_key_ref.on_update),
                                }));
                            }

                            for constraint_ref in table_ref.constraints {
                                match constraint_ref.kind {
                                    drizzle::core::SQLConstraintKind::Unique => {
                                        if let ::core::option::Option::Some(name) = constraint_ref.name {
                                            snapshot.add_entity(MigEntity::UniqueConstraint(MigUniqueConstraint {
                                                database: database.clone(),
                                                table: ::std::borrow::Cow::Borrowed(table_name),
                                                name: ::std::borrow::Cow::Borrowed(name),
                                                columns: constraint_ref.columns.iter().copied().map(::std::borrow::Cow::Borrowed).collect(),
                                            }));
                                        }
                                    }
                                    drizzle::core::SQLConstraintKind::Check => {
                                        if let (::core::option::Option::Some(name), ::core::option::Option::Some(expression)) =
                                            (constraint_ref.name, constraint_ref.check_expression)
                                        {
                                            snapshot.add_entity(MigEntity::CheckConstraint(MigCheckConstraint {
                                                database: database.clone(),
                                                table: ::std::borrow::Cow::Borrowed(table_name),
                                                name: ::std::borrow::Cow::Borrowed(name),
                                                expression: ::std::borrow::Cow::Borrowed(expression),
                                                enforced: ::core::option::Option::None,
                                            }));
                                        }
                                    }
                                    drizzle::core::SQLConstraintKind::PrimaryKey
                                    | drizzle::core::SQLConstraintKind::ForeignKey => {}
                                }
                            }
                        }
                        #mysql_schema_type::Index(index_info) => {
                            let table_ref = #sql_index_info::table(index_info);
                            let using = <#field_types as drizzle::mysql::index::MySQLSchemaItemMetadata>::INDEX_METHOD
                                .map(|method| match method {
                                    drizzle::mysql::index::MySQLIndexMethod::BTree => MigIndexMethod::Btree,
                                    drizzle::mysql::index::MySQLIndexMethod::Hash => MigIndexMethod::Hash,
                                });
                            let algorithm = <#field_types as drizzle::mysql::index::MySQLSchemaItemMetadata>::INDEX_ALGORITHM
                                .map(|algorithm| match algorithm {
                                    drizzle::mysql::index::MySQLIndexAlgorithm::Default => MigIndexAlgorithm::Default,
                                    drizzle::mysql::index::MySQLIndexAlgorithm::Inplace => MigIndexAlgorithm::Inplace,
                                    drizzle::mysql::index::MySQLIndexAlgorithm::Copy => MigIndexAlgorithm::Copy,
                                });
                            let lock = <#field_types as drizzle::mysql::index::MySQLSchemaItemMetadata>::INDEX_LOCK
                                .map(|lock| match lock {
                                    drizzle::mysql::index::MySQLIndexLock::Default => MigIndexLock::Default,
                                    drizzle::mysql::index::MySQLIndexLock::None => MigIndexLock::None,
                                    drizzle::mysql::index::MySQLIndexLock::Shared => MigIndexLock::Shared,
                                    drizzle::mysql::index::MySQLIndexLock::Exclusive => MigIndexLock::Exclusive,
                                });
                            snapshot.add_entity(MigEntity::Index(MigIndex {
                                database: table_ref.schema.map(::std::borrow::Cow::Borrowed),
                                table: ::std::borrow::Cow::Borrowed(table_ref.name),
                                name: ::std::borrow::Cow::Borrowed(#sql_index_info::name(index_info)),
                                columns: #sql_index_info::columns(index_info).iter().copied().map(MigIndexColumn::column).collect(),
                                unique: #sql_index_info::is_unique(index_info),
                                using,
                                algorithm,
                                lock,
                                comment: ::core::option::Option::None,
                                visible: ::core::option::Option::None,
                            }));
                        }
                        #mysql_schema_type::View(view_info) => {
                            let definition = view_info.definition_sql();
                            let mut view = MigView::new(
                                #sql_table_info::name(view_info),
                                definition.clone(),
                            );
                            view.database = #sql_table_info::schema(view_info)
                                .map(::std::borrow::Cow::Borrowed);
                            if definition.is_empty() {
                                view.definition = ::core::option::Option::None;
                            }
                            view.algorithm = view_info.algorithm().map(|algorithm| match algorithm {
                                drizzle::mysql::ViewAlgorithm::Undefined => MigViewAlgorithm::Undefined,
                                drizzle::mysql::ViewAlgorithm::Merge => MigViewAlgorithm::Merge,
                                drizzle::mysql::ViewAlgorithm::Temptable => MigViewAlgorithm::Temptable,
                            });
                            view.sql_security = view_info.sql_security().map(|security| match security {
                                drizzle::mysql::ViewSqlSecurity::Definer => MigViewSqlSecurity::Definer,
                                drizzle::mysql::ViewSqlSecurity::Invoker => MigViewSqlSecurity::Invoker,
                            });
                            view.check_option = view_info.check_option().map(|option| match option {
                                drizzle::mysql::ViewCheckOption::Cascaded => MigViewCheckOption::Cascaded,
                                drizzle::mysql::ViewCheckOption::Local => MigViewCheckOption::Local,
                            });
                            view.is_existing = view_info.is_existing();
                            snapshot.add_entity(MigEntity::View(view));
                        }
                    }
                )*

                let ddl = MigDDL::try_from_entities(snapshot.ddl.clone()).expect(
                    "MySQLSchema must use one database scope with complete table references",
                );
                snapshot.ddl = ddl.to_entities();
                #mig_snapshot::MySQL(snapshot)
            }
        }
    }
}
