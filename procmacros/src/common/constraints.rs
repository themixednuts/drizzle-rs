//! Shared constraint generation code for SQLite and PostgreSQL table macros.
//!
//! These functions generate primary key, unique, foreign key, constraint capability,
//! and relation impls. They are generic over `ConstraintFieldInfo` and `ForeignKeyRef`
//! traits that each dialect implements for its own `FieldInfo` / FK reference types.

use crate::paths::core as core_paths;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use std::collections::HashMap;
use syn::Result;

// =============================================================================
// Trait abstractions
// =============================================================================

/// Minimal field interface for shared constraint generation.
pub(crate) trait ConstraintFieldInfo {
    type ForeignKey: ForeignKeyRef;

    fn ident(&self) -> &Ident;
    fn column_name(&self) -> &str;
    fn is_primary(&self) -> bool;
    fn is_unique(&self) -> bool;
    fn foreign_key(&self) -> Option<&Self::ForeignKey>;
}

/// Foreign key reference abstraction over dialect-specific FK types.
pub(crate) trait ForeignKeyRef {
    fn ref_table(&self) -> &Ident;
    fn ref_column(&self) -> &Ident;
}

/// Composite foreign key abstraction.
pub(crate) trait CompositeForeignKeyRef {
    fn target_table(&self) -> &Ident;
    fn source_columns(&self) -> &[Ident];
    fn target_columns(&self) -> &[Ident];
}

// =============================================================================
// Shared constraint generation functions
// =============================================================================

