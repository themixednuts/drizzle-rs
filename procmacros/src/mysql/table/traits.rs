use super::context::MacroContext;
use crate::common::ref_gen::{self, ColumnRefInput, ConstraintRefInput, ForeignKeyRefInput};
use crate::generators::{DrizzleTableConfig, generate_drizzle_table};
use crate::mysql::field::{FieldInfo, MySQLDefault};
use crate::mysql::generators::{
    SQLTableConfig, generate_mysql_table, generate_sql_schema, generate_sql_table, generate_to_sql,
};
use crate::paths::core as core_paths;
use crate::paths::mysql as mysql_paths;
use drizzle_types::mysql::MySQLType;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{Ident, Result};

/// Generate trait implementations for the `MySQL` table
pub(super) fn generate_table_impls(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    _required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let columns_len = column_zst_idents.len();
    let struct_ident = ctx.struct_ident;
    let alias_type_ident = format_ident!("{}Alias", struct_ident);
    let table_name = &ctx.table_name;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_cons = core_paths::type_set_cons();
    let type_set_nil = core_paths::type_set_nil();
    let sql_table_info = core_paths::sql_table_info();
    let no_constraint = core_paths::no_constraint();
    let table_ref = core_paths::table_ref();
    let mysql_value = mysql_paths::mysql_value();
    let mysql_schema_type = mysql_paths::mysql_schema_type();

    // Render from the authoritative metadata ref so database qualification is
    // preserved in queries as well as DDL.
    let to_sql_body = quote! {
        #sql::table(<Self as drizzle::core::DrizzleTable>::TABLE_REF)
    };

    let column_names: Vec<&String> = ctx.field_infos.iter().map(|f| &f.column_name).collect();

    // Generate compile-time SQL for const SQL, using concatcp! for FK references
    let sql_const = super::ddl::generate_schema_sql_const(ctx);

    // Use generator functions for consistent pattern with SQLite
    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        &quote! { #table_name },
        &quote! {
            {
                #mysql_schema_type::Table(&<#struct_ident as drizzle::core::DrizzleTable>::TABLE_REF)
            }
        },
        &sql_const,
    );
    let dialect_types = crate::common::constraints::DialectTypes {
        sql_schema: core_paths::sql_schema(),
        schema_type: mysql_paths::mysql_schema_type(),
        value_type: mysql_paths::mysql_value(),
        unique_constraint_suffix: "_key",
    };
    let (foreign_key_impls, _sql_foreign_keys, foreign_keys_type, fk_constraint_idents) =
        crate::common::constraints::generate_foreign_keys(
            ctx.field_infos,
            &ctx.attrs.composite_foreign_keys,
            struct_ident,
            ctx.struct_vis,
        );
    let mut mysql_fk_sql_type_validations = Vec::new();
    for field in ctx.field_infos {
        let Some(foreign_key) = &field.foreign_key else {
            continue;
        };
        let source = format_ident!(
            "{}{}",
            struct_ident,
            field.ident.to_string().to_upper_camel_case()
        );
        let target = format_ident!(
            "{}{}",
            foreign_key.table,
            foreign_key.column.to_string().to_upper_camel_case()
        );
        mysql_fk_sql_type_validations.push(mysql_fk_sql_type_validation(&source, &target));
    }
    for foreign_key in &ctx.attrs.composite_foreign_keys {
        for (source_column, target_column) in foreign_key
            .source_columns
            .iter()
            .zip(&foreign_key.target_columns)
        {
            let source = format_ident!(
                "{}{}",
                struct_ident,
                source_column.to_string().to_upper_camel_case()
            );
            let target = format_ident!(
                "{}{}",
                foreign_key.target_table,
                target_column.to_string().to_upper_camel_case()
            );
            mysql_fk_sql_type_validations.push(mysql_fk_sql_type_validation(&source, &target));
        }
    }
    let (primary_key_impls, _sql_primary_key, primary_key_type, pk_constraint_ident) =
        crate::common::constraints::generate_primary_key(
            ctx.field_infos,
            struct_ident,
            ctx.struct_vis,
        );
    let (unique_constraint_impls, unique_constraint_idents) =
        crate::common::constraints::generate_unique_constraints(
            ctx.field_infos,
            struct_ident,
            ctx.struct_vis,
        );
    let (table_unique_constraint_impls, table_unique_constraint_idents) =
        generate_table_unique_constraints(ctx, struct_ident, ctx.struct_vis);
    let (check_constraint_impls, check_constraint_idents) =
        generate_check_constraints(ctx, struct_ident, ctx.struct_vis);

    let mut constraint_idents = Vec::new();
    if let Some(pk_ident) = pk_constraint_ident {
        constraint_idents.push(pk_ident);
    }
    constraint_idents.extend(fk_constraint_idents);
    constraint_idents.extend(unique_constraint_idents);
    constraint_idents.extend(table_unique_constraint_idents);
    constraint_idents.extend(check_constraint_idents);

    let constraints_type = if constraint_idents.is_empty() {
        quote! { #no_constraint }
    } else {
        quote! { (#(#constraint_idents,)*) }
    };

    let non_empty_marker = core_paths::non_empty_marker();
    let sql_table_impl = generate_sql_table(SQLTableConfig {
        struct_ident,
        select: quote! { #select_model },
        insert: quote! { #insert_model<'a, T> },
        update: quote! { #update_model<'a, #non_empty_marker> },
        aliased: quote! { #alias_type_ident },
        foreign_keys: quote! { #foreign_keys_type },
        primary_key: quote! { #primary_key_type },
        constraints: quote! { #constraints_type },
    });

    let mut dependencies = Vec::new();
    let mut seen_dependencies = HashSet::new();
    for field in ctx.field_infos {
        if let Some(fk) = &field.foreign_key {
            let name = fk.table.to_string();
            if seen_dependencies.insert(name) {
                dependencies.push(fk.table.clone());
            }
        }
    }
    for fk in &ctx.attrs.composite_foreign_keys {
        let name = fk.target_table.to_string();
        if seen_dependencies.insert(name) {
            dependencies.push(fk.target_table.clone());
        }
    }
    // Build dependency_names: &[<Dep>::TABLE_NAME, ...]
    let dependency_name_exprs: Vec<TokenStream> = dependencies
        .iter()
        .map(|dep| quote! { <#dep as drizzle::core::DrizzleTable>::NAME })
        .collect();
    let _dependencies_len = dependencies.len();
    let database_name = ctx.attrs.database.as_deref();
    let qualified_name = database_name.map_or_else(
        || table_name.clone(),
        |database| format!("{database}.{table_name}"),
    );

    // Build TABLE_REF const
    let column_dialect = core_paths::column_dialect();
    let table_dialect = core_paths::table_dialect();
    let table_ref_name_expr = quote! {
        <Self as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::NAME
    };
    let table_ref_qualified_name_expr = quote! { #qualified_name };
    let table_ref_schema_expr = database_name.map_or_else(
        || quote! { ::core::option::Option::None },
        |database| quote! { ::core::option::Option::Some(#database) },
    );
    let table_ref_columns: Vec<ColumnRefInput> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let auto_increment = f.is_auto_increment;
            let generated_expression = f.generated_column.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |generated| {
                    let expression = &generated.expression;
                    quote! { ::core::option::Option::Some(#expression) }
                },
            );
            let generated_stored = f
                .generated_column
                .as_ref()
                .is_some_and(|generated| generated.stored);
            let default = if f.generated_column.is_none() {
                f.default.as_ref().map_or_else(
                    || quote! { ::core::option::Option::None },
                    |default| {
                        let default_str = match default {
                            MySQLDefault::Literal(s) | MySQLDefault::RawSql(s) => s.clone(),
                        };
                        quote! { ::core::option::Option::Some(#default_str) }
                    },
                )
            } else {
                quote! { ::core::option::Option::None }
            };
            let collate = f.collate.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |collate| quote! { ::core::option::Option::Some(#collate) },
            );
            let charset = f.charset.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |charset| quote! { ::core::option::Option::Some(#charset) },
            );
            let on_update = f.on_update.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |on_update| quote! { ::core::option::Option::Some(#on_update) },
            );
            let flags = crate::common::ref_gen::ColumnRefFlags::new()
                .with(
                    crate::common::ref_gen::ColumnRefFlags::NOT_NULL,
                    !f.is_nullable,
                )
                .with(
                    crate::common::ref_gen::ColumnRefFlags::PRIMARY_KEY,
                    f.is_primary(),
                )
                .with(
                    crate::common::ref_gen::ColumnRefFlags::UNIQUE,
                    f.is_unique(),
                )
                .with(
                    crate::common::ref_gen::ColumnRefFlags::HAS_DEFAULT,
                    f.has_default,
                );
            ColumnRefInput {
                column_name: f.column_name.clone(),
                sql_type: f.sql_type_expr(),
                flags,
                dialect: quote! {
                    #column_dialect::MySQL {
                        auto_increment: #auto_increment,
                        default: #default,
                        generated_expression: #generated_expression,
                        generated_stored: #generated_stored,
                        charset: #charset,
                        collate: #collate,
                        on_update: #on_update,
                    }
                },
            }
        })
        .collect();
    let pk_columns: Vec<String> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_primary())
        .map(|f| f.column_name.clone())
        .collect();
    let mut table_ref_fks: Vec<ForeignKeyRefInput> = ctx
        .field_infos
        .iter()
        .filter_map(|f| {
            f.foreign_key.as_ref().map(|fk| {
                let target_table = &fk.table;
                let fk_name = format!("{}_{}_fkey", ctx.table_name, f.column_name);
                let target_schema = quote! {
                    match <#target_table as drizzle::core::DrizzleTable>::SCHEMA {
                        ::core::option::Option::Some(schema) => schema,
                        ::core::option::Option::None => "",
                    }
                };
                // Target column resolves through the referenced column's
                // `SQLSchema::NAME` const so explicit `name = "..."` renames
                // on the target table are honored.
                let target_column_expr = crate::common::constraints::cross_table_column_name_const(
                    &fk.table,
                    &fk.column,
                    &dialect_types,
                );
                ForeignKeyRefInput {
                    name: quote! { #fk_name },
                    name_explicit: false,
                    source_columns: vec![f.column_name.clone()],
                    target_schema,
                    target_table: quote! { <#target_table as drizzle::core::DrizzleTable>::NAME },
                    target_columns: vec![target_column_expr],
                    on_delete: fk.on_delete.clone(),
                    on_update: fk.on_update.clone(),
                    deferrable: false,
                    initially_deferred: false,
                }
            })
        })
        .collect();
    for cfk in &ctx.attrs.composite_foreign_keys {
        let target_table = &cfk.target_table;
        let source_columns: Vec<String> = cfk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| &f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let fk_name = format!("{}_{}_fkey", ctx.table_name, source_columns[0]);
        let target_schema = quote! {
            match <#target_table as drizzle::core::DrizzleTable>::SCHEMA {
                ::core::option::Option::Some(schema) => schema,
                ::core::option::Option::None => "",
            }
        };
        table_ref_fks.push(ForeignKeyRefInput {
            name: quote! { #fk_name },
            name_explicit: false,
            source_columns,
            target_schema,
            target_table: quote! { <#target_table as drizzle::core::DrizzleTable>::NAME },
            target_columns: cfk
                .target_columns
                .iter()
                .map(|col| {
                    crate::common::constraints::cross_table_column_name_const(
                        &cfk.target_table,
                        col,
                        &dialect_types,
                    )
                })
                .collect(),
            on_delete: cfk.on_delete.clone(),
            on_update: cfk.on_update.clone(),
            deferrable: false,
            initially_deferred: false,
        });
    }
    let mut table_ref_constraints: Vec<ConstraintRefInput> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            let expr = field.check_constraint.as_ref()?;
            Some(ConstraintRefInput {
                name: Some(format!("{}_{}_check", ctx.table_name, field.column_name)),
                name_explicit: false,
                kind: quote! { drizzle::core::SQLConstraintKind::Check },
                columns: vec![field.column_name.clone()],
                check_expression: Some(expr.clone()),
                deferrable: false,
                initially_deferred: false,
            })
        })
        .collect();
    for unique in &ctx.attrs.unique_constraints {
        let columns = table_unique_column_names(ctx, &unique.columns);
        let name = table_unique_name(ctx, &columns, &unique.name);
        table_ref_constraints.push(ConstraintRefInput {
            name: Some(name),
            name_explicit: unique.name.is_some(),
            kind: quote! { drizzle::core::SQLConstraintKind::Unique },
            columns,
            check_expression: None,
            deferrable: false,
            initially_deferred: false,
        });
    }
    for (idx, check) in ctx.attrs.check_constraints.iter().enumerate() {
        table_ref_constraints.push(ConstraintRefInput {
            name: Some(table_check_name(ctx, idx, &check.name)),
            name_explicit: check.name.is_some(),
            kind: quote! { drizzle::core::SQLConstraintKind::Check },
            columns: Vec::new(),
            check_expression: Some(check.expr.clone()),
            deferrable: false,
            initially_deferred: false,
        });
    }
    let is_temporary = ctx.attrs.temporary;
    let engine = ctx.attrs.engine.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |engine| quote! { ::core::option::Option::Some(#engine) },
    );
    let charset = ctx.attrs.charset.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |charset| quote! { ::core::option::Option::Some(#charset) },
    );
    let collate = ctx.attrs.collate.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |collate| quote! { ::core::option::Option::Some(#collate) },
    );
    let table_comment = ctx.attrs.comment.as_ref().or(ctx.table_comment.as_ref());
    let comment = table_comment.map_or_else(
        || quote! { ::core::option::Option::None },
        |comment| quote! { ::core::option::Option::Some(#comment) },
    );
    let table_ref_dialect = quote! {
        #table_dialect::MySQL {
            is_temporary: #is_temporary,
            engine: #engine,
            charset: #charset,
            collate: #collate,
            comment: #comment,
        }
    };
    let dep_names_expr = quote! { &[#(#dependency_name_exprs),*] };
    let table_ref_const = ref_gen::generate_table_ref_const(
        &table_ref_name_expr,
        &table_ref_qualified_name_expr,
        &table_ref_schema_expr,
        &column_names,
        &table_ref_columns,
        &pk_columns,
        &table_ref_fks,
        &table_ref_constraints,
        &dep_names_expr,
        &table_ref_dialect,
    );

    let drizzle_table_impl = generate_drizzle_table(DrizzleTableConfig {
        struct_ident,
        name: quote! {
            <Self as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::NAME
        },
        qualified_name: quote! { #qualified_name },
        schema: table_ref_schema_expr,
        dependency_names: quote! { &[#(#dependency_name_exprs),*] },
        table_ref_const,
    });

    let ddl_qualified_name = ctx.attrs.database.as_ref().map_or_else(
        || super::ddl::quoted(table_name),
        |database| {
            format!(
                "{}.{}",
                super::ddl::quoted(database),
                super::ddl::quoted(table_name),
            )
        },
    );
    let mysql_table_impl = generate_mysql_table(struct_ident, &quote!(#ddl_qualified_name));
    let to_sql_impl = generate_to_sql(struct_ident, &to_sql_body);

    // Generate compile-time relation marker impls
    let relations_impl = crate::common::constraints::generate_relations(
        ctx.field_infos,
        &ctx.attrs.composite_foreign_keys,
        ctx.struct_ident,
    )?;
    let has_check = ctx
        .field_infos
        .iter()
        .any(|f| f.check_constraint.as_ref().is_some())
        || !ctx.attrs.check_constraints.is_empty();
    let capability_impls = generate_mysql_constraint_capabilities(ctx, has_check);

    let has_select_model = core_paths::has_select_model();
    let into_select_target = core_paths::into_select_target();
    let select_star = core_paths::select_star();
    let insert_select_columns = column_zst_idents.iter().rev().fold(
        quote! { #type_set_nil },
        |tail, column| quote! { #type_set_cons<#column, #tail> },
    );
    let mysql_inline_type_match_arms: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            let column_name = &field.column_name;
            if field.is_enum {
                let enum_type = &field.base_type;
                Some(quote! {
                    #column_name => ::core::option::Option::Some(
                        drizzle::mysql::index::MySQLInlineTypeMetadata::Enum(
                            <#enum_type as drizzle::mysql::traits::MySQLEnum>::VARIANTS,
                        ),
                    ),
                })
            } else if let MySQLType::Set(values) = &field.column_type {
                Some(quote! {
                    #column_name => ::core::option::Option::Some(
                        drizzle::mysql::index::MySQLInlineTypeMetadata::Set(&[#(#values),*]),
                    ),
                })
            } else {
                None
            }
        })
        .collect();
    let mysql_column_comment_match_arms: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            let column_name = &field.column_name;
            field.comment.as_ref().map(|comment| {
                quote! {
                    #column_name => ::core::option::Option::Some(#comment),
                }
            })
        })
        .collect();

    Ok(quote! {
        #foreign_key_impls
        #(#mysql_fk_sql_type_validations)*
        #primary_key_impls
        #unique_constraint_impls
        #table_unique_constraint_impls
        #check_constraint_impls

        #sql_schema_impl
        #sql_table_impl
        #drizzle_table_impl
        impl<'a> #sql_table_info for &'a #struct_ident {
            fn name(&self) -> &'static str {
                <#struct_ident as #sql_table_info>::name(*self)
            }

            fn schema(&self) -> ::core::option::Option<&'static str> {
                <#struct_ident as #sql_table_info>::schema(*self)
            }

            fn qualified_name(&self) -> ::std::borrow::Cow<'static, str> {
                <#struct_ident as #sql_table_info>::qualified_name(*self)
            }
        }
        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_cons<#struct_ident, #type_set_nil>;
            const TABLE_REF_CONST: ::core::option::Option<&'static #table_ref> = {
                ::core::option::Option::Some(&<#struct_ident as drizzle::core::DrizzleTable>::TABLE_REF)
            };
        }
        impl drizzle::mysql::index::__private::MySQLSchemaItemSealed for #struct_ident {}
        impl drizzle::mysql::index::MySQLSchemaItemMetadata for #struct_ident {
            fn inline_type(
                column: &str,
            ) -> ::core::option::Option<drizzle::mysql::index::MySQLInlineTypeMetadata> {
                match column {
                    #(#mysql_inline_type_match_arms)*
                    _ => ::core::option::Option::None,
                }
            }

            fn column_comment(column: &str) -> ::core::option::Option<&'static str> {
                match column {
                    #(#mysql_column_comment_match_arms)*
                    _ => ::core::option::Option::None,
                }
            }
        }
        impl #has_select_model for #struct_ident {
            type SelectModel = #select_model;
            const COLUMN_COUNT: usize = #columns_len;
        }
        impl #into_select_target for #struct_ident {
            type Marker = #select_star;
        }
        impl drizzle::mysql::traits::MySQLInsertSelectTarget for #struct_ident {
            type Columns = #insert_select_columns;
        }
        #mysql_table_impl
        #to_sql_impl
        #relations_impl
        #capability_impls
    })
}

fn mysql_fk_sql_type_validation(source: &Ident, target: &Ident) -> TokenStream {
    quote! {
        const _: () = {
            fn assert_mysql_fk_sql_type<Source, Target>()
            where
                Source: drizzle::core::expr::Expr<
                    'static,
                    drizzle::mysql::MySQLValue<'static>,
                >,
                Target: drizzle::core::expr::Expr<
                    'static,
                    drizzle::mysql::MySQLValue<'static>,
                >,
                <Source as drizzle::core::expr::Expr<
                    'static,
                    drizzle::mysql::MySQLValue<'static>,
                >>::SQLType: drizzle::core::TypeEq<
                    <Target as drizzle::core::expr::Expr<
                        'static,
                        drizzle::mysql::MySQLValue<'static>,
                    >>::SQLType,
                >,
            {
            }

            let _ = assert_mysql_fk_sql_type::<#source, #target>;

            assert!(
                drizzle::mysql::traits::optional_str_eq(
                    <#source as drizzle::mysql::traits::MySQLColumn<'static>>::CHARSET,
                    <#target as drizzle::mysql::traits::MySQLColumn<'static>>::CHARSET,
                ),
                "MySQL foreign-key character sets must match",
            );
            assert!(
                drizzle::mysql::traits::optional_str_eq(
                    <#source as drizzle::mysql::traits::MySQLColumn<'static>>::COLLATE,
                    <#target as drizzle::mysql::traits::MySQLColumn<'static>>::COLLATE,
                ),
                "MySQL foreign-key collations must match",
            );
        };
    }
}

fn generate_mysql_constraint_capabilities(ctx: &MacroContext, has_check: bool) -> TokenStream {
    let table = ctx.struct_ident;
    let has_primary_key = core_paths::has_primary_key();
    let has_constraint = core_paths::has_constraint();
    let primary_key_kind = core_paths::primary_key_kind();
    let foreign_key_kind = core_paths::foreign_key_kind();
    let unique_kind = core_paths::unique_kind();
    let check_kind = core_paths::check_kind();

    let primary = ctx.field_infos.iter().any(FieldInfo::is_primary);
    let foreign = ctx
        .field_infos
        .iter()
        .any(|field| field.foreign_key.is_some())
        || !ctx.attrs.composite_foreign_keys.is_empty();
    let unique = ctx
        .field_infos
        .iter()
        .any(|field| field.is_unique() && !field.is_primary())
        || !ctx.attrs.unique_constraints.is_empty();

    let primary_impl = primary.then(|| {
        quote! {
            impl #has_primary_key for #table {}
            impl #has_constraint<#primary_key_kind> for #table {}
        }
    });
    let foreign_impl =
        foreign.then(|| quote!(impl #has_constraint<#foreign_key_kind> for #table {}));
    let unique_impl = unique.then(|| quote!(impl #has_constraint<#unique_kind> for #table {}));
    let check_impl = has_check.then(|| quote!(impl #has_constraint<#check_kind> for #table {}));

    quote!(#primary_impl #foreign_impl #unique_impl #check_impl)
}

fn table_unique_column_data(
    ctx: &MacroContext,
    unique: &crate::mysql::table::attributes::UniqueConstraintAttr,
) -> (Vec<Ident>, Vec<String>, Vec<TokenStream>) {
    let col_zsts = unique
        .columns
        .iter()
        .map(|src| {
            let pascal = src.to_string().to_upper_camel_case();
            format_ident!("{}{}", ctx.struct_ident, pascal)
        })
        .collect::<Vec<_>>();
    let col_names = table_unique_column_names(ctx, &unique.columns);
    let source_checks = unique
        .columns
        .iter()
        .map(|src| {
            let table = ctx.struct_ident;
            quote! {
                const _: () = { let _ = &#table::#src; };
            }
        })
        .collect::<Vec<_>>();

    (col_zsts, col_names, source_checks)
}

fn table_unique_column_names(ctx: &MacroContext, columns: &[Ident]) -> Vec<String> {
    columns
        .iter()
        .map(|src| {
            ctx.field_infos
                .iter()
                .find(|field| &field.ident == src)
                .map_or_else(|| src.to_string(), |field| field.column_name.clone())
        })
        .collect()
}

fn table_unique_name(ctx: &MacroContext, columns: &[String], explicit: &Option<String>) -> String {
    explicit
        .clone()
        .unwrap_or_else(|| format!("{}_{}_key", ctx.table_name, columns.join("_")))
}

fn table_check_name(ctx: &MacroContext, idx: usize, explicit: &Option<String>) -> String {
    explicit.clone().unwrap_or_else(|| {
        if ctx.attrs.check_constraints.len() == 1 {
            format!("{}_check", ctx.table_name)
        } else {
            format!("{}_check{}", ctx.table_name, idx + 1)
        }
    })
}

fn generate_table_unique_constraints(
    ctx: &MacroContext,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
) -> (TokenStream, Vec<Ident>) {
    let sql_constraint = core_paths::sql_constraint();
    let unique_kind = core_paths::unique_kind();
    let columns_belong_to = core_paths::columns_belong_to();
    let non_empty_col_set = core_paths::non_empty_col_set();
    let no_duplicate_col_set = core_paths::no_duplicate_col_set();

    let mut impls = Vec::new();
    let mut idents = Vec::new();

    for (idx, unique) in ctx.attrs.unique_constraints.iter().enumerate() {
        let uq_ident = format_ident!("__UniqueComposite_{}_{}", struct_ident, idx);
        let (col_zsts, _, source_checks) = table_unique_column_data(ctx, unique);
        let col_tuple = quote! { (#(#col_zsts,)*) };

        impls.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            #struct_vis struct #uq_ident;

            const _: () = {
                struct __ValidateUnique;
                impl #no_duplicate_col_set<#col_tuple> for __ValidateUnique {}

                const fn assert_unique()
                where
                    (): #non_empty_col_set<#col_tuple>
                        + #columns_belong_to<#struct_ident, #col_tuple>,
                    __ValidateUnique: #no_duplicate_col_set<#col_tuple>,
                {
                }
                assert_unique();
            };

            #(#source_checks)*

            impl #sql_constraint for #uq_ident {
                type Table = #struct_ident;
                type Kind = #unique_kind;
                type Columns = #col_tuple;
            }
        });

        idents.push(uq_ident);
    }

    (quote! { #(#impls)* }, idents)
}

fn generate_check_constraints(
    ctx: &MacroContext,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
) -> (TokenStream, Vec<Ident>) {
    let sql_constraint = core_paths::sql_constraint();
    let check_kind = core_paths::check_kind();
    let columns_belong_to = core_paths::columns_belong_to();
    let non_empty_col_set = core_paths::non_empty_col_set();
    let no_duplicate_col_set = core_paths::no_duplicate_col_set();

    let mut impls = Vec::new();
    let mut idents = Vec::new();

    for field in ctx
        .field_infos
        .iter()
        .filter(|f| f.check_constraint.as_ref().is_some())
    {
        let field_pascal = field.ident.to_string().to_upper_camel_case();
        let chk_ident = format_ident!("__Check_{}_{}", struct_ident, field_pascal);
        let col_ident = format_ident!("{}{}", struct_ident, field_pascal);

        impls.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #struct_vis struct #chk_ident;

            const _: () = {
                struct __ValidateCheck;
                impl #no_duplicate_col_set<(#col_ident,)> for __ValidateCheck {}

                const fn assert_check()
                where
                    (): #non_empty_col_set<(#col_ident,)>
                        + #columns_belong_to<#struct_ident, (#col_ident,)>,
                    __ValidateCheck: #no_duplicate_col_set<(#col_ident,)>,
                {
                }
                assert_check();
            };

            impl #sql_constraint for #chk_ident {
                type Table = #struct_ident;
                type Kind = #check_kind;
                type Columns = (#col_ident,);
            }
        });

        idents.push(chk_ident);
    }

    for (idx, _check) in ctx.attrs.check_constraints.iter().enumerate() {
        let chk_ident = format_ident!("__CheckComposite_{}_{}", struct_ident, idx);

        impls.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #struct_vis struct #chk_ident;

            impl #sql_constraint for #chk_ident {
                type Table = #struct_ident;
                type Kind = #check_kind;
                type Columns = ();
            }
        });

        idents.push(chk_ident);
    }

    (quote! { #(#impls)* }, idents)
}
