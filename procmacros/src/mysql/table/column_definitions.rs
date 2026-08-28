use super::context::MacroContext;
use crate::common::{generate_arithmetic_ops, generate_expr_impl, rust_type_to_nullability};
use crate::generators::{generate_impl, generate_sql_column_info};
use crate::mysql::field::FieldInfo;
use crate::mysql::generators::{
    generate_mysql_column, generate_sql_column, generate_sql_schema_field, generate_to_sql,
};
use crate::paths::{core as core_paths, mysql as mysql_paths};
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Result;

/// Generate a const that references the original marker tokens from the attribute.
///
/// This creates a hidden const that uses the exact tokens from `#[column(primary, unique)]`,
/// enabling rust-analyzer to resolve them and provide hover documentation.
fn generate_marker_const(info: &FieldInfo, _zst_ident: &Ident) -> TokenStream {
    if info.marker_exprs.is_empty() {
        return TokenStream::new();
    }

    let field_name = info.ident.to_string().to_uppercase();
    let marker_const_name = format_ident!("_ATTR_MARKERS_{}", field_name);
    let markers = &info.marker_exprs;

    quote! {
        /// Hidden const that references the original attribute markers.
        /// This enables IDE hover documentation for `#[column(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: () = {
            #( let _ = #markers; )*
        };
    }
}

pub(super) fn generate_custom_comparison_operand_impls(
    info: &FieldInfo,
    zst_ident: &Ident,
    mysql_value: &TokenStream,
) -> TokenStream {
    if !info.is_custom_type {
        return TokenStream::new();
    }

    let value_type = &info.base_type;
    let drizzle_mysql_column = mysql_paths::drizzle_mysql_column();

    quote! {
        impl<'a> drizzle::core::expr::ComparisonOperand<'a, #mysql_value<'a>, #zst_ident>
            for #value_type
        {
            type SQLType = <#value_type as #drizzle_mysql_column>::SQLType;
            type Aggregate = drizzle::core::expr::Scalar;

            fn into_comparison_sql(self) -> drizzle::core::SQL<'a, #mysql_value<'a>> {
                let value: #mysql_value<'a> =
                    <#value_type as #drizzle_mysql_column>::encode_owned(self).into();
                drizzle::core::SQL::param(value)
            }
        }

        impl<'a, 'value>
            drizzle::core::expr::ComparisonOperand<'a, #mysql_value<'a>, #zst_ident>
            for &'value #value_type
        {
            type SQLType = <#value_type as #drizzle_mysql_column>::SQLType;
            type Aggregate = drizzle::core::expr::Scalar;

            fn into_comparison_sql(self) -> drizzle::core::SQL<'a, #mysql_value<'a>> {
                let value: #mysql_value<'a> =
                    <#value_type as #drizzle_mysql_column>::encode(self)
                        .into_owned()
                        .into();
                drizzle::core::SQL::param(value)
            }
        }
    }
}

