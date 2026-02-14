use super::context::MacroContext;
use crate::generators::generate_sql_table_info;
use crate::paths::core as core_paths;
#[allow(unused_imports)]
use crate::paths::sqlite as sqlite_paths;
use crate::sqlite::generators::*;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use syn::Result;

/// Generates the `SQLSchema` and `SQLTable` implementations.
pub(crate) fn generate_table_impls(
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
    let sql_column_info = core_paths::sql_column_info();
    let sql_table_info = core_paths::sql_table_info();
    let sql_static_table_info = core_paths::sql_static_table_info();
    let no_constraint = core_paths::no_constraint();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_cons = core_paths::type_set_cons();
    let type_set_nil = core_paths::type_set_nil();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_column_info = sqlite_paths::sqlite_column_info();
    let sqlite_table_info = sqlite_paths::sqlite_table_info();

    // Generate SQL implementation based on whether table has foreign keys
    let create_table_sql = &ctx.create_table_sql;

    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        // Call create_table_sql() which includes FK constraints via the DDL definitions
        (
            quote! { "" }, // Empty const, use runtime method
            Some(quote! {
                #sql::raw(Self::create_table_sql())
            }),
        )
    } else {
        // Use static SQL for tables without foreign keys
        (quote! { #create_table_sql }, None)
    };

    let to_sql_body = quote! {
        static INSTANCE: #struct_ident = #struct_ident::new();
        #sql::table(&INSTANCE)
    };

    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        quote! {#table_name},
        quote! {
            {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                #sqlite_schema_type::Table(&TABLE_INSTANCE)
            }
        },
        quote! {#sql_const},
        sql_method,
    );
    let (foreign_key_impls, sql_foreign_keys, foreign_keys_type, fk_constraint_idents) =
        generate_foreign_keys(
            ctx,
            struct_ident,
            ctx.struct_vis,
            &sql_table_info,
            &sql_column_info,
        );
    let (primary_key_impls, sql_primary_key, primary_key_type, pk_constraint_ident) =
        generate_primary_key(ctx, struct_ident, ctx.struct_vis, &sql_table_info);
    let (unique_constraint_impls, unique_constraint_idents) =
        generate_unique_constraints(ctx, struct_ident, ctx.struct_vis, &sql_table_info);

    let sql_constraint_info = core_paths::sql_constraint_info();
    let mut constraint_idents = Vec::new();
    if let Some(pk_ident) = pk_constraint_ident {
        constraint_idents.push(pk_ident);
    }
    constraint_idents.extend(fk_constraint_idents);
    constraint_idents.extend(unique_constraint_idents);

    let constraints_type = if constraint_idents.is_empty() {
        quote! { #no_constraint }
    } else {
        quote! { (#(#constraint_idents,)*) }
    };
    let constraint_len = constraint_idents.len();
    let constraint_static_names: Vec<Ident> = constraint_idents
        .iter()
        .enumerate()
        .map(|(idx, _)| format_ident!("__CONSTRAINT_STATIC_{}_{}", struct_ident, idx))
        .collect();
    let sql_constraints = if constraint_idents.is_empty() {
        quote! { &[] }
    } else {
        quote! {
            #(#[allow(non_upper_case_globals)] static #constraint_static_names: #constraint_idents = #constraint_idents;)*
            #[allow(non_upper_case_globals)]
            static CONSTRAINTS: [&'static dyn #sql_constraint_info; #constraint_len] =
                [#(&#constraint_static_names,)*];
            &CONSTRAINTS
        }
    };

    let aliased_table_ident = format_ident!("Aliased{}", struct_ident);
    let sql_table_impl = generate_sql_table(
        struct_ident,
        quote! {#select_model},
        quote! {#insert_model<'a, T>},
        quote! {#update_model<'a>},
        quote! {#aliased_table_ident},
        quote! {#foreign_keys_type},
        quote! {#primary_key_type},
        quote! { #constraints_type },
    );

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
    let dependencies_len = dependencies.len();
    let dependency_statics: Vec<_> = dependencies
        .iter()
        .enumerate()
        .map(|(idx, ident)| format_ident!("__DRIZZLE_DEP_{}_{}", idx, ident))
        .collect();
    let sql_dependencies = quote! {
        #(#[allow(non_upper_case_globals)] static #dependency_statics: #dependencies = #dependencies::new(); )*
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sql_table_info; #dependencies_len] =
            [#(&#dependency_statics,)*];
        &DEPENDENCIES
    };
    let sqlite_dependencies = quote! {
        #(#[allow(non_upper_case_globals)] static #dependency_statics: #dependencies = #dependencies::new(); )*
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sqlite_table_info; #dependencies_len] =
            [#(&#dependency_statics,)*];
        &DEPENDENCIES
    };
    let sql_table_info_impl = generate_sql_table_info(
        struct_ident,
        quote! {
            <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME
        },
        quote! { ::std::option::Option::None },
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static COLUMNS: [&'static dyn #sql_column_info; #columns_len] =
                [#(&#column_zst_idents,)*];
            &COLUMNS
        },
        quote! {
            #sql_primary_key
        },
        quote! {
            #sql_foreign_keys
        },
        quote! {
            #sql_constraints
        },
        sql_dependencies,
    );
    let sqlite_table_info_impl = generate_sqlite_table_info(
        struct_ident,
        quote! {
            &<Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE
        },
        quote! {#strict},
        quote! {#without_rowid},
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static SQLITE_COLUMNS: [&'static dyn #sqlite_column_info; #columns_len] =
                [#(&#column_zst_idents,)*];
            &SQLITE_COLUMNS
        },
        sqlite_dependencies,
    );
    let sqlite_table_impl =
        generate_sqlite_table(struct_ident, quote! {#without_rowid}, quote! {#strict});
    let to_sql_impl = generate_to_sql(struct_ident, to_sql_body);

    // Generate compile-time relation marker impls
    let relations_impl = generate_relations(ctx)?;
    let capability_impls = generate_constraint_capabilities(ctx);

    Ok(quote! {
        #foreign_key_impls
        #primary_key_impls
        #unique_constraint_impls

        #sql_schema_impl
        #sql_table_impl
        #sql_table_info_impl
        impl #sql_static_table_info for #struct_ident {
            fn static_table() -> &'static Self {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                &TABLE_INSTANCE
            }
        }
        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_cons<#struct_ident, #type_set_nil>;
        }
        #sqlite_table_info_impl
        #sqlite_table_impl
        #to_sql_impl
        #relations_impl
        #capability_impls
    })
}

fn generate_foreign_keys(
    ctx: &MacroContext,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
    sql_table_info: &TokenStream,
    sql_column_info: &TokenStream,
) -> (TokenStream, TokenStream, TokenStream, Vec<Ident>) {
    let sql_foreign_key_info = core_paths::sql_foreign_key_info();
    let sql_foreign_key = core_paths::sql_foreign_key();
    let sql_constraint_info = core_paths::sql_constraint_info();
    let sql_constraint = core_paths::sql_constraint();
    let sql_constraint_kind = core_paths::sql_constraint_kind();
    let foreign_key_kind = core_paths::foreign_key_kind();
    let columns_belong_to = core_paths::columns_belong_to();
    let non_empty_col_set = core_paths::non_empty_col_set();
    let no_duplicate_col_set = core_paths::no_duplicate_col_set();
    let fk_arity_match = core_paths::fk_arity_match();
    let fk_type_match = core_paths::fk_type_match();

    let mut fk_impls = Vec::new();
    let mut fk_zst_idents = Vec::new();
    let mut fk_static_names = Vec::new();

    for (idx, field) in ctx.field_infos.iter().enumerate() {
        let Some(fk) = &field.foreign_key else {
            continue;
        };

        let source_col_pascal = field.ident.to_string().to_upper_camel_case();
        let fk_zst_ident = format_ident!("__Fk_{}_{}", struct_ident, source_col_pascal);
        let fk_static_name = format_ident!(
            "__FK_STATIC_{}_{}",
            struct_ident.to_string().to_ascii_uppercase(),
            idx
        );

        let source_column = field.column_name.clone();
        let ref_table_ident = &fk.table_ident;
        let source_col_zst_ident = format_ident!("{}{}", struct_ident, source_col_pascal);
        let ref_column_ident = &fk.column_ident;
        let ref_column_pascal = ref_column_ident.to_string().to_upper_camel_case();
        let ref_column_zst_ident = format_ident!("{}{}", ref_table_ident, ref_column_pascal);

        let constraint_name = format!("{}_{}_fk", ctx.table_name, source_column);

        fk_impls.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #struct_vis struct #fk_zst_ident;

            const _: () = {
                struct __ValidateFk;
                impl #no_duplicate_col_set<(#source_col_zst_ident,)> for __ValidateFk {}
                impl #no_duplicate_col_set<(#ref_column_zst_ident,)> for __ValidateFk {}

                const fn assert_fk()
                where
                    (): #non_empty_col_set<(#source_col_zst_ident,)>
                        + #non_empty_col_set<(#ref_column_zst_ident,)>
                        + #columns_belong_to<#struct_ident, (#source_col_zst_ident,)>
                        + #columns_belong_to<#ref_table_ident, (#ref_column_zst_ident,)>
                        + #fk_arity_match<(#source_col_zst_ident,), (#ref_column_zst_ident,)>
                        + #fk_type_match<(#source_col_zst_ident,), (#ref_column_zst_ident,)>,
                    __ValidateFk: #no_duplicate_col_set<(#source_col_zst_ident,)>
                        + #no_duplicate_col_set<(#ref_column_zst_ident,)>,
                {
                }
                assert_fk();
            };

            impl #sql_foreign_key_info for #fk_zst_ident {
                fn source_table(&self) -> &'static dyn #sql_table_info {
                    #[allow(non_upper_case_globals)]
                    static SOURCE_TABLE: #struct_ident = #struct_ident::new();
                    &SOURCE_TABLE
                }

                fn target_table(&self) -> &'static dyn #sql_table_info {
                    #[allow(non_upper_case_globals)]
                    static TARGET_TABLE: #ref_table_ident = #ref_table_ident::new();
                    &TARGET_TABLE
                }

                fn source_columns(&self) -> &'static [&'static str] {
                    &[#source_column]
                }

                fn target_columns(&self) -> &'static [&'static str] {
                    #[allow(non_upper_case_globals)]
                    static REF_COLUMN: #ref_column_zst_ident = #ref_column_zst_ident::new();
                    static REF_COLUMNS: ::std::sync::LazyLock<[&'static str; 1]> =
                        ::std::sync::LazyLock::new(|| [#sql_column_info::name(&REF_COLUMN)]);
                    &*REF_COLUMNS
                }
            }

            impl #sql_foreign_key for #fk_zst_ident {
                type SourceTable = #struct_ident;
                type TargetTable = #ref_table_ident;
                type SourceColumns = (#source_col_zst_ident,);
                type TargetColumns = (#ref_column_zst_ident,);
            }

            impl #sql_constraint_info for #fk_zst_ident {
                fn table(&self) -> &'static dyn #sql_table_info {
                    <Self as #sql_foreign_key_info>::source_table(self)
                }

                fn name(&self) -> Option<&'static str> {
                    Some(#constraint_name)
                }

                fn kind(&self) -> #sql_constraint_kind {
                    #sql_constraint_kind::ForeignKey
                }

                fn columns(&self) -> &'static [&'static str] {
                    <Self as #sql_foreign_key_info>::source_columns(self)
                }

                fn foreign_key(&self) -> Option<&'static dyn #sql_foreign_key_info> {
                    #[allow(non_upper_case_globals)]
                    static FK: #fk_zst_ident = #fk_zst_ident;
                    Some(&FK as &'static dyn #sql_foreign_key_info)
                }
            }

            impl #sql_constraint for #fk_zst_ident {
                type Table = #struct_ident;
                type Kind = #foreign_key_kind;
                type Columns = (#source_col_zst_ident,);
            }
        });

        fk_zst_idents.push(fk_zst_ident);
        fk_static_names.push(fk_static_name);
    }

    for (idx, fk) in ctx.attrs.composite_foreign_keys.iter().enumerate() {
        let fk_zst_ident = format_ident!("__FkComposite_{}_{}", struct_ident, idx);
        let fk_static_name = format_ident!(
            "__FK_COMPOSITE_STATIC_{}_{}",
            struct_ident.to_string().to_ascii_uppercase(),
            idx
        );

        let ref_table_ident = &fk.target_table;
        let source_column_names: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| f.ident == src)
                    .map(|f| f.column_name.clone())
                    .unwrap_or_else(|| src.to_string())
            })
            .collect();

        let source_col_zst_idents: Vec<Ident> = fk
            .source_columns
            .iter()
            .map(|src| {
                let pascal = src.to_string().to_upper_camel_case();
                format_ident!("{}{}", struct_ident, pascal)
            })
            .collect();
        let target_col_zst_idents: Vec<Ident> = fk
            .target_columns
            .iter()
            .map(|target_col| {
                let pascal = target_col.to_string().to_upper_camel_case();
                format_ident!("{}{}", ref_table_ident, pascal)
            })
            .collect();

        let source_checks = fk.source_columns.iter().map(|src| {
            quote! {
                const _: () = { let _ = &#struct_ident::#src; };
            }
        });
        let target_checks = fk.target_columns.iter().map(|target_col| {
            quote! {
                const _: () = { let _ = &#ref_table_ident::#target_col; };
            }
        });

        let source_len = source_column_names.len();
        let target_len = target_col_zst_idents.len();

        let constraint_name = format!("{}_composite_fk_{}", ctx.table_name, idx);
        let src_tuple = quote! { (#(#source_col_zst_idents,)*) };
        let dst_tuple = quote! { (#(#target_col_zst_idents,)*) };

        fk_impls.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #struct_vis struct #fk_zst_ident;

            const _: () = {
                struct __ValidateFk;
                impl #no_duplicate_col_set<#src_tuple> for __ValidateFk {}
                impl #no_duplicate_col_set<#dst_tuple> for __ValidateFk {}

                const fn assert_fk()
                where
                    (): #non_empty_col_set<#src_tuple>
                        + #non_empty_col_set<#dst_tuple>
                        + #columns_belong_to<#struct_ident, #src_tuple>
                        + #columns_belong_to<#ref_table_ident, #dst_tuple>
                        + #fk_arity_match<#src_tuple, #dst_tuple>
                        + #fk_type_match<#src_tuple, #dst_tuple>,
                    __ValidateFk: #no_duplicate_col_set<#src_tuple>
                        + #no_duplicate_col_set<#dst_tuple>,
                {
                }
                assert_fk();
            };

            #(#source_checks)*
            #(#target_checks)*

            impl #sql_foreign_key_info for #fk_zst_ident {
                fn source_table(&self) -> &'static dyn #sql_table_info {
                    #[allow(non_upper_case_globals)]
                    static SOURCE_TABLE: #struct_ident = #struct_ident::new();
                    &SOURCE_TABLE
                }

                fn target_table(&self) -> &'static dyn #sql_table_info {
                    #[allow(non_upper_case_globals)]
                    static TARGET_TABLE: #ref_table_ident = #ref_table_ident::new();
                    &TARGET_TABLE
                }

                fn source_columns(&self) -> &'static [&'static str] {
                    static SRC_COLUMNS: [&str; #source_len] = [#(#source_column_names),*];
                    &SRC_COLUMNS
                }

                fn target_columns(&self) -> &'static [&'static str] {
                    #(#[allow(non_upper_case_globals)] static #target_col_zst_idents: #target_col_zst_idents = #target_col_zst_idents::new();)*
                    #[allow(non_upper_case_globals)]
                    static REF_COLUMNS: ::std::sync::LazyLock<[&'static str; #target_len]> =
                        ::std::sync::LazyLock::new(|| [#(#sql_column_info::name(&#target_col_zst_idents)),*]);
                    &*REF_COLUMNS
                }
            }

            impl #sql_foreign_key for #fk_zst_ident {
                type SourceTable = #struct_ident;
                type TargetTable = #ref_table_ident;
                type SourceColumns = (#(#source_col_zst_idents,)*);
                type TargetColumns = (#(#target_col_zst_idents,)*);
            }

            impl #sql_constraint_info for #fk_zst_ident {
                fn table(&self) -> &'static dyn #sql_table_info {
                    <Self as #sql_foreign_key_info>::source_table(self)
                }

                fn name(&self) -> Option<&'static str> {
                    Some(#constraint_name)
                }

                fn kind(&self) -> #sql_constraint_kind {
                    #sql_constraint_kind::ForeignKey
                }

                fn columns(&self) -> &'static [&'static str] {
                    <Self as #sql_foreign_key_info>::source_columns(self)
                }

                fn foreign_key(&self) -> Option<&'static dyn #sql_foreign_key_info> {
                    #[allow(non_upper_case_globals)]
                    static FK: #fk_zst_ident = #fk_zst_ident;
                    Some(&FK as &'static dyn #sql_foreign_key_info)
                }
            }

            impl #sql_constraint for #fk_zst_ident {
                type Table = #struct_ident;
                type Kind = #foreign_key_kind;
                type Columns = (#(#source_col_zst_idents,)*);
            }
        });

        fk_zst_idents.push(fk_zst_ident);
        fk_static_names.push(fk_static_name);
    }

    let fk_len = fk_zst_idents.len();
    let fk_list = if fk_len == 0 {
        quote! { &[] }
    } else {
        quote! {
            #(#[allow(non_upper_case_globals)] static #fk_static_names: #fk_zst_idents = #fk_zst_idents;)*
            #[allow(non_upper_case_globals)]
            static FOREIGN_KEYS: [&'static dyn #sql_foreign_key_info; #fk_len] =
                [#(&#fk_static_names,)*];
            &FOREIGN_KEYS
        }
    };

    let fk_types = if fk_zst_idents.is_empty() {
        quote! { () }
    } else {
        quote! { (#(#fk_zst_idents,)*) }
    };

    (quote! { #(#fk_impls)* }, fk_list, fk_types, fk_zst_idents)
}

fn generate_primary_key(
    ctx: &MacroContext,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
    sql_table_info: &TokenStream,
) -> (TokenStream, TokenStream, TokenStream, Option<Ident>) {
    let sql_primary_key_info = core_paths::sql_primary_key_info();
    let sql_primary_key = core_paths::sql_primary_key();
    let sql_constraint_info = core_paths::sql_constraint_info();
    let sql_constraint = core_paths::sql_constraint();
    let sql_constraint_kind = core_paths::sql_constraint_kind();
    let primary_key_kind = core_paths::primary_key_kind();
    let columns_belong_to = core_paths::columns_belong_to();
    let non_empty_col_set = core_paths::non_empty_col_set();
    let no_duplicate_col_set = core_paths::no_duplicate_col_set();
    let pk_not_null = core_paths::pk_not_null();
    let no_primary_key = core_paths::no_primary_key();

    let pk_fields: Vec<_> = ctx
        .field_infos
        .iter()
        .filter(|field| field.is_primary)
        .collect();
    if pk_fields.is_empty() {
        return (
            TokenStream::new(),
            quote! { ::std::option::Option::None },
            quote! { #no_primary_key },
            None,
        );
    }

    let pk_zst_ident = format_ident!("__Pk_{}", struct_ident);
    let pk_static_name = format_ident!(
        "__PK_STATIC_{}",
        struct_ident.to_string().to_ascii_uppercase()
    );
    let pk_column_names: Vec<String> = pk_fields
        .iter()
        .map(|field| field.column_name.clone())
        .collect();
    let pk_col_zst_idents: Vec<Ident> = pk_fields
        .iter()
        .map(|field| {
            let pascal = field.ident.to_string().to_upper_camel_case();
            format_ident!("{}{}", struct_ident, pascal)
        })
        .collect();
    let pk_len = pk_column_names.len();
    let pk_name = format!("{}_pk", ctx.table_name);
    let pk_col_tuple = quote! { (#(#pk_col_zst_idents,)*) };

    let pk_impl = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #struct_vis struct #pk_zst_ident;

        const _: () = {
            struct __ValidatePk;
            impl #no_duplicate_col_set<#pk_col_tuple> for __ValidatePk {}

            const fn assert_pk()
            where
                (): #non_empty_col_set<#pk_col_tuple>
                    + #columns_belong_to<#struct_ident, #pk_col_tuple>
                    + #pk_not_null<#pk_col_tuple>,
                __ValidatePk: #no_duplicate_col_set<#pk_col_tuple>,
            {
            }
            assert_pk();
        };

        impl #sql_primary_key_info for #pk_zst_ident {
            fn table(&self) -> &'static dyn #sql_table_info {
                #[allow(non_upper_case_globals)]
                static TABLE: #struct_ident = #struct_ident::new();
                &TABLE
            }

            fn columns(&self) -> &'static [&'static str] {
                static PK_COLUMNS: [&str; #pk_len] = [#(#pk_column_names),*];
                &PK_COLUMNS
            }
        }

        impl #sql_primary_key for #pk_zst_ident {
            type Table = #struct_ident;
            type Columns = (#(#pk_col_zst_idents,)*);
        }

        impl #sql_constraint_info for #pk_zst_ident {
            fn table(&self) -> &'static dyn #sql_table_info {
                <Self as #sql_primary_key_info>::table(self)
            }

            fn name(&self) -> Option<&'static str> {
                Some(#pk_name)
            }

            fn kind(&self) -> #sql_constraint_kind {
                #sql_constraint_kind::PrimaryKey
            }

            fn columns(&self) -> &'static [&'static str] {
                <Self as #sql_primary_key_info>::columns(self)
            }

            fn primary_key(&self) -> Option<&'static dyn #sql_primary_key_info> {
                #[allow(non_upper_case_globals)]
                static PK: #pk_zst_ident = #pk_zst_ident;
                Some(&PK as &'static dyn #sql_primary_key_info)
            }
        }

        impl #sql_constraint for #pk_zst_ident {
            type Table = #struct_ident;
            type Kind = #primary_key_kind;
            type Columns = (#(#pk_col_zst_idents,)*);
        }
    };

    let pk_meta = quote! {
        #[allow(non_upper_case_globals)]
        static #pk_static_name: #pk_zst_ident = #pk_zst_ident;
        ::std::option::Option::Some(&#pk_static_name as &'static dyn #sql_primary_key_info)
    };

    (
        pk_impl,
        pk_meta,
        quote! { #pk_zst_ident },
        Some(pk_zst_ident),
    )
}

fn generate_unique_constraints(
    ctx: &MacroContext,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
    sql_table_info: &TokenStream,
) -> (TokenStream, Vec<Ident>) {
    let sql_constraint_info = core_paths::sql_constraint_info();
    let sql_constraint = core_paths::sql_constraint();
    let sql_constraint_kind = core_paths::sql_constraint_kind();
    let unique_kind = core_paths::unique_kind();
    let columns_belong_to = core_paths::columns_belong_to();
    let non_empty_col_set = core_paths::non_empty_col_set();
    let no_duplicate_col_set = core_paths::no_duplicate_col_set();

    let mut impls = Vec::new();
    let mut idents = Vec::new();

    for field in ctx
        .field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
    {
        let field_pascal = field.ident.to_string().to_upper_camel_case();
        let uq_ident = format_ident!("__Unique_{}_{}", struct_ident, field_pascal);
        let col_ident = format_ident!("{}{}", struct_ident, field_pascal);
        let constraint_name = format!("{}_{}_unique", ctx.table_name, field.column_name);
        let col_name = field.column_name.clone();

        impls.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #struct_vis struct #uq_ident;

            const _: () = {
                struct __ValidateUnique;
                impl #no_duplicate_col_set<(#col_ident,)> for __ValidateUnique {}

                const fn assert_unique()
                where
                    (): #non_empty_col_set<(#col_ident,)>
                        + #columns_belong_to<#struct_ident, (#col_ident,)>,
                    __ValidateUnique: #no_duplicate_col_set<(#col_ident,)>,
                {
                }
                assert_unique();
            };

            impl #sql_constraint_info for #uq_ident {
                fn table(&self) -> &'static dyn #sql_table_info {
                    #[allow(non_upper_case_globals)]
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }

                fn name(&self) -> Option<&'static str> {
                    Some(#constraint_name)
                }

                fn kind(&self) -> #sql_constraint_kind {
                    #sql_constraint_kind::Unique
                }

                fn columns(&self) -> &'static [&'static str] {
                    &[#col_name]
                }
            }

            impl #sql_constraint for #uq_ident {
                type Table = #struct_ident;
                type Kind = #unique_kind;
                type Columns = (#col_ident,);
            }
        });

        idents.push(uq_ident);
    }

    (quote! { #(#impls)* }, idents)
}

fn generate_constraint_capabilities(ctx: &MacroContext) -> TokenStream {
    let has_primary_key = core_paths::has_primary_key();
    let has_constraint = core_paths::has_constraint();
    let primary_key_kind = core_paths::primary_key_kind();
    let foreign_key_kind = core_paths::foreign_key_kind();
    let unique_kind = core_paths::unique_kind();
    let conflict_target = core_paths::conflict_target();
    let named_constraint = core_paths::named_constraint();

    let struct_ident = ctx.struct_ident;
    let pk_fields: Vec<_> = ctx.field_infos.iter().filter(|f| f.is_primary).collect();
    let has_pk = !pk_fields.is_empty();
    let has_fk = ctx.field_infos.iter().any(|f| f.foreign_key.is_some())
        || !ctx.attrs.composite_foreign_keys.is_empty();
    let has_unique = ctx.field_infos.iter().any(|f| f.is_unique && !f.is_primary);

    let mut tokens = TokenStream::new();

    if has_pk {
        tokens.extend(quote! {
            impl #has_primary_key for #struct_ident {}
            impl #has_constraint<#primary_key_kind> for #struct_ident {}
        });

        // ConflictTarget for each PK column ZST
        for field in &pk_fields {
            let col_pascal = field.ident.to_string().to_upper_camel_case();
            let col_zst = format_ident!("{}{}", struct_ident, col_pascal);
            let col_name = &field.column_name;
            tokens.extend(quote! {
                impl #conflict_target<#struct_ident> for #col_zst {
                    fn conflict_columns(&self) -> &'static [&'static str] { &[#col_name] }
                }
            });
        }

        // ConflictTarget for __Pk_ ZST (all PK column names)
        let pk_zst = format_ident!("__Pk_{}", struct_ident);
        let pk_col_names: Vec<&String> = pk_fields.iter().map(|f| &f.column_name).collect();
        tokens.extend(quote! {
            impl #conflict_target<#struct_ident> for #pk_zst {
                fn conflict_columns(&self) -> &'static [&'static str] { &[#(#pk_col_names),*] }
            }
        });

        // Composite PK tuple impl
        if pk_fields.len() > 1 {
            let pk_col_zsts: Vec<Ident> = pk_fields
                .iter()
                .map(|f| {
                    let pascal = f.ident.to_string().to_upper_camel_case();
                    format_ident!("{}{}", struct_ident, pascal)
                })
                .collect();
            tokens.extend(quote! {
                impl #conflict_target<#struct_ident> for (#(#pk_col_zsts,)*) {
                    fn conflict_columns(&self) -> &'static [&'static str] { &[#(#pk_col_names),*] }
                }
            });
        }
    }

    if has_fk {
        tokens.extend(quote! {
            impl #has_constraint<#foreign_key_kind> for #struct_ident {}
        });
    }

    if has_unique {
        tokens.extend(quote! {
            impl #has_constraint<#unique_kind> for #struct_ident {}
        });
    }

    // ConflictTarget + NamedConstraint for each unique (non-PK) column ZST and its __Unique_ ZST
    for field in ctx
        .field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
    {
        let col_pascal = field.ident.to_string().to_upper_camel_case();
        let col_zst = format_ident!("{}{}", struct_ident, col_pascal);
        let uq_zst = format_ident!("__Unique_{}_{}", struct_ident, col_pascal);
        let col_name = &field.column_name;
        let constraint_name = format!("{}_{}_unique", ctx.table_name, field.column_name);

        tokens.extend(quote! {
            impl #conflict_target<#struct_ident> for #col_zst {
                fn conflict_columns(&self) -> &'static [&'static str] { &[#col_name] }
            }
            impl #conflict_target<#struct_ident> for #uq_zst {
                fn conflict_columns(&self) -> &'static [&'static str] { &[#col_name] }
            }
            impl #named_constraint<#struct_ident> for #uq_zst {
                fn constraint_name(&self) -> &'static str { #constraint_name }
            }
        });
    }

    tokens
}

/// Generates `Relation` and `Joinable` impls from FK declarations.
fn generate_relations(ctx: &MacroContext) -> Result<TokenStream> {
    let relation_marker = core_paths::relation_marker();
    let joinable_marker = core_paths::joinable_marker();

    let struct_ident = ctx.struct_ident;

    // (target_ident, Vec<(source_cols, target_cols)>) per target table
    type FkTargetMap = HashMap<String, (Ident, Vec<(Vec<String>, Vec<String>)>)>;
    let mut target_map: FkTargetMap = HashMap::new();

    for field in ctx.field_infos {
        let Some(fk) = &field.foreign_key else {
            continue;
        };
        let ref_table_ident = &fk.table_ident;
        let ref_table_name = ref_table_ident.to_string();

        let source_col = field.column_name.clone();
        let target_col = fk.column_ident.to_string();

        target_map
            .entry(ref_table_name)
            .or_insert_with(|| (ref_table_ident.clone(), Vec::new()))
            .1
            .push((vec![source_col], vec![target_col]));
    }

    for comp_fk in &ctx.attrs.composite_foreign_keys {
        let ref_table_ident = &comp_fk.target_table;
        let ref_table_name = ref_table_ident.to_string();

        let source_cols: Vec<String> = comp_fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| *f.ident == *src)
                    .map(|f| f.column_name.clone())
                    .unwrap_or_else(|| src.to_string())
            })
            .collect();
        let target_cols: Vec<String> = comp_fk
            .target_columns
            .iter()
            .map(|t| t.to_string())
            .collect();

        target_map
            .entry(ref_table_name)
            .or_insert_with(|| (ref_table_ident.clone(), Vec::new()))
            .1
            .push((source_cols, target_cols));
    }

    if target_map.is_empty() {
        return Ok(TokenStream::new());
    }

    let mut tokens = TokenStream::new();

    for (target_ident, fk_relations) in target_map.values() {
        tokens.extend(quote! {
            impl #relation_marker<#target_ident> for #struct_ident {}
        });

        if fk_relations.len() == 1 {
            let (src_cols, tgt_cols) = &fk_relations[0];
            tokens.extend(quote! {
                impl #joinable_marker<#target_ident> for #struct_ident {
                    fn fk_columns() -> &'static [(&'static str, &'static str)] {
                        &[#((#src_cols, #tgt_cols)),*]
                    }
                }
            });
        }
    }

    Ok(tokens)
}
