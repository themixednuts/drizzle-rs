// Add to your proc-macro's Cargo.toml:
// heck = "0.4"

#[cfg(feature = "rusqlite")]
pub mod rusqlite;

use super::field::{FieldInfo, SQLiteType};
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rusqlite::generate_rusqlite_from_to_sql;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Meta, Result, parse::Parse, parse_macro_input};

// A context struct to hold all the necessary information for generation.
// This avoids passing many arguments to every function.
struct MacroContext<'a> {
    struct_ident: &'a Ident,
    table_name: String,
    create_table_sql: String,
    field_infos: &'a [FieldInfo<'a>],
    select_model_ident: Ident,
    insert_model_ident: Ident,
    update_model_ident: Ident,
}

// ============================================================================
// 1. Attribute Parsing
// ============================================================================

#[derive(Default)]
pub(crate) struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) strict: bool,
    pub(crate) without_rowid: bool,
}

impl Parse for TableAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = TableAttributes::default();
        let metas = input.parse_terminated(Meta::parse, syn::Token![,])?;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    if let syn::Expr::Lit(lit) = nv.clone().value {
                        if let syn::Lit::Str(str_lit) = lit.lit {
                            attrs.name = Some(str_lit.value());
                            continue;
                        }
                    }
                    return Err(syn::Error::new(
                        nv.span(),
                        "Expected a string literal for 'name'",
                    ));
                }
                Meta::Path(path) if path.is_ident("strict") => attrs.strict = true,
                Meta::Path(path) if path.is_ident("without_rowid") => attrs.without_rowid = true,
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "Unrecognized table attribute. Supported attributes are: name, strict, without_rowid",
                    ));
                }
            }
        }
        Ok(attrs)
    }
}

// ============================================================================
// 2. Generation Logic (Broken into smaller functions)
// ============================================================================

/// Generates the `CREATE TABLE` SQL string.
fn generate_create_table_sql(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> String {
    let column_defs: Vec<_> = field_infos
        .iter()
        .map(|info| {
            // Add AUTOINCREMENT only for single-column integer primary keys
            if info.is_autoincrement && info.is_primary && !is_composite_pk {
                format!("{} AUTOINCREMENT", info.sql_definition)
            } else {
                info.sql_definition.clone()
            }
        })
        .collect();

    let mut create_sql = format!(
        "CREATE TABLE \"{}\" ({})",
        table_name,
        column_defs.join(", ")
    );

    if is_composite_pk {
        let pk_cols = field_infos
            .iter()
            .filter(|info| info.is_primary)
            .map(|info| format!("\"{}\"", info.column_name))
            .collect::<Vec<_>>()
            .join(", ");
        create_sql.push_str(&format!(", PRIMARY KEY ({})", pk_cols));
    }

    create_sql.push(')');
    if without_rowid {
        create_sql.push_str(" WITHOUT ROWID");
    }
    if strict {
        create_sql.push_str(" STRICT");
    }
    create_sql.push(';');
    create_sql
}

/// Generates the `impl` block on the table struct for individual column access.
/// E.g., `impl User { pub const id: UserId = UserId; }`
fn generate_column_accessors(
    struct_ident: &Ident,
    field_infos: &[FieldInfo],
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let const_defs = field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst_ident)| {
            let const_name = info.ident; // The original field name, e.g., `id`
            quote! {
                pub const #const_name: #zst_ident = #zst_ident;
            }
        });

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        impl #struct_ident {
            #(#const_defs)*
        }
    })
}

