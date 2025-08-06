// Add to your proc-macro's Cargo.toml:
// heck = "0.4"

#[cfg(feature = "rusqlite")]
pub mod rusqlite;

use super::field::{FieldInfo, SQLiteType};
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
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
        "CREATE TABLE \"{}\" ({}",
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
        let (is_primary, is_not_null, is_unique, is_autoincrement) = (
            info.is_primary,
            !info.is_nullable,
            info.is_unique,
            info.is_autoincrement,
        );

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

                fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
                    #default_fn_body
                }
            }

            impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for #zst_ident {
                const AUTOINCREMENT: bool = #is_autoincrement;
            }

            impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #zst_ident {
                fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                    SQL::raw(#name)
                }
            }

            impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for #zst_ident {
                fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
                    ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed(#name))
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
    let column_len = column_zst_idents.len();

    // Generate SQLColumnInfo implementations for each column ZST
    let column_info_impls = column_zst_idents.iter().enumerate().map(|(i, ident)| {
        // You'll need to pass column metadata to generate these properly
        quote! {
            impl ::drizzle_rs::core::SQLColumnInfo for #ident {

                fn name(&self) -> &str {
                    <Self as ::drizzle_rs::core::SQLColumn<'_, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::Schema::NAME
                }
                fn r#type(&self) -> &str {
                    <Self as ::drizzle_rs::core::SQLColumn<'_, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::Schema::TYPE
                }
                fn is_primary_key(&self) -> bool {
                    <Self as ::drizzle_rs::core::SQLColumn<'_, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::PRIMARY_KEY
                }
                fn is_not_null(&self) -> bool {
                    <Self as ::drizzle_rs::core::SQLColumn<'_, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::NOT_NULL
                }
                fn is_unique(&self) -> bool {
                    <Self as ::drizzle_rs::core::SQLColumn<'_, ::drizzle_rs::sqlite::SQLiteValue<'_>>>::UNIQUE
                }
            }

            impl ::drizzle_rs::sqlite::SQLiteColumnInfo for #ident {
                fn is_autoincrement(&self) -> bool {
                    <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
                }
            }
        }
    });

    Ok(quote! {
        #(#column_info_impls)*

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
            const COUNT: usize = #column_len;
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
    let mut insert_field_conversions = Vec::new();
    let mut update_field_conversions = Vec::new();
    let mut select_column_names = Vec::new();
    let mut insert_column_names = Vec::new();
    let mut update_column_names = Vec::new();
    let mut update_field_names = Vec::new();
    let mut insert_convenience_methods = Vec::new();
    let mut update_convenience_methods = Vec::new();
    let mut constructor_params = Vec::new();
    let mut constructor_assignments = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let (select_type, insert_type, update_type) = (
            info.get_select_type(),
            info.get_insert_type(),
            info.get_update_type(),
        );
        let base_type = info.base_type;

        select_fields.push(quote! { pub #name: #select_type });
        update_fields.push(quote! { pub #name: #update_type });

        // Generate Insert model fields using original types
        if info.is_nullable || info.has_default {
            insert_fields.push(quote! { pub #name: Option<#base_type> });
        } else {
            insert_fields.push(quote! { pub #name: #base_type });
        }

        // Logic for Insert model's `Default` impl
        let default_value = if let Some(f) = &info.default_fn {
            if info.is_nullable || info.has_default {
                quote! { #name: Some(#f()) }
            } else {
                quote! { #name: #f() }
            }
        } else {
            quote! { #name: ::std::default::Default::default() }
        };
        insert_default_fields.push(default_value);

        // Generate field conversion for Insert model ToSQL
        let field_conversion = if info.is_primary && info.is_autoincrement {
            // Skip auto-increment primary keys - they shouldn't be in insert
            continue;
        } else {
            // Add column name for Insert model (non auto-increment fields)
            let column_name = &info.column_name;
            insert_column_names.push(quote! { #column_name });

            if info.is_nullable || info.has_default {
                quote! {
                    match &self.#name {
                        Some(val) => val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        None => ::drizzle_rs::sqlite::SQLiteValue::Null,
                    }
                }
            } else {
                quote! {
                    self.#name.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null)
                }
            }
        };
        insert_field_conversions.push(field_conversion);

        // Generate field conversion for Update model ToSQL
        // Update models typically have all fields as Option<T>, only include Some() values
        let update_conversion = if info.is_primary {
            // Skip primary key fields in updates
            quote! {}
        } else {
            let column_name = &info.column_name;
            update_column_names.push(quote! { #column_name });
            update_field_names.push(name);
            quote! {
                if let Some(val) = &self.#name {
                    assignments.push((#column_name, val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null)));
                }
            }
        };
        update_field_conversions.push(update_conversion);

        // Add column name for Select model
        let column_name = &info.column_name;
        select_column_names.push(quote! { #column_name });

        // Generate convenience methods for Update model (with_ methods only)
        if !info.is_primary {
            let update_method_name = format_ident!("with_{}", name);
            
            // For Update models, all fields are Option<T>, so we always set Some(value)
            if info.is_uuid {
                let update_convenience_methods_item = quote! {
                    pub fn #update_method_name<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                };
                update_convenience_methods.push(update_convenience_methods_item);
            } else if base_type.to_token_stream().to_string().contains("String") {
                let update_convenience_methods_item = quote! {
                    pub fn #update_method_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                };
                update_convenience_methods.push(update_convenience_methods_item);
            } else if base_type.to_token_stream().to_string().contains("Vec")
                && base_type.to_token_stream().to_string().contains("u8")
            {
                let update_convenience_methods_item = quote! {
                    pub fn #update_method_name<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                };
                update_convenience_methods.push(update_convenience_methods_item);
            } else {
                let update_convenience_methods_item = quote! {
                    pub fn #update_method_name(mut self, value: #base_type) -> Self {
                        self.#name = Some(value);
                        self
                    }
                };
                update_convenience_methods.push(update_convenience_methods_item);
            }
        }

        // Generate convenience methods for Insert model (with_ methods only)
        let method_name = format_ident!("with_{}", name);

        if info.is_nullable || info.has_default {
            // For special types that need custom conversion
            if info.is_uuid {
                let convenience_methods = quote! {
                    pub fn #method_name<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            } else if base_type.to_token_stream().to_string().contains("String") {
                let convenience_methods = quote! {
                    pub fn #method_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            } else if base_type.to_token_stream().to_string().contains("Vec")
                && base_type.to_token_stream().to_string().contains("u8")
            {
                let convenience_methods = quote! {
                    pub fn #method_name<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            } else {
                let convenience_methods = quote! {
                    pub fn #method_name(mut self, value: #base_type) -> Self {
                        self.#name = Some(value);
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            }
        } else {
            // For non-optional fields
            if info.is_uuid {
                let convenience_methods = quote! {
                    pub fn #method_name<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
                        self.#name = value.into();
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            } else if base_type.to_token_stream().to_string().contains("String") {
                let convenience_methods = quote! {
                    pub fn #method_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
                        self.#name = value.into();
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            } else if base_type.to_token_stream().to_string().contains("Vec")
                && base_type.to_token_stream().to_string().contains("u8")
            {
                let convenience_methods = quote! {
                    pub fn #method_name<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
                        self.#name = value.into();
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            } else {
                let convenience_methods = quote! {
                    pub fn #method_name(mut self, value: #base_type) -> Self {
                        self.#name = value;
                        self
                    }
                };
                insert_convenience_methods.push(convenience_methods);
            }
        }

        // Generate constructor parameters and assignments (skip auto-increment primary keys)
        if !(info.is_primary && info.is_autoincrement) {
            if info.is_nullable || info.has_default {
                // Optional parameter for nullable/default fields
                if info.is_uuid {
                    constructor_params.push(quote! { #name: Option<impl Into<::uuid::Uuid>> });
                    constructor_assignments.push(quote! { #name: #name.map(|v| v.into()) });
                } else if base_type.to_token_stream().to_string().contains("String") {
                    constructor_params.push(quote! { #name: Option<impl Into<::std::string::String>> });
                    constructor_assignments.push(quote! { #name: #name.map(|v| v.into()) });
                } else if base_type.to_token_stream().to_string().contains("Vec")
                    && base_type.to_token_stream().to_string().contains("u8")
                {
                    constructor_params.push(quote! { #name: Option<impl Into<::std::vec::Vec<u8>>> });
                    constructor_assignments.push(quote! { #name: #name.map(|v| v.into()) });
                } else {
                    constructor_params.push(quote! { #name: Option<#base_type> });
                    constructor_assignments.push(quote! { #name });
                }
            } else {
                // Required parameter for non-nullable fields
                if info.is_uuid {
                    constructor_params.push(quote! { #name: impl Into<::uuid::Uuid> });
                    constructor_assignments.push(quote! { #name: #name.into() });
                } else if base_type.to_token_stream().to_string().contains("String") {
                    constructor_params.push(quote! { #name: impl Into<::std::string::String> });
                    constructor_assignments.push(quote! { #name: #name.into() });
                } else if base_type.to_token_stream().to_string().contains("Vec")
                    && base_type.to_token_stream().to_string().contains("u8")
                {
                    constructor_params.push(quote! { #name: impl Into<::std::vec::Vec<u8>> });
                    constructor_assignments.push(quote! { #name: #name.into() });
                } else {
                    constructor_params.push(quote! { #name: #base_type });
                    constructor_assignments.push(quote! { #name });
                }
            }
        }
    }

    Ok(quote! {
        // Select Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #select_model { #(#select_fields,)* }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;

                // Generate column list for SELECT
                const COLUMN_NAMES: &'static [&'static str] = &[#(#select_column_names,)*];
                SQL::columns(COLUMN_NAMES)
            }
        }

        // Insert Model
        #[derive(Debug, Clone, PartialEq)]
        pub struct #insert_model {
            #(#insert_fields,)*
        }
        impl Default for #insert_model {
            fn default() -> Self { Self { #(#insert_default_fields,)* } }
        }
        impl #insert_model {
            pub fn new(#(#constructor_params),*) -> Self {
                Self {
                    #(#constructor_assignments,)*
                    ..Self::default()
                }
            }

            // Convenience methods for setting fields
            #(#insert_convenience_methods)*
        }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                use ::drizzle_rs::sqlite::SQLiteValue;
                use ::std::convert::TryInto;

                let mut values = Vec::new();

                // Generate field value extraction
                #(values.push(#insert_field_conversions);)*

                SQL::parameters(values)
            }
        }

        // Update Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #update_model { #(#update_fields,)* }
        impl #update_model {
            // Convenience methods for setting fields
            #(#update_convenience_methods)*
        }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                use ::drizzle_rs::sqlite::SQLiteValue;
                use ::std::convert::TryInto;

                let mut assignments = Vec::new();

                // Generate field assignment pairs, only for Some() values
                #(#update_field_conversions)*

                SQL::assignments(assignments)
            }
        }

        // SQLModel implementations
        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                const COLUMN_NAMES: &'static [&'static str] = &[#(#select_column_names,)*];
                SQL::columns(COLUMN_NAMES)
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                SQL::raw("*")
            }
        }

        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                const COLUMN_NAMES: &'static [&'static str] = &[#(#insert_column_names,)*];
                SQL::columns(COLUMN_NAMES)
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                use ::drizzle_rs::sqlite::SQLiteValue;
                use ::std::convert::TryInto;

                let mut values = Vec::new();
                #(values.push(#insert_field_conversions);)*
                SQL::parameters(values)
            }
        }

        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                const COLUMN_NAMES: &'static [&'static str] = &[#(#update_column_names,)*];
                SQL::columns(COLUMN_NAMES)
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::SQL;
                use ::drizzle_rs::sqlite::SQLiteValue;
                use ::std::convert::TryInto;

                let mut values = Vec::new();
                // For Update model, only include values that are Some()
                #(
                    if let Some(val) = &self.#update_field_names {
                        values.push(val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null));
                    }
                )*
                SQL::parameters(values)
            }
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
                            ::serde_json::to_vec(&value)
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
                            ::serde_json::to_string(&value)
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

    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = rusqlite::generate_rusqlite_impls(
        &context.select_model_ident,
        &context.insert_model_ident,
        &context.update_model_ident,
        &field_infos,
    )?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote!();

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
        #rusqlite_impls
    })
}
