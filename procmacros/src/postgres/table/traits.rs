use super::context::MacroContext;
use crate::common::ref_gen::{self, ColumnRefInput, ConstraintRefInput, ForeignKeyRefInput};
use crate::generators::{DrizzleTableConfig, generate_drizzle_table};
use crate::paths::core as core_paths;
use crate::paths::postgres as postgres_paths;
use crate::postgres::generators::*;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{Ident, Result};

/// Generate trait implementations for the PostgreSQL table
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
    let sql_column_info = core_paths::sql_column_info();
    let no_constraint = core_paths::no_constraint();
    let table_ref = core_paths::table_ref();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    // Column names for TableRef
    let column_names: Vec<&String> = ctx.field_infos.iter().map(|f| &f.column_name).collect();

    // Generate ToSQL body using TableRef
    let to_sql_body = quote! {
        #sql::table(#table_ref::sql(Self::TABLE_NAME, &[#(#column_names),*]))
    };

    // Generate compile-time SQL for const SQL, using concatcp! for FK references
    let sql_const = super::ddl::generate_schema_sql_const(ctx);

    // Use generator functions for consistent pattern with SQLite
    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        quote! { #table_name },
        quote! {
            {
                #postgres_schema_type::Table(&<#struct_ident as drizzle::core::DrizzleTable>::TABLE_REF)
            }
        },
        sql_const,
    );
    let dialect_types = crate::common::constraints::DialectTypes {
        sql_schema: core_paths::sql_schema(),
        schema_type: postgres_paths::postgres_schema_type(),
        value_type: postgres_paths::postgres_value(),
    };
    let (foreign_key_impls, _sql_foreign_keys, foreign_keys_type, fk_constraint_idents) =
        crate::common::constraints::generate_foreign_keys(
            ctx.field_infos,
            &ctx.attrs.composite_foreign_keys,
            &ctx.table_name,
            struct_ident,
            ctx.struct_vis,
            &sql_table_info,
            &sql_column_info,
            &dialect_types,
        )?;
    let (primary_key_impls, _sql_primary_key, primary_key_type, pk_constraint_ident) =
        crate::common::constraints::generate_primary_key(
            ctx.field_infos,
            &ctx.table_name,
            struct_ident,
            ctx.struct_vis,
            &sql_table_info,
            &dialect_types,
        );
    let (unique_constraint_impls, unique_constraint_idents) =
        crate::common::constraints::generate_unique_constraints(
            ctx.field_infos,
            &ctx.table_name,
            struct_ident,
            ctx.struct_vis,
            &sql_table_info,
            &dialect_types,
        );
    let (check_constraint_impls, check_constraint_idents) =
        generate_check_constraints(ctx, struct_ident, ctx.struct_vis, &sql_table_info);

    let mut constraint_idents = Vec::new();
    if let Some(pk_ident) = pk_constraint_ident {
        constraint_idents.push(pk_ident);
    }
    constraint_idents.extend(fk_constraint_idents);
    constraint_idents.extend(unique_constraint_idents);
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
    let schema_name = ctx.attrs.schema.as_deref().unwrap_or("public");
    let qualified_name = format!("{}.{}", schema_name, table_name);

    // Build TABLE_REF const
    let column_dialect = core_paths::column_dialect();
    let table_dialect = core_paths::table_dialect();
    let table_ref_name_expr = quote! {
        <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::NAME
    };
    let table_ref_qualified_name_expr = quote! { #qualified_name };
    let table_ref_schema_expr = quote! { ::core::option::Option::Some(#schema_name) };
    let table_ref_columns: Vec<ColumnRefInput> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let pg_type = f.column_type.to_sql_type().to_string();
            let is_serial = f.is_serial
                && matches!(
                    f.column_type,
                    crate::postgres::field::PostgreSQLType::Serial
                );
            let is_bigserial = f.is_serial
                && matches!(
                    f.column_type,
                    crate::postgres::field::PostgreSQLType::Bigserial
                );
            let is_generated_identity = f.is_generated_identity;
            let is_identity_always = f
                .identity_mode
                .as_ref()
                .is_some_and(|m| matches!(m, crate::postgres::field::IdentityMode::Always));
            ColumnRefInput {
                column_name: f.column_name.clone(),
                sql_type: f.column_type.to_sql_type().to_string(),
                not_null: !f.is_nullable,
                primary_key: f.is_primary,
                unique: f.is_unique,
                has_default: f.has_default,
                dialect: quote! {
                    #column_dialect::PostgreSQL {
                        postgres_type: #pg_type,
                        is_serial: #is_serial,
                        is_bigserial: #is_bigserial,
                        is_generated_identity: #is_generated_identity,
                        is_identity_always: #is_identity_always,
                    }
                },
            }
        })
        .collect();
    let pk_columns: Vec<String> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_primary)
        .map(|f| f.column_name.clone())
        .collect();
    let mut table_ref_fks: Vec<ForeignKeyRefInput> = ctx
        .field_infos
        .iter()
        .filter_map(|f| {
            f.foreign_key.as_ref().map(|fk| {
                let target_table = &fk.table;
                ForeignKeyRefInput {
                    source_columns: vec![f.column_name.clone()],
                    target_table: quote! { <#target_table as drizzle::core::DrizzleTable>::NAME },
                    target_columns: vec![fk.column.to_string()],
                }
            })
        })
        .collect();
    for cfk in &ctx.attrs.composite_foreign_keys {
        let target_table = &cfk.target_table;
        table_ref_fks.push(ForeignKeyRefInput {
            source_columns: cfk.source_columns.iter().map(|c| c.to_string()).collect(),
            target_table: quote! { <#target_table as drizzle::core::DrizzleTable>::NAME },
            target_columns: cfk.target_columns.iter().map(|c| c.to_string()).collect(),
        });
    }
    let table_ref_constraints: Vec<ConstraintRefInput> = Vec::new(); // TODO: populate if needed
    let table_ref_dialect = quote! { #table_dialect::PostgreSQL };
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
            <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::NAME
        },
        qualified_name: quote! { #qualified_name },
        schema: quote! { ::std::option::Option::Some(#schema_name) },
        dependency_names: quote! { &[#(#dependency_name_exprs),*] },
        table_ref_const,
    });

    let postgres_table_impl = generate_postgres_table(struct_ident);
    let to_sql_impl = generate_to_sql(struct_ident, to_sql_body);

    // Generate compile-time relation marker impls
    let relations_impl = crate::common::constraints::generate_relations(
        ctx.field_infos,
        &ctx.attrs.composite_foreign_keys,
        ctx.struct_ident,
    )?;
    let has_check = ctx
        .field_infos
        .iter()
        .any(|f| f.check_constraint.as_ref().is_some());
    let capability_impls = crate::common::constraints::generate_constraint_capabilities(
        ctx.field_infos,
        &ctx.table_name,
        ctx.struct_ident,
        !ctx.attrs.composite_foreign_keys.is_empty(),
        has_check,
        &dialect_types,
    );

    let has_select_model = core_paths::has_select_model();
    let into_select_target = core_paths::into_select_target();
    let select_star = core_paths::select_star();

    Ok(quote! {
        #foreign_key_impls
        #primary_key_impls
        #unique_constraint_impls
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
        impl #has_select_model for #struct_ident {
            type SelectModel = #select_model;
            const COLUMN_COUNT: usize = #columns_len;
        }
        impl #into_select_target for #struct_ident {
            type Marker = #select_star;
        }
        #postgres_table_impl
        #to_sql_impl
        #relations_impl
        #capability_impls
    })
}

fn generate_check_constraints(
    ctx: &MacroContext,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
    sql_table_info: &TokenStream,
) -> (TokenStream, Vec<Ident>) {
    let sql_constraint_info = core_paths::sql_constraint_info();
    let sql_constraint = core_paths::sql_constraint();
    let sql_constraint_kind = core_paths::sql_constraint_kind();
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
        let constraint_name = format!("{}_{}_check", ctx.table_name, field.column_name);
        let col_name = field.column_name.clone();
        let expr = field
            .check_constraint
            .as_ref()
            .expect("filtered to check constraints")
            .clone();

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

            impl #sql_constraint_info for #chk_ident {
                fn table(&self) -> &'static dyn #sql_table_info {
                    #[allow(non_upper_case_globals)]
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }

                fn name(&self) -> Option<&'static str> {
                    Some(#constraint_name)
                }

                fn kind(&self) -> #sql_constraint_kind {
                    #sql_constraint_kind::Check
                }

                fn columns(&self) -> &'static [&'static str] {
                    &[#col_name]
                }

                fn check_expression(&self) -> Option<&'static str> {
                    Some(#expr)
                }
            }

            impl #sql_constraint for #chk_ident {
                type Table = #struct_ident;
                type Kind = #check_kind;
                type Columns = (#col_ident,);
            }
        });

        idents.push(chk_ident);
    }

    (quote! { #(#impls)* }, idents)
}