/// Generates the column ZSTs and their `SQLColumn` implementations.
pub fn generate_column_definitions(ctx: &MacroContext<'_>) -> Result<(TokenStream, Vec<Ident>)> {
    let mut all_column_code = TokenStream::new();
    let mut column_zst_idents = Vec::new();
    let MacroContext {
        struct_ident,
        struct_vis,
        field_infos,
        ..
    } = *ctx;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_column = core_paths::sql_column();
    let no_foreign_key = core_paths::no_foreign_key();
    let column_of = core_paths::column_of();
    let column_not_null = core_paths::column_not_null();
    let column_value_type = core_paths::column_value_type();
    let expr_value_type = core_paths::expr_value_type();
    let into_select_target = core_paths::into_select_target();
    let select_cols = core_paths::select_cols();
    let column_ref = core_paths::column_ref();
    let _mysql_column = mysql_paths::mysql_column();
    let mysql_value = mysql_paths::mysql_value();
    let mysql_schema_type = mysql_paths::mysql_schema_type();

    for info in field_infos {
        let field_pascal_case = info.ident.to_string().to_upper_camel_case();
        let zst_ident = format_ident!("{}{}", ctx.struct_ident, field_pascal_case);
        column_zst_idents.push(zst_ident.clone());

        let (value_type, rust_type) = (&info.base_type, &info.field_type);
        let (_is_primary, _is_not_null, _is_unique, _is_auto_increment, has_default) = (
            info.is_primary(),
            !info.is_nullable,
            info.is_unique(),
            info.is_auto_increment,
            info.has_default || info.default_fn.is_some(),
        );

        // Only DEFAULT_FN generates an application-side value.
        let default_const = quote! { ::std::option::Option::None };

        let default_fn_body = info.default_fn.as_ref().map_or_else(
            || quote! { ::std::option::Option::None::<fn() -> Self::Type> },
            |func| quote! { ::std::option::Option::Some(#func) },
        );

        let sql_def = info.sql_definition_expr();

        let name = &info.column_name;
        let ddl_name = super::ddl::quoted(name);
        let col_type = info.sql_type_expr();

        // Generate foreign key reference implementation (kept for FK const validation)
        let _foreign_key_impl = info.foreign_key.as_ref().map_or_else(
            || quote! { ::std::option::Option::None },
            |fk| {
                let table_ident = &fk.table;
                let column_ident = &fk.column;
                let column_pascal_case = column_ident.to_string().to_upper_camel_case();
                let fk_zst_ident = format_ident!("{}{}", table_ident, column_pascal_case);
                quote! {
                    // Const validation that the FK column exists and implements SQLColumnInfo
                    const _: () = { let _ = &#table_ident::#column_ident; };
                    #[allow(non_upper_case_globals)]
                    static FK_COLUMN: #fk_zst_ident = #fk_zst_ident::new();
                    ::std::option::Option::Some(&FK_COLUMN)
                }
            },
        );

        // Generate individual trait implementations using generators
        let struct_def = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #zst_ident;
        };

        let impl_new = generate_impl(
            &zst_ident,
            &quote! {
                pub const fn new() -> #zst_ident {
                    #zst_ident
                }
            },
        );

        let to_sql_body = quote! {
            #sql::column(#column_ref::sql(
                <#struct_ident as drizzle::core::DrizzleTable>::NAME,
                #name,
            ))
        };

        // Use generators for trait implementations
        let sql_schema_field_impl =
            generate_sql_schema_field(&zst_ident, &quote! {#name}, &col_type, &sql_def);
        let sql_column_info_impl = generate_sql_column_info(
            &zst_ident,
            &quote! {
                <Self as #sql_schema<'_, &'static str, #mysql_value<'_>>>::NAME
            },
            &quote! {
                <Self as #sql_schema<'_, &'static str, #mysql_value<'_>>>::TYPE
            },
            &quote! {
                <Self as #sql_column<'_, #mysql_value<'_>>>::PRIMARY_KEY
            },
            &quote! {
                <Self as #sql_column<'_, #mysql_value<'_>>>::NOT_NULL
            },
            &quote! {
                <Self as #sql_column<'_, #mysql_value<'_>>>::UNIQUE
            },
            &quote! {
                #has_default
            },
            &quote! {
                static TABLE: #struct_ident = #struct_ident::new();
                &TABLE
            },
        );

        // Direct const expressions - no runtime builder types needed
        let is_primary = info.is_primary();
        let is_not_null = !info.is_nullable;
        let is_unique = info.is_unique();
        let is_auto_increment = info.is_auto_increment;

        // Compute SQL type and nullability markers for type-safe expressions
        let sql_type_marker = info.sql_type_marker();
        let sql_nullable_marker = rust_type_to_nullability(rust_type);

        let mut foreign_key_types = Vec::new();
        if info.foreign_key.is_some() {
            let field_pascal_case = info.ident.to_string().to_upper_camel_case();
            let fk_ident = format_ident!("__Fk_{}_{}", struct_ident, field_pascal_case);
            foreign_key_types.push(quote! { #fk_ident });
        }
        for (fk_idx, fk) in ctx.attrs.composite_foreign_keys.iter().enumerate() {
            if fk.source_columns.iter().any(|src| src == &info.ident) {
                let fk_ident = format_ident!("__FkComposite_{}_{}", struct_ident, fk_idx);
                foreign_key_types.push(quote! { #fk_ident });
            }
        }
        let foreign_keys_type = if foreign_key_types.is_empty() {
            quote! { (#no_foreign_key,) }
        } else {
            quote! { (#(#foreign_key_types,)*) }
        };

        let sql_column_impl = generate_sql_column(
            &zst_ident,
            &quote! {#struct_ident},
            &quote! {#mysql_schema_type},
            &foreign_keys_type,
            &quote! {#rust_type},
            &quote! { #is_primary },
            &quote! { #is_not_null || #is_primary },
            &quote! { #is_unique },
            &quote! {#default_const},
            &quote! {#default_fn_body},
        );

        // Generate Expr trait implementation for type-safe expressions
        let expr_impl = generate_expr_impl(
            &zst_ident,
            &mysql_value,
            &sql_type_marker,
            &sql_nullable_marker,
        );
        let custom_comparison_operand_impls =
            generate_custom_comparison_operand_impls(info, &zst_ident, &mysql_value);
        let arithmetic_ops = if !info.is_custom_type && info.is_numeric() {
            generate_arithmetic_ops(
                &zst_ident,
                mysql_value.clone(),
                sql_type_marker.clone(),
                sql_nullable_marker.clone(),
            )
        } else {
            TokenStream::new()
        };
        let assignment_validation = if info.is_custom_type {
            TokenStream::new()
        } else {
            quote! {
            const _: () = {
                fn assert_mysql_assignment<T, Expected>()
                where
                    T: drizzle::core::ValueTypeForDialect<drizzle::mysql::MySQLDialect>,
                    Expected: drizzle::core::types::Assignable<
                        <T as drizzle::core::ValueTypeForDialect<
                            drizzle::mysql::MySQLDialect,
                        >>::SQLType,
                    >,
                {
                }

                let _ = assert_mysql_assignment::<#value_type, #sql_type_marker>;
            };
            }
        };
        let codec_storage_validation = if info.is_custom_type && info.has_explicit_type {
            let drizzle_mysql_column = mysql_paths::drizzle_mysql_column();
            quote! {
                const _: fn() = || {
                    fn assert_column_storage<
                        T: #drizzle_mysql_column<SQLType = #sql_type_marker>,
                    >() {}
                    assert_column_storage::<#value_type>();
                };
            }
        } else {
            TokenStream::new()
        };
        let effective_charset = info.charset.as_deref().or(ctx.attrs.charset.as_deref());
        let charset = effective_charset.map_or_else(
            || quote!(::core::option::Option::None),
            |value| quote!(::core::option::Option::Some(#value)),
        );
        let effective_collate = info.collate.as_deref().or(ctx.attrs.collate.as_deref());
        let collate = effective_collate.map_or_else(
            || quote!(::core::option::Option::None),
            |value| quote!(::core::option::Option::Some(#value)),
        );
        let mysql_column_impl = generate_mysql_column(
            &zst_ident,
            &quote! { #ddl_name },
            &quote! { #is_auto_increment },
            &charset,
            &collate,
        );
        let index_column_impl = if info.is_custom_type {
            let drizzle_mysql_column = mysql_paths::drizzle_mysql_column();
            quote! {
                impl drizzle::mysql::traits::MySQLIndexColumn for #zst_ident
                where
                    <#value_type as #drizzle_mysql_column>::SQLType:
                        drizzle::mysql::index::IndexType,
                {}
            }
        } else if info.is_indexable_without_prefix() {
            quote!(impl drizzle::mysql::traits::MySQLIndexColumn for #zst_ident {})
        } else {
            TokenStream::new()
        };
        let insert_column_impl = info
            .generated_column
            .is_none()
            .then(|| crate::common::insert_select::generate_column_impl(&zst_ident, struct_ident));
        let to_sql_impl = generate_to_sql(&zst_ident, &to_sql_body);

        // Generate marker const using original tokens for IDE documentation
        let marker_const = generate_marker_const(info, &zst_ident);

        let column_membership_impl = quote! {
            impl #column_of<#struct_ident> for #zst_ident {}
            impl #column_value_type for #zst_ident {
                type ValueType = #value_type;
            }
        };
        let column_scope_impl =
            crate::common::insert_select::generate_column_scope_impl(&zst_ident, struct_ident);
        let column_not_null_impl = if !info.is_nullable || info.is_primary() {
            quote! {
                impl #column_not_null for #zst_ident {}
            }
        } else {
            quote! {}
        };

        // Grouping by a table's sole primary key functionally determines the
        // whole row (SQL:1999), so `.group_by(table.pk)` produces the
        // `PkGroup` marker, which lets any scalar column of the table appear
        // in SELECT. Composite-PK members keep exact-list semantics.
        let group_by_columns_ty = if info.constraint.is_inline_primary() {
            quote! { drizzle::core::PkGroup<#struct_ident> }
        } else {
            quote! { drizzle::core::Cons<#zst_ident, drizzle::core::Nil> }
        };

        let column_code = quote! {
            #struct_def
            impl<'a> ::core::default::Default for &'a #zst_ident {
                fn default() -> Self {
                    static COLUMN: #zst_ident = #zst_ident;
                    &COLUMN
                }
            }
            #impl_new

            impl #zst_ident {
                #marker_const
            }

            #sql_schema_field_impl
            #sql_column_info_impl
            #sql_column_impl
            #mysql_column_impl
            #index_column_impl
            #insert_column_impl
            #column_membership_impl
            #column_scope_impl
            #column_not_null_impl
            impl #expr_value_type for #zst_ident {
                type ValueType = #rust_type;
            }
            impl #into_select_target for #zst_ident {
                type Marker = #select_cols<(#zst_ident,)>;
            }
            impl drizzle::core::expr::HasAggStatus for #zst_ident {
                type Status = drizzle::core::expr::AllScalar;
            }
            impl drizzle::core::GroupByIdentity for #zst_ident {
                type Identity = #zst_ident;
            }
            #to_sql_impl
            impl<'a> drizzle::core::IntoGroupBy<'a, #mysql_value<'a>> for #zst_ident {
                type Columns = #group_by_columns_ty;
            }
            #expr_impl
            #custom_comparison_operand_impls
            #arithmetic_ops
            #assignment_validation
            #codec_storage_validation
        };
        all_column_code.extend(column_code);
    }
    Ok((all_column_code, column_zst_idents))
}

/// Generates the `impl` block on the table struct for individual column access.
/// E.g., `impl User { pub const id: UserId = UserId; }`
pub fn generate_column_accessors(ctx: &MacroContext, column_zst_idents: &[Ident]) -> TokenStream {
    let MacroContext {
        struct_ident,
        field_infos,
        ..
    } = &ctx;
    let const_defs = field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst_ident)| {
            let const_name = &info.ident; // The original field name, e.g., `id`
            quote! {
                pub const #const_name: #zst_ident = #zst_ident;
            }
        });

    let fields = field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst)| {
            let name = &info.ident;
            quote! {
                #name: #zst
            }
        });

    quote! {
        #[allow(non_upper_case_globals)]
        impl #struct_ident {
            pub const fn new() -> Self {
                Self {
                    #(#fields,)*
                }
            }
            #(#const_defs)*
        }
    }
}

/// Generates the column fields for the table struct.
pub fn generate_column_fields(ctx: &MacroContext, column_zst_idents: &[Ident]) -> TokenStream {
    let const_defs =
        ctx.field_infos
            .iter()
            .zip(column_zst_idents.iter())
            .map(|(info, zst_ident)| {
                let const_name = &info.ident; // The original field name, e.g., `id`
                quote! {
                    pub #const_name: #zst_ident
                }
            });

    quote! {
        #(#const_defs,)*
    }
}