pub(crate) fn generate_primary_key<F: ConstraintFieldInfo>(
    field_infos: &[F],
    table_name: &str,
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

    let pk_fields: Vec<_> = field_infos
        .iter()
        .filter(|field| field.is_primary())
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
        .map(|field| field.column_name().to_owned())
        .collect();
    let pk_col_zst_idents: Vec<Ident> = pk_fields
        .iter()
        .map(|field| {
            let pascal = field.ident().to_string().to_upper_camel_case();
            format_ident!("{}{}", struct_ident, pascal)
        })
        .collect();
    let pk_len = pk_column_names.len();
    let pk_name = format!("{}_pk", table_name);
    let pk_col_tuple = quote! { (#(#pk_col_zst_idents,)*) };

    let column_not_null = core_paths::column_not_null();
    let pk_not_null_asserts: Vec<TokenStream> = pk_fields
        .iter()
        .map(|field| {
            let field_span = field.ident().span();
            let pascal = field.ident().to_string().to_upper_camel_case();
            let col_zst = format_ident!("{}{}", struct_ident, pascal);
            quote_spanned! {field_span=>
                const _: () = {
                    const fn assert_pk_not_null()
                    where #col_zst: #column_not_null,
                    { }
                    assert_pk_not_null();
                };
            }
        })
        .collect();

    let pk_impl = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #struct_vis struct #pk_zst_ident;

        #(#pk_not_null_asserts)*

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

pub(crate) fn generate_unique_constraints<F: ConstraintFieldInfo>(
    field_infos: &[F],
    table_name: &str,
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

    for field in field_infos
        .iter()
        .filter(|f| f.is_unique() && !f.is_primary())
    {
        let field_pascal = field.ident().to_string().to_upper_camel_case();
        let uq_ident = format_ident!("__Unique_{}_{}", struct_ident, field_pascal);
        let col_ident = format_ident!("{}{}", struct_ident, field_pascal);
        let constraint_name = format!("{}_{}_unique", table_name, field.column_name());
        let col_name = field.column_name().to_owned();

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

pub(crate) fn generate_foreign_keys<F: ConstraintFieldInfo, C: CompositeForeignKeyRef>(
    field_infos: &[F],
    composite_fks: &[C],
    table_name: &str,
    struct_ident: &Ident,
    struct_vis: &syn::Visibility,
    sql_table_info: &TokenStream,
    sql_column_info: &TokenStream,
) -> Result<(TokenStream, TokenStream, TokenStream, Vec<Ident>)> {
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

    for (idx, field) in field_infos.iter().enumerate() {
        let Some(fk) = field.foreign_key() else {
            continue;
        };

        let source_col_pascal = field.ident().to_string().to_upper_camel_case();
        let fk_zst_ident = format_ident!("__Fk_{}_{}", struct_ident, source_col_pascal);
        let fk_static_name = format_ident!(
            "__FK_STATIC_{}_{}",
            struct_ident.to_string().to_ascii_uppercase(),
            idx
        );

        let source_column = field.column_name().to_owned();
        let ref_table_ident = fk.ref_table();
        let source_col_zst_ident = format_ident!("{}{}", struct_ident, source_col_pascal);
        let ref_column_ident = fk.ref_column();
        let ref_column_pascal = ref_column_ident.to_string().to_upper_camel_case();
        let ref_column_zst_ident = format_ident!("{}{}", ref_table_ident, ref_column_pascal);

        let constraint_name = format!("{}_{}_fk", table_name, source_column);

        let field_span = field.ident().span();
        let type_match_assert = quote_spanned! {field_span=>
            const _: () = {
                const fn assert_fk_types()
                where
                    (): #fk_type_match<(#source_col_zst_ident,), (#ref_column_zst_ident,)>,
                {
                }
                assert_fk_types();
            };
        };

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
                        + #fk_arity_match<(#source_col_zst_ident,), (#ref_column_zst_ident,)>,
                    __ValidateFk: #no_duplicate_col_set<(#source_col_zst_ident,)>
                        + #no_duplicate_col_set<(#ref_column_zst_ident,)>,
                {
                }
                assert_fk();
            };

            #type_match_assert

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

    for (idx, fk) in composite_fks.iter().enumerate() {
        let fk_zst_ident = format_ident!("__FkComposite_{}_{}", struct_ident, idx);
        let fk_static_name = format_ident!(
            "__FK_COMPOSITE_STATIC_{}_{}",
            struct_ident.to_string().to_ascii_uppercase(),
            idx
        );

        let ref_table_ident = fk.target_table();
        let source_column_names: Vec<String> = fk
            .source_columns()
            .iter()
            .map(|src| {
                field_infos
                    .iter()
                    .find(|f| f.ident() == src)
                    .map(|f| f.column_name().to_owned())
                    .ok_or_else(|| {
                        syn::Error::new(
                            src.span(),
                            format!(
                                "composite foreign key references field `{}` which does not exist on `{}`",
                                src, struct_ident
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        let source_col_zst_idents: Vec<Ident> = fk
            .source_columns()
            .iter()
            .map(|src| {
                let pascal = src.to_string().to_upper_camel_case();
                format_ident!("{}{}", struct_ident, pascal)
            })
            .collect();
        let target_col_zst_idents: Vec<Ident> = fk
            .target_columns()
            .iter()
            .map(|target_col| {
                let pascal = target_col.to_string().to_upper_camel_case();
                format_ident!("{}{}", ref_table_ident, pascal)
            })
            .collect();

        let source_checks = fk.source_columns().iter().map(|src| {
            quote! {
                const _: () = { let _ = &#struct_ident::#src; };
            }
        });
        let target_checks = fk.target_columns().iter().map(|target_col| {
            quote! {
                const _: () = { let _ = &#ref_table_ident::#target_col; };
            }
        });

        let source_len = source_column_names.len();
        let target_len = target_col_zst_idents.len();

        let constraint_name = format!("{}_composite_fk_{}", table_name, idx);
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

    Ok((quote! { #(#fk_impls)* }, fk_list, fk_types, fk_zst_idents))
}

pub(crate) fn generate_constraint_capabilities<F: ConstraintFieldInfo>(
    field_infos: &[F],
    table_name: &str,
    struct_ident: &Ident,
    has_composite_fks: bool,
    has_check_constraints: bool,
) -> TokenStream {
    let has_primary_key = core_paths::has_primary_key();
    let has_constraint = core_paths::has_constraint();
    let primary_key_kind = core_paths::primary_key_kind();
    let foreign_key_kind = core_paths::foreign_key_kind();
    let unique_kind = core_paths::unique_kind();
    let conflict_target = core_paths::conflict_target();
    let named_constraint = core_paths::named_constraint();

    let pk_fields: Vec<_> = field_infos.iter().filter(|f| f.is_primary()).collect();
    let has_pk = !pk_fields.is_empty();
    let has_fk = field_infos.iter().any(|f| f.foreign_key().is_some()) || has_composite_fks;
    let has_unique = field_infos.iter().any(|f| f.is_unique() && !f.is_primary());

    let mut tokens = TokenStream::new();

    if has_pk {
        tokens.extend(quote! {
            impl #has_primary_key for #struct_ident {}
            impl #has_constraint<#primary_key_kind> for #struct_ident {}
        });

        for field in &pk_fields {
            let col_pascal = field.ident().to_string().to_upper_camel_case();
            let col_zst = format_ident!("{}{}", struct_ident, col_pascal);
            let col_name = field.column_name();
            tokens.extend(quote! {
                impl #conflict_target<#struct_ident> for #col_zst {
                    fn conflict_columns(&self) -> &'static [&'static str] { &[#col_name] }
                }
            });
        }

        let pk_zst = format_ident!("__Pk_{}", struct_ident);
        let pk_col_names: Vec<&str> = pk_fields.iter().map(|f| f.column_name()).collect();
        tokens.extend(quote! {
            impl #conflict_target<#struct_ident> for #pk_zst {
                fn conflict_columns(&self) -> &'static [&'static str] { &[#(#pk_col_names),*] }
            }
        });

        if pk_fields.len() > 1 {
            let pk_col_zsts: Vec<Ident> = pk_fields
                .iter()
                .map(|f| {
                    let pascal = f.ident().to_string().to_upper_camel_case();
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

    if has_check_constraints {
        let check_kind = core_paths::check_kind();
        tokens.extend(quote! {
            impl #has_constraint<#check_kind> for #struct_ident {}
        });
    }

    for field in field_infos
        .iter()
        .filter(|f| f.is_unique() && !f.is_primary())
    {
        let col_pascal = field.ident().to_string().to_upper_camel_case();
        let col_zst = format_ident!("{}{}", struct_ident, col_pascal);
        let uq_zst = format_ident!("__Unique_{}_{}", struct_ident, col_pascal);
        let col_name = field.column_name();
        let constraint_name = format!("{}_{}_unique", table_name, field.column_name());

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

pub(crate) fn generate_relations<F: ConstraintFieldInfo, C: CompositeForeignKeyRef>(
    field_infos: &[F],
    composite_fks: &[C],
    struct_ident: &Ident,
) -> Result<TokenStream> {
    let relation_marker = core_paths::relation_marker();
    let joinable_marker = core_paths::joinable_marker();

    type FkTargetMap = HashMap<String, (Ident, Vec<(Vec<String>, Vec<String>)>)>;
    let mut target_map: FkTargetMap = HashMap::new();

    for field in field_infos {
        let Some(fk) = field.foreign_key() else {
            continue;
        };
        let ref_table_ident = fk.ref_table();
        let ref_table_name = ref_table_ident.to_string();

        let source_col = field.column_name().to_owned();
        let target_col = fk.ref_column().to_string();

        target_map
            .entry(ref_table_name)
            .or_insert_with(|| (ref_table_ident.clone(), Vec::new()))
            .1
            .push((vec![source_col], vec![target_col]));
    }

    for comp_fk in composite_fks {
        let ref_table_ident = comp_fk.target_table();
        let ref_table_name = ref_table_ident.to_string();

        let source_cols: Vec<String> = comp_fk
            .source_columns()
            .iter()
            .map(|src| {
                field_infos
                    .iter()
                    .find(|f| f.ident() == src)
                    .map(|f| f.column_name().to_owned())
                    .ok_or_else(|| {
                        syn::Error::new(
                            src.span(),
                            format!(
                                "composite foreign key references field `{}` which does not exist on `{}`",
                                src, struct_ident
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let target_cols: Vec<String> = comp_fk
            .target_columns()
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

            if struct_ident != target_ident {
                tokens.extend(quote! {
                    impl #relation_marker<#struct_ident> for #target_ident {}
                    impl #joinable_marker<#struct_ident> for #target_ident {
                        fn fk_columns() -> &'static [(&'static str, &'static str)] {
                            &[#((#tgt_cols, #src_cols)),*]
                        }
                    }
                });
            }
        }
    }

    Ok(tokens)
}