/// Generates the column ZSTs and their `SQLColumn` implementations.
fn generate_column_definitions<'a>(ctx: &MacroContext<'a>) -> Result<(TokenStream, Vec<Ident>)> {
    let mut all_column_code = TokenStream::new();
    let mut column_zst_idents = Vec::new();
    let struct_ident = &ctx.struct_ident;

    for info in ctx.field_infos {
        let field_pascal_case = info.ident.to_string().to_upper_camel_case();
        let zst_ident = format_ident!("{}{}", ctx.struct_ident, field_pascal_case);
        column_zst_idents.push(zst_ident.clone());

        let (value_type, rust_type) = (&info.base_type, &info.field_type);
        let (is_primary, is_not_null, is_unique) =
            (info.is_primary, !info.is_nullable, info.is_unique);

        let default_const = info
            .default_value
            .as_ref()
            .map_or_else(|| quote! { None }, |val| quote! { Some(#val) });

        let default_fn_body = info.default_fn.as_ref().map_or_else(
            || quote! { None::<fn() -> Self::Type> },
            |func| quote! { Some(|| #func) },
        );

        let sql = &info.sql_definition;

        let name = &info.column_name;
        let col_type = &info.column_type.to_sql_type();

        let column_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default)]
            pub struct #zst_ident;

            impl <'a> ::drizzle_rs::core::SQLSchema<'a, &'a str> for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: &'a str = #sql;
            }

            impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #zst_ident {
                type Table = #struct_ident;
                type Type = #rust_type;
                type Schema = Self;

                const PRIMARY_KEY: bool = #is_primary;
                const NOT_NULL: bool = #is_not_null;
                const UNIQUE: bool = #is_unique;
                const DEFAULT: Option<Self::Type> = #default_const;

                fn default_fn() -> Option<impl Fn() -> Self::Type> {
                    #default_fn_body
                }
            }

            impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #zst_ident {
                fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                    unimplemented!()
                }
            }
        };
        all_column_code.extend(column_code);
    }
    Ok((all_column_code, column_zst_idents))
}

