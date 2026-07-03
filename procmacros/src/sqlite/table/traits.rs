use super::context::MacroContext;
use super::ddl::generate_schema_sql_const;
use crate::common::ref_gen::{self, ColumnRefInput, ConstraintRefInput, ForeignKeyRefInput};
use crate::generators::{DrizzleTableConfig, generate_drizzle_table};
use crate::paths::core as core_paths;
use crate::paths::sqlite as sqlite_paths;
use crate::sqlite::generators::{
    SQLTableConfig, generate_sql_schema, generate_sql_table, generate_sqlite_table, generate_to_sql,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::Result;

/// Generates the `SQLSchema` and `SQLTable` implementations.
pub fn generate_table_impls(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    _required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let columns_len = column_zst_idents.len();
    let strict = ctx.attrs.strict;
    let without_rowid = ctx.attrs.without_rowid;
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_table_info = core_paths::sql_table_info();
    let no_constraint = core_paths::no_constraint();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_cons = core_paths::type_set_cons();
    let type_set_nil = core_paths::type_set_nil();
    let table_ref = core_paths::table_ref();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();

    // Generate compile-time SQL using concatcp! for FK tables, literal for non-FK
    let schema_sql_const = generate_schema_sql_const(ctx);

    // Column names for TableRef
    let column_names: Vec<&String> = ctx.field_infos.iter().map(|f| &f.column_name).collect();

    let to_sql_body = quote! {
        #sql::table(#table_ref::sql(Self::TABLE_NAME, &[#(#column_names),*]))
    };

    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        &quote! {#table_name},
        &quote! {
            {
                #sqlite_schema_type::Table(&<#struct_ident as drizzle::core::DrizzleTable>::TABLE_REF)
            }
        },
        &quote! {#schema_sql_const},
    );
    let dialect_types = crate::common::constraints::DialectTypes {
        sql_schema: core_paths::sql_schema(),
        schema_type: sqlite_paths::sqlite_schema_type(),
        value_type: sqlite_paths::sqlite_value(),
    };
    let (foreign_key_impls, _sql_foreign_keys, foreign_keys_type, fk_constraint_idents) =
        crate::common::constraints::generate_foreign_keys(
            ctx.field_infos,
            &ctx.attrs.composite_foreign_keys,
            struct_ident,
            ctx.struct_vis,
        );
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

    let alias_type_ident = format_ident!("{}Alias", struct_ident);
    let non_empty_marker = core_paths::non_empty_marker();
    let sql_table_impl = generate_sql_table(SQLTableConfig {
        struct_ident,
        select: quote! {#select_model},
        insert: quote! {#insert_model<'a, T>},
        update: quote! {#update_model<'a, #non_empty_marker>},
        aliased: quote! {#alias_type_ident},
        foreign_keys: quote! {#foreign_keys_type},
        primary_key: quote! {#primary_key_type},
        constraints: quote! { #constraints_type },
    });

    let mut dependencies = Vec::new();
    let mut seen_dependencies = HashSet::new();
    for field in ctx.field_infos {
        if let Some(fk) = &field.foreign_key {
            let name = fk.table_ident.to_string();
            if seen_dependencies.insert(name) {
                dependencies.push(fk.table_ident.clone());
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
    // Build TABLE_REF const
    let column_dialect = core_paths::column_dialect();
    let table_dialect = core_paths::table_dialect();
    let table_ref_name_expr = quote! {
        <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME
    };
    let table_ref_qualified_name_expr = quote! {
        <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME
    };
    let table_ref_schema_expr = quote! { ::core::option::Option::None };
    let table_ref_columns: Vec<ColumnRefInput> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let autoincrement = f.is_autoincrement;
            let default = sqlite_default_sql(f).map_or_else(
                || quote! { ::core::option::Option::None },
                |default| quote! { ::core::option::Option::Some(#default) },
            );
            let generated_expression = f.generated_column.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |generated| {
                    let expression = parenthesized_sql_expression(&generated.expression);
                    quote! { ::core::option::Option::Some(#expression) }
                },
            );
            let generated_stored = f
                .generated_column
                .as_ref()
                .is_some_and(|generated| generated.stored);
            let collate = f.collate.as_ref().map_or_else(
                || quote! { ::core::option::Option::None },
                |collate| quote! { ::core::option::Option::Some(#collate) },
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
                    #column_dialect::SQLite {
                        autoincrement: #autoincrement,
                        default: #default,
                        generated_expression: #generated_expression,
                        generated_stored: #generated_stored,
                        collate: #collate,
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
                let target_table = &fk.table_ident;
                let target_table_name = fk.table_ident.to_string().to_snake_case();
                let target_column = fk.column_ident.to_string().to_snake_case();
                ForeignKeyRefInput {
                    name: format!(
                        "fk_{}_{}_{}_{}_fk",
                        ctx.table_name, f.column_name, target_table_name, target_column
                    ),
                    name_explicit: false,
                    source_columns: vec![f.column_name.clone()],
                    target_schema: quote! { "" },
                    target_table: quote! { <#target_table as drizzle::core::DrizzleTable>::NAME },
                    target_columns: vec![target_column],
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
            .map(std::string::ToString::to_string)
            .collect();
        let target_table_name = cfk.target_table.to_string().to_snake_case();
        let target_columns: Vec<String> = cfk
            .target_columns
            .iter()
            .map(|col| col.to_string().to_snake_case())
            .collect();
        let fk_name = format!(
            "fk_{}_{}_{}_{}_fk",
            ctx.table_name,
            source_columns.join("_"),
            target_table_name,
            target_columns.join("_")
        );
        table_ref_fks.push(ForeignKeyRefInput {
            name: fk_name,
            name_explicit: false,
            source_columns,
            target_schema: quote! { "" },
            target_table: quote! { <#target_table as drizzle::core::DrizzleTable>::NAME },
            target_columns,
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
    for unique in &ctx.attrs.unique_constraints {
        let columns: Vec<String> = unique
            .columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let name = unique
            .name
            .clone()
            .unwrap_or_else(|| format!("{}_{}_unique", ctx.table_name, columns.join("_")));
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
    let table_ref_dialect =
        quote! { #table_dialect::SQLite { without_rowid: #without_rowid, strict: #strict } };
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
            <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME
        },
        qualified_name: quote! {
            <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME
        },
        schema: quote! { ::std::option::Option::None },
        dependency_names: quote! { &[#(#dependency_name_exprs),*] },
        table_ref_const,
    });
    let sqlite_table_impl =
        generate_sqlite_table(struct_ident, &quote! {#without_rowid}, &quote! {#strict});
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
    let capability_impls = crate::common::constraints::generate_constraint_capabilities(
        ctx.field_infos,
        ctx.struct_ident,
        !ctx.attrs.composite_foreign_keys.is_empty(),
        has_check,
        &dialect_types,
    );
    let table_unique_capability_impls = generate_table_unique_capability_impls(ctx);

    let has_select_model = core_paths::has_select_model();
    let into_select_target = core_paths::into_select_target();
    let select_star = core_paths::select_star();

    Ok(quote! {
        #foreign_key_impls
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
        impl #has_select_model for #struct_ident {
            type SelectModel = #select_model;
            const COLUMN_COUNT: usize = #columns_len;
        }
        impl #into_select_target for #struct_ident {
            type Marker = #select_star;
        }
        #sqlite_table_impl
        #to_sql_impl
        #relations_impl
        #capability_impls
        #table_unique_capability_impls
    })
}

fn sqlite_default_sql(field: &crate::sqlite::field::FieldInfo<'_>) -> Option<String> {
    if let Some(default_sql) = &field.default_sql {
        return Some(default_sql.clone());
    }

    let syn::Expr::Lit(expr_lit) = field.default_value.as_ref()? else {
        return None;
    };

    match &expr_lit.lit {
        syn::Lit::Int(i) => Some(i.to_string()),
        syn::Lit::Float(f) => Some(f.to_string()),
        syn::Lit::Bool(b) => Some(if b.value() { "1" } else { "0" }.to_string()),
        syn::Lit::Str(s) => Some(format!("'{}'", s.value().replace('\'', "''"))),
        _ => None,
    }
}

fn parenthesized_sql_expression(expression: &str) -> String {
    let expression = expression.trim();
    if expression.starts_with('(') && expression.ends_with(')') {
        expression.to_string()
    } else {
        format!("({expression})")
    }
}

fn table_unique_column_data(
    ctx: &MacroContext,
    unique: &crate::sqlite::table::attributes::UniqueConstraintAttr,
) -> (Vec<Ident>, Vec<String>, Vec<TokenStream>) {
    let col_zsts = unique
        .columns
        .iter()
        .map(|src| {
            let pascal = src.to_string().to_upper_camel_case();
            format_ident!("{}{}", ctx.struct_ident, pascal)
        })
        .collect::<Vec<_>>();
    let col_names = unique
        .columns
        .iter()
        .map(|src| {
            ctx.field_infos
                .iter()
                .find(|field| field.ident == src)
                .map_or_else(|| src.to_string(), |field| field.column_name.clone())
        })
        .collect::<Vec<_>>();
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

fn generate_table_unique_capability_impls(ctx: &MacroContext) -> TokenStream {
    if ctx.attrs.unique_constraints.is_empty() {
        return TokenStream::new();
    }

    let struct_ident = ctx.struct_ident;
    let has_constraint = core_paths::has_constraint();
    let unique_kind = core_paths::unique_kind();
    let conflict_target = core_paths::conflict_target();
    let named_constraint = core_paths::named_constraint();
    let has_field_unique = ctx
        .field_infos
        .iter()
        .any(|f| f.is_unique() && !f.is_primary());

    let mut tokens = TokenStream::new();
    if !has_field_unique {
        tokens.extend(quote! {
            impl #has_constraint<#unique_kind> for #struct_ident {}
        });
    }

    for (idx, unique) in ctx.attrs.unique_constraints.iter().enumerate() {
        let uq_ident = format_ident!("__UniqueComposite_{}_{}", struct_ident, idx);
        let (col_zsts, col_names, _) = table_unique_column_data(ctx, unique);
        let constraint_name = unique
            .name
            .clone()
            .unwrap_or_else(|| format!("{}_{}_unique", ctx.table_name, col_names.join("_")));

        tokens.extend(quote! {
            impl #conflict_target<#struct_ident> for #uq_ident {
                fn conflict_columns(&self) -> &'static [&'static str] { &[#(#col_names),*] }
            }
            impl #conflict_target<#struct_ident> for (#(#col_zsts,)*) {
                fn conflict_columns(&self) -> &'static [&'static str] { &[#(#col_names),*] }
            }
            impl #named_constraint<#struct_ident> for #uq_ident {
                fn constraint_name(&self) -> &'static str { #constraint_name }
            }
        });
    }

    tokens
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