/// Generates the `SQLSchema` and `SQLTable` implementations.
fn generate_table_impls(ctx: &MacroContext, column_zst_idents: &[Ident]) -> Result<TokenStream> {
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let create_table_sql = &ctx.create_table_sql;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    Ok(quote! {
        impl<'a> ::drizzle_rs::core::SQLSchema<'a, ::drizzle_rs::core::SQLSchemaType> for #struct_ident {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
            const SQL: &'a str = #create_table_sql;
        }

        impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_ident {
            type Schema = Self;
            type Select = #select_model;
            type Insert = #insert_model;
            type Update = #update_model;

            type Columns = (#(#column_zst_idents,)*);
            const COLUMNS: Self::Columns = (#(#column_zst_idents,)*);
        }
    })
}

/// Generates the `Select`, `Insert`, `Update` model structs and their impls.
fn generate_model_definitions(ctx: &MacroContext) -> Result<TokenStream> {
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    let mut select_fields = Vec::new();
    let mut insert_fields = Vec::new();
    let mut update_fields = Vec::new();
    let mut insert_default_fields = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let (select_type, insert_type, update_type) = (
            info.get_select_type(),
            info.get_insert_type(),
            info.get_update_type(),
        );
        select_fields.push(quote! { pub #name: #select_type });
        insert_fields.push(quote! { pub #name: #insert_type });
        update_fields.push(quote! { pub #name: #update_type });

        // Logic for Insert model's `Default` impl
        let default_value = if let Some(f) = &info.default_fn {
            let base_type = info.base_type;
            let needs_option = quote!(#insert_type).to_string().starts_with("Option <");
            if needs_option {
                quote! { #name: Some(#f()) }
            } else {
                quote! { #name: #f() }
            }
        } else {
            quote! { #name: ::std::default::Default::default() }
        };
        insert_default_fields.push(default_value);
    }

    Ok(quote! {
        // Select Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #select_model { #(#select_fields,)* }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> { unimplemented!() }
        }

        // Insert Model
        #[derive(Debug, Clone, PartialEq)]
        pub struct #insert_model { #(#insert_fields,)* }
        impl Default for #insert_model {
            fn default() -> Self { Self { #(#insert_default_fields,)* } }
        }
        impl #insert_model {
            pub fn new() -> Self { Self::default() }
        }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> { unimplemented!() }
        }

        // Update Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #update_model { #(#update_fields,)* }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> { unimplemented!() }
        }
    })
}

/// Generates `FromSql` and `ToSql` impls for JSON fields.
fn generate_json_impls(field_infos: &[FieldInfo<'_>]) -> Result<TokenStream> {
    // Create a filter for JSON fields
    let json_fields: Vec<_> = field_infos.iter().filter(|info| info.is_json).collect();

    // If no JSON fields, return an empty TokenStream
    if json_fields.is_empty() {
        return Ok(quote!());
    }

    let json_impls = json_fields.iter()
        .map(|info| {
            if info.is_json && !cfg!(feature = "serde") {
                return Err(syn::Error::new_spanned(info.ident, "serde feature is required for JSON fields"))
            }
            let struct_name = info.base_type;
            let column_type_str = info.column_type.to_sql_type();

            // Different implementation based on the column type (TEXT vs BLOB)
            let impl_block = if column_type_str == "blob" {
                quote! {
                    impl<'a> ::std::convert::TryFrom<#struct_name> for ::drizzle_rs::sqlite::SQLiteValue<'a> {
                        type Error = ::drizzle_rs::core::DrizzleError;

                        fn try_from(value: #struct_name) -> ::std::result::Result<Self, Self::Error> {
                            serde_json::to_vec(&value)
                                .map(|json| ::drizzle_rs::sqlite::SQLiteValue::Blob(json.into()))
                                .map_err(|e| ::drizzle_rs::core::DrizzleError::ParameterError(
                                    format!("Failed to serialize JSON to blob: {}", e)
                                ))
                            }
                        }
                }

            } else {
                // Default to TEXT
                quote! {
                        impl<'a> ::std::convert::TryFrom<#struct_name> for ::drizzle_rs::sqlite::SQLiteValue<'a> {

                            type Error = ::drizzle_rs::core::DrizzleError;

                            fn try_from(value: #struct_name) -> ::std::result::Result<Self, Self::Error> {
                                serde_json::to_string(&value)
                                    .map(|json| ::drizzle_rs::sqlite::SQLiteValue::Text(json.into()))
                                    .map_err(|e| ::drizzle_rs::core::DrizzleError::ParameterError(
                                        format!("Failed to serialize JSON to text: {}", e)
                                    ))
                            }
                        }
                }
            };

            Ok(impl_block)
        })
        .collect::<Result<Vec<_>>>()?;

    let impls = generate_rusqlite_from_to_sql(&json_fields)?;

    let json_types_impl = quote! {
        #(#json_impls)*
        #(#impls)*
    };

    Ok(json_types_impl)
}

// ============================================================================
// 3. Main Macro Entry Point
// ============================================================================

pub(crate) fn table_attr_macro(input: DeriveInput, attrs: TableAttributes) -> Result<TokenStream> {
    // -------------------
    // 1. Setup Phase
    // -------------------
    let struct_ident = &input.ident;
    let table_name = attrs.name.unwrap_or_else(|| struct_ident.to_string());

    let fields = if let Data::Struct(data) = &input.data {
        &data.fields
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Table macro can only be applied to structs.",
        ));
    };

    let primary_key_count = fields
        .iter()
        .filter(|f| FieldInfo::from_field(f, false).is_ok_and(|f| f.is_primary))
        .count();
    let is_composite_pk = primary_key_count > 1;

    let field_infos: Vec<FieldInfo> = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    let create_table_sql = generate_create_table_sql(
        &table_name,
        &field_infos,
        is_composite_pk,
        attrs.strict,
        attrs.without_rowid,
    );

    let context = MacroContext {
        struct_ident,
        table_name,
        create_table_sql,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
    };

    // -------------------
    // 2. Generation Phase
    // -------------------
    let (column_definitions, column_zst_idents) = generate_column_definitions(&context)?;
    let column_accessors =
        generate_column_accessors(struct_ident, &field_infos, &column_zst_idents)?;
    let table_impls = generate_table_impls(&context, &column_zst_idents)?;
    let model_definitions = generate_model_definitions(&context)?;
    let json_impls = generate_json_impls(&field_infos)?;

    // #[cfg(feature = "rusqlite")]
    // let rusqlite_impls = rusqlite::generate_rusqlite_impls(...) else { quote!() };

    // -------------------
    // 3. Assembly Phase
    // -------------------
    Ok(quote! {
        // The main, user-facing struct is now a ZST.
        // It acts as a namespace for the table's schema.
        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct #struct_ident;
        #column_accessors

        // All generated code is scoped under a module to avoid polluting the global namespace.
        // Or you can output it directly as done here.

        #column_definitions
        #table_impls
        #model_definitions
        #json_impls
        // #rusqlite_impls
    })
}
