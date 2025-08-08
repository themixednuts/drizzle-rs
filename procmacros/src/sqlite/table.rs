// Add to your proc-macro's Cargo.toml:
// heck = "0.4"

#[cfg(feature = "rusqlite")]
pub mod rusqlite;

use super::field::FieldInfo;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
#[cfg(feature = "rusqlite")]
use rusqlite::generate_rusqlite_from_to_sql;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Expr, Meta, Result, parse::Parse};

// Common SQLite documentation URLs for error messages and macro docs
const SQLITE_CREATE_TABLE_URL: &str = "https://sqlite.org/lang_createtable.html";
const SQLITE_AUTOINCREMENT_URL: &str = "https://sqlite.org/autoinc.html"; 
const SQLITE_WITHOUT_ROWID_URL: &str = "https://sqlite.org/withoutrowid.html";
const SQLITE_STRICT_TABLES_URL: &str = "https://sqlite.org/stricttables.html";
const SQLITE_DATATYPE_URL: &str = "https://sqlite.org/datatype3.html";
const SQLITE_JSON_URL: &str = "https://sqlite.org/json1.html";
const SQLITE_CONSTRAINTS_URL: &str = "https://sqlite.org/lang_createtable.html#constraints";

// Enhanced context struct to hold all the necessary information for generation.
// This provides helper methods to reduce code duplication and improve maintainability.
struct MacroContext<'a> {
    struct_ident: &'a Ident,
    table_name: String,
    create_table_sql: String,
    field_infos: &'a [FieldInfo<'a>],
    select_model_ident: Ident,
    select_model_partial_ident: Ident,
    insert_model_ident: Ident,
    update_model_ident: Ident,
    without_rowid: bool,
    strict: bool,
}

impl<'a> MacroContext<'a> {
    /// Checks if a field should be optional in the Insert model
    fn is_field_optional_in_insert(&self, field: &FieldInfo) -> bool {
        // Nullable fields are always optional
        if field.is_nullable {
            return true;
        }
        
        // Fields with explicit defaults (SQL or runtime) are optional  
        if field.has_default || field.default_fn.is_some() {
            return true;
        }
        
        // Primary key logic depends on table type and field type
        if field.is_primary {
            // WITHOUT ROWID tables: primary keys never auto-increment, need explicit default
            if self.without_rowid {
                return false;
            }
            
            // Regular tables: only INTEGER primary keys can auto-increment
            use crate::sqlite::field::SQLiteType;
            match field.column_type {
                SQLiteType::Integer => true,  // INTEGER PRIMARY KEY can auto-increment
                _ => false,  // TEXT, BLOB, etc. primary keys cannot auto-increment
            }
        } else {
            false  // Non-primary, non-nullable, no-default fields are required
        }
    }

    /// Checks if a field should be optional in the Update model (all fields are optional)
    fn is_field_optional_in_update(&self, _field: &FieldInfo) -> bool {
        true
    }

    /// Checks if a field should generate a convenience method
    fn should_generate_convenience_method(&self, field: &FieldInfo) -> bool {
        !field.is_autoincrement || !field.is_primary
    }

    /// Gets the appropriate field type for a specific model
    fn get_field_type_for_model(&self, field: &FieldInfo, model_type: ModelType) -> TokenStream {
        let base_type = field.base_type;
        match model_type {
            ModelType::Select => field.get_select_type(),
            ModelType::Insert => {
                if self.is_field_optional_in_insert(field) {
                    quote!(Option<#base_type>)
                } else {
                    quote!(#base_type)
                }
            },
            ModelType::Update => quote!(Option<#base_type>),
            ModelType::PartialSelect => quote!(Option<#base_type>),
        }
    }

    /// Checks if a field should be skipped in insert ToSQL conversion (autoincrement primary keys)
    fn should_skip_field_in_insert(&self, field: &FieldInfo) -> bool {
        field.is_primary && field.is_autoincrement
    }

    /// Gets the default value expression for insert model
    fn get_insert_default_value(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        
        // Handle runtime function defaults (default_fn)  
        if let Some(f) = &field.default_fn {
            if self.is_field_optional_in_insert(field) {
                return quote! { #name: Some((|| #f())()) };
            } else {
                return quote! { #name: (|| #f())() };
            }
        }
        
        // Handle compile-time SQL defaults (default = literal)
        if field.has_default {
            if self.is_field_optional_in_insert(field) {
                // SQL defaults handled at database level, insert None to let DB apply default
                return quote! { #name: None };
            } else {
                // This shouldn't happen - fields with SQL defaults should be optional
                return quote! { #name: ::std::default::Default::default() };
            }
        }
        
        // Handle fields without explicit defaults
        if self.is_field_optional_in_insert(field) {
            quote! { #name: None }
        } else {
            // Required field without default - this should cause a compile error
            // For now, we'll use Default::default() but this may fail at runtime
            quote! { #name: ::std::default::Default::default() }
        }
    }

    /// Generates field conversion for insert ToSQL
    fn get_insert_field_conversion(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        
        // Default conversion for all fields (UUIDs will use generic From<Uuid> -> SQLiteValue::Blob)
        if self.is_field_optional_in_insert(field) {
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
    }

    /// Generates field conversion for update ToSQL
    fn get_update_field_conversion(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        let column_name = &field.column_name;
        
        // Handle UUID fields with field-type-aware conversion
        if field.is_uuid {
            use crate::sqlite::field::SQLiteType;
            let uuid_conversion = match field.column_type {
                SQLiteType::Text => {
                    // Store UUID as TEXT (string format)
                    quote! { ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Owned(val.to_string())) }
                },
                SQLiteType::Blob => {
                    // Store UUID as BLOB (binary format) 
                    quote! { ::drizzle_rs::sqlite::SQLiteValue::Blob(::std::borrow::Cow::Owned(val.as_bytes().to_vec())) }
                },
                _ => {
                    // Fallback to generic conversion for other types
                    quote! { val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null) }
                }
            };
            
            return quote! {
                if let Some(val) = &self.#name {
                    assignments.push((#column_name, #uuid_conversion));
                }
            };
        }
        
        // Default conversion for non-UUID fields
        quote! {
            if let Some(val) = &self.#name {
                assignments.push((#column_name, val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null)));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ModelType {
    Select,
    Insert,
    Update,
    PartialSelect,
}

/// Helper struct for generating convenience methods in a DRY manner
struct ConvenienceMethodGenerator;

impl ConvenienceMethodGenerator {
    /// Generates a convenience method for a field based on its type
    fn generate_method(field: &FieldInfo, model_type: ModelType) -> TokenStream {
        let field_name = field.ident;
        let base_type = field.base_type;
        let method_name = format_ident!("with_{}", field_name);

        let (assignment, return_type) = match model_type {
            ModelType::Insert => {
                if field.is_nullable || field.has_default || field.is_primary {
                    (quote! { self.#field_name = Some(value); }, quote!(Option<#base_type>))
                } else {
                    (quote! { self.#field_name = value; }, quote!(#base_type))
                }
            },
            ModelType::Update => {
                (quote! { self.#field_name = Some(value); }, quote!(Option<#base_type>))
            },
            ModelType::PartialSelect => {
                (quote! { self.#field_name = Some(value); }, quote!(Option<#base_type>))
            },
            _ => return quote!(), // Only generate for Insert, Update, and PartialSelect models
        };

        // Generate type-specific convenience methods using modern pattern matching
        let type_string = base_type.to_token_stream().to_string();
        match (field.is_uuid, type_string.as_str()) {
            (true, _) => quote! {
                pub fn #method_name<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
                    let value = value.into();
                    #assignment
                    self
                }
            },
            (_, s) if s.contains("String") => quote! {
                pub fn #method_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
                    let value = value.into();
                    #assignment
                    self
                }
            },
            (_, s) if s.contains("Vec") && s.contains("u8") => quote! {
                pub fn #method_name<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
                    let value = value.into();
                    #assignment
                    self
                }
            },
            _ => quote! {
                pub fn #method_name(mut self, value: #base_type) -> Self {
                    #assignment
                    self
                }
            },
        }
    }
}

/// Helper struct for generating constructor parameters in a DRY manner
struct ConstructorGenerator;

impl ConstructorGenerator {
    /// Generates constructor parameter and assignment for a field
    fn generate_param_and_assignment(field: &FieldInfo) -> (TokenStream, TokenStream) {
        let field_name = field.ident;
        let base_type = field.base_type;

        // Skip autoincrement primary keys in constructor
        if field.is_primary && field.is_autoincrement {
            return (quote!(), quote!());
        }

        let is_optional = field.is_nullable || field.has_default || field.is_primary;
        let type_string = base_type.to_token_stream().to_string();

        match (is_optional, field.is_uuid, type_string.as_str()) {
            // Optional parameters
            (true, true, _) => (
                quote! { #field_name: Option<impl Into<::uuid::Uuid>> },
                quote! { #field_name: #field_name.map(|v| v.into()) }
            ),
            (true, false, s) if s.contains("String") => (
                quote! { #field_name: Option<impl Into<::std::string::String>> },
                quote! { #field_name: #field_name.map(|v| v.into()) }
            ),
            (true, false, s) if s.contains("Vec") && s.contains("u8") => (
                quote! { #field_name: Option<impl Into<::std::vec::Vec<u8>>> },
                quote! { #field_name: #field_name.map(|v| v.into()) }
            ),
            (true, false, _) => (
                quote! { #field_name: Option<#base_type> },
                quote! { #field_name }
            ),
            
            // Required parameters
            (false, true, _) => (
                quote! { #field_name: impl Into<::uuid::Uuid> },
                quote! { #field_name: #field_name.into() }
            ),
            (false, false, s) if s.contains("String") => (
                quote! { #field_name: impl Into<::std::string::String> },
                quote! { #field_name: #field_name.into() }
            ),
            (false, false, s) if s.contains("Vec") && s.contains("u8") => (
                quote! { #field_name: impl Into<::std::vec::Vec<u8>> },
                quote! { #field_name: #field_name.into() }
            ),
            (false, false, _) => (
                quote! { #field_name: #base_type },
                quote! { #field_name }
            ),
        }
    }
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
                        "Unrecognized table attribute.\n\
                         Supported attributes:\n\
                         - name: Custom table name (e.g., #[table(name = \"custom_name\")])\n\
                         - strict: Enable STRICT mode (e.g., #[table(strict)])\n\
                         - without_rowid: Use WITHOUT ROWID optimization (e.g., #[table(without_rowid)])\n\
                         See: {SQLITE_CREATE_TABLE_URL}",
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
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let MacroContext {struct_ident, field_infos, ..} = &ctx; 
    let const_defs = field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst_ident)| {
            let const_name = info.ident; // The original field name, e.g., `id`
            quote! {
                pub const #const_name: #zst_ident = #zst_ident;
            }
        });

    let fields = field_infos.iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst)| {
        let name = info.ident;
        quote! {
            #name: #zst::new()
        }
    });

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        impl #struct_ident {
            const fn new() -> Self {
                Self {
                    #(#fields,)*
                }
            }
            #(#const_defs)*
        }
    })
}

/// Generates the `impl` block on the table struct for individual column access.
/// E.g., `impl User { pub const id: UserId = UserId; }`
fn generate_column_fields(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let const_defs = ctx.field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst_ident)| {
            let const_name = info.ident; // The original field name, e.g., `id`
            quote! {
                pub #const_name: #zst_ident
            }
        });

    Ok(quote! {
        #(#const_defs,)*
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
            |func| quote! { Some(|| #func()) },
        );

        let sql = &info.sql_definition;

        let name = &info.column_name;
        let col_type = &info.column_type.to_sql_type();

        let column_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            pub struct #zst_ident;

            impl #zst_ident {
                const fn new() -> #zst_ident {
                    #zst_ident {}
                }
            }

            impl <'a> ::drizzle_rs::core::SQLSchema<'a, &'a str> for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: &'a str = #sql;
            }
            impl ::drizzle_rs::core::SQLColumnInfo for #zst_ident {

                fn name(&self) -> &str {
                    <Self as ::drizzle_rs::core::SQLSchema<'_, _>>::NAME
                }
                fn r#type(&self) -> &str {
                    <Self as ::drizzle_rs::core::SQLSchema<'_, _>>::TYPE
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
                fn table(&self) -> &dyn SQLTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }
            }

            impl ::drizzle_rs::sqlite::SQLiteColumnInfo for #zst_ident {
                fn is_autoincrement(&self) -> bool {
                    <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
                }
            }

            impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #zst_ident {
                type Table = #struct_ident;
                type Type = #rust_type;

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

            impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #zst_ident
            {
                fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                    use ::drizzle_rs::core::ToSQL;
                    static INSTANCE: #zst_ident = #zst_ident::new();

                    INSTANCE.as_column().to_sql()
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



    Ok(quote! {
        impl<'a> ::drizzle_rs::core::SQLSchema<'a, ::drizzle_rs::core::SQLSchemaType> for #struct_ident {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
            const SQL: &'a str = #create_table_sql;
        }

        impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_ident {
            type Select = #select_model;
            type Insert = #insert_model;
            type Update = #update_model;
        }

        impl ::drizzle_rs::core::SQLTableInfo for #struct_ident {
            fn name(&self) -> &str {
                <Self as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType>>::NAME
            }
            fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
                <Self as ::drizzle_rs::core::SQLSchema<'_, ::drizzle_rs::core::SQLSchemaType>>::TYPE
            }
            fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            
                Box::new([#(#column_zst_idents.as_column(),)*])
            }
        }

        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                use ::drizzle_rs::core::ToSQL;
                static INSTANCE: #struct_ident = #struct_ident::new();
                INSTANCE.as_table().to_sql()
            }
        }
    })
}

/// Generates compile-time validation blocks for default literals
fn generate_default_validations(field_infos: &[FieldInfo]) -> TokenStream {
    let validations: Vec<TokenStream> = field_infos
        .iter()
        .filter_map(|info| {
            if let Some(Expr::Lit(expr_lit)) = &info.default_value {
                let base_type = info.base_type;
                let field_name = &info.ident.to_string();
                Some(quote! {
                    // Compile-time validation: ensure default literal is compatible with field type
                    const _: () = {
                        // This will cause a compile error if the literal type doesn't match the field type
                        // For example: `let _: i32 = "string";` will fail at compile time
                        //              `let _: String = 42;` will fail at compile time
                        let _: #base_type = #expr_lit;
                    };
                })
            } else {
                None
            }
        })
        .collect();

    if validations.is_empty() {
        quote!() // No validations needed
    } else {
        quote! {
            // Default literal validations - these blocks ensure type compatibility at compile time
            #(#validations)*
        }
    }
}

/// Generates the `Select`, `Insert`, `Update` model structs and their impls.
fn generate_model_definitions(ctx: &MacroContext) -> Result<TokenStream> {
    let select_model = generate_select_model(ctx)?;
    let insert_model = generate_insert_model(ctx)?;
    let update_model = generate_update_model(ctx)?;
    let model_impls = generate_model_trait_impls(ctx)?;

    Ok(quote! {
        #select_model
        #insert_model  
        #update_model
        #model_impls
    })
}

/// Generates the Select model and its partial variant
fn generate_select_model(ctx: &MacroContext) -> Result<TokenStream> {
    let (select_model, select_model_partial) = (
        &ctx.select_model_ident,
        &ctx.select_model_partial_ident,
    );

    let mut select_fields = Vec::new();
    let mut partial_select_fields = Vec::new();
    let mut select_column_names = Vec::new();
    let mut select_field_names = Vec::new();
    let mut partial_convenience_methods = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let select_type = info.get_select_type();
        let base_type = info.base_type;
        let column_name = &info.column_name;

        select_fields.push(quote! { pub #name: #select_type });
        partial_select_fields.push(quote! { pub #name: Option<#base_type> });
        select_column_names.push(quote! { #column_name });
        select_field_names.push(name);

        // Generate convenience methods for partial select
        if ctx.should_generate_convenience_method(info) {
            partial_convenience_methods.push(
                ConvenienceMethodGenerator::generate_method(info, ModelType::PartialSelect)
            );
        }
    }

    Ok(quote! {
        // Select Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #select_model { #(#select_fields,)* }
        
        // Partial Select Model - all fields are optional for selective querying
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #select_model_partial { #(#partial_select_fields,)* }

        impl #select_model_partial {
            // Convenience methods for setting fields
            #(#partial_convenience_methods)*
        }

        // Implement SQLPartial trait for SelectModel
        impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            type Partial = #select_model_partial;
        }
        
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_partial {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                // Only include columns that are Some() for selective querying
                let mut selected_columns = Vec::new();
                #(
                    if self.#select_field_names.is_some() {
                        selected_columns.push(#select_column_names);
                    }
                )*
                
                if selected_columns.is_empty() {
                    // If no fields selected, default to all columns
                    const ALL_COLUMNS: &'static [&'static str] = &[#(#select_column_names,)*];
                    ::drizzle_rs::core::SQL::columns(ALL_COLUMNS)
                } else {
                    ::drizzle_rs::core::SQL::columns(&selected_columns)
                }
            }
        }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                // Generate column list for SELECT
                const COLUMN_NAMES: &'static [&'static str] = &[#(#select_column_names,)*];
                ::drizzle_rs::core::SQL::columns(COLUMN_NAMES)
            }
        }
    })
}

/// Generates the Insert model with convenience methods and constructor
fn generate_insert_model(ctx: &MacroContext) -> Result<TokenStream> {
    let insert_model = &ctx.insert_model_ident;

    let mut insert_fields = Vec::new();
    let mut insert_default_fields = Vec::new();
    let mut insert_field_conversions = Vec::new();
    let mut insert_column_names = Vec::new();
    let mut insert_convenience_methods = Vec::new();
    let mut constructor_params = Vec::new();
    let mut constructor_assignments = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let base_type = info.base_type;
        let field_type = ctx.get_field_type_for_model(info, ModelType::Insert);

        // Generate field definition
        insert_fields.push(quote! { pub #name: #field_type });

        // Generate default value
        insert_default_fields.push(ctx.get_insert_default_value(info));

        // Generate field conversion for ToSQL (skip autoincrement primary keys)
        if !ctx.should_skip_field_in_insert(info) {
            let column_name = &info.column_name;
            insert_column_names.push(quote! { #column_name });
            insert_field_conversions.push(ctx.get_insert_field_conversion(info));
        }

        // Generate convenience methods
        if ctx.should_generate_convenience_method(info) {
            insert_convenience_methods.push(
                ConvenienceMethodGenerator::generate_method(info, ModelType::Insert)
            );
        }

        // Generate constructor parameters
        let (param, assignment) = ConstructorGenerator::generate_param_and_assignment(info);
        if !param.is_empty() {
            constructor_params.push(param);
            constructor_assignments.push(assignment);
        }
    }

    Ok(quote! {
        // Insert Model
        #[derive(Debug, Clone, PartialEq)]
        pub struct #insert_model {
            #(#insert_fields,)*
        }
        
        impl Default for #insert_model {
            fn default() -> Self { 
                Self { #(#insert_default_fields,)* } 
            }
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
                let mut values = Vec::new();
                #(values.push(#insert_field_conversions);)*
                ::drizzle_rs::core::SQL::parameters(values)
            }
        }
    })
}

/// Generates the Update model with convenience methods
fn generate_update_model(ctx: &MacroContext) -> Result<TokenStream> {
    let update_model = &ctx.update_model_ident;

    let mut update_fields = Vec::new();
    let mut update_field_conversions = Vec::new();
    let mut update_column_names = Vec::new();
    let mut update_field_names = Vec::new();
    let mut update_convenience_methods = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let update_type = info.get_update_type();
        let column_name = &info.column_name;

        // Generate field definition
        update_fields.push(quote! { pub #name: #update_type });

        // Generate field conversion for ToSQL
        update_column_names.push(quote! { #column_name });
        update_field_names.push(name);
        update_field_conversions.push(ctx.get_update_field_conversion(info));

        // Generate convenience methods
        update_convenience_methods.push(
            ConvenienceMethodGenerator::generate_method(info, ModelType::Update)
        );
    }

    Ok(quote! {
        // Update Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #update_model { 
            #(#update_fields,)* 
        }
        
        impl #update_model {
            // Convenience methods for setting fields
            #(#update_convenience_methods)*
        }
        
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                let mut assignments = Vec::new();
                #(#update_field_conversions)*
                ::drizzle_rs::core::SQL::assignments(assignments)
            }
        }
    })
}

/// Generates SQLModel trait implementations for all model types
fn generate_model_trait_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let (select_model, select_model_partial, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.select_model_partial_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    // Collect column information for each model type
    let mut select_column_names = Vec::new();
    let mut select_field_names = Vec::new();
    let mut insert_column_names = Vec::new();
    let mut insert_field_conversions = Vec::new();
    let mut update_column_names = Vec::new();
    let mut update_field_names = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let column_name = &info.column_name;

        // Select model columns
        select_column_names.push(quote! { #column_name });
        select_field_names.push(name);

        // Insert model columns (skip autoincrement primary keys)
        if !ctx.should_skip_field_in_insert(info) {
            insert_column_names.push(quote! { #column_name });
            insert_field_conversions.push(ctx.get_insert_field_conversion(info));
        }

        // Update model columns
        update_column_names.push(quote! { #column_name });
        update_field_names.push(name);
    }

    Ok(quote! {
        // SQLModel implementations
        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                const COLUMN_NAMES: &'static [&'static str] = &[#(#select_column_names,)*];
                ::drizzle_rs::core::SQL::columns(COLUMN_NAMES)
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                ::drizzle_rs::core::SQL::raw("*")
            }
        }

        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                const COLUMN_NAMES: &'static [&'static str] = &[#(#insert_column_names,)*];
                ::drizzle_rs::core::SQL::columns(COLUMN_NAMES)
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                let mut values = Vec::new();
                #(values.push(#insert_field_conversions);)*
                ::drizzle_rs::core::SQL::parameters(values)
            }
        }

        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                const COLUMN_NAMES: &'static [&'static str] = &[#(#update_column_names,)*];
                ::drizzle_rs::core::SQL::columns(COLUMN_NAMES)
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                let mut values = Vec::new();
                // For Update model, only include values that are Some()
                #(
                    if let Some(val) = &self.#update_field_names {
                        values.push(val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null));
                    }
                )*
                ::drizzle_rs::core::SQL::parameters(values)
            }
        }
        
        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_partial {
            fn columns(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                // Only include columns that are Some() for selective querying
                let mut selected_columns = Vec::new();
                #(
                    if self.#select_field_names.is_some() {
                        selected_columns.push(#select_column_names);
                    }
                )*
                
                if selected_columns.is_empty() {
                    // If no fields selected, default to all columns
                    const ALL_COLUMNS: &'static [&'static str] = &[#(#select_column_names,)*];
                    ::drizzle_rs::core::SQL::columns(ALL_COLUMNS)
                } else {
                    ::drizzle_rs::core::SQL::columns(&selected_columns)
                }
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                ::drizzle_rs::core::SQL::raw("*")
            }
        }
    })
}

/// Generates `FromSql` and `ToSql` impls for JSON fields.
fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    // Create a filter for JSON fields
    let json_fields: Vec<_> = ctx.field_infos.iter().filter(|info| info.is_json).collect();

    // If no JSON fields, return an empty TokenStream
    if json_fields.is_empty() {
        return Ok(quote!());
    }

    let json_impls = json_fields.iter()
        .map(|info| {
            if info.is_json && !cfg!(feature = "serde") {
                return Err(syn::Error::new_spanned(
                    info.ident, 
                    format!("The 'serde' feature must be enabled to use JSON fields.\n\
                     Add to Cargo.toml: drizzle-rs = {{ version = \"*\", features = [\"serde\"] }}\n\
                     See: {SQLITE_JSON_URL}")
                ))
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

    #[cfg(feature = "rusqlite")]
    let impls = generate_rusqlite_from_to_sql(&json_fields)?;
    
    #[cfg(not(feature = "rusqlite"))]
    let impls = vec![];

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
            "The #[SQLiteTable] attribute can only be applied to struct definitions.\n",
        ));
    };

    let primary_key_count = fields
        .iter()
        .filter(|f| FieldInfo::from_field(f, false).is_ok_and(|f| f.is_primary))
        .count();
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
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

    let ctx = MacroContext {
        struct_ident,
        table_name,
        create_table_sql,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        without_rowid: attrs.without_rowid,
        strict: attrs.strict,
    };

    // -------------------
    // 2. Generation Phase
    // -------------------
    let (column_definitions, column_zst_idents) = generate_column_definitions(&ctx)?;
    let column_fields = generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors =
        generate_column_accessors(&ctx, &column_zst_idents)?;
    let table_impls = generate_table_impls(&ctx, &column_zst_idents)?;
    let model_definitions = generate_model_definitions(&ctx)?;
    let json_impls = generate_json_impls(&ctx)?;

    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = rusqlite::generate_rusqlite_impls(&ctx)?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote!();

    // Generate compile-time validation for default literals
    let default_validations = generate_default_validations(&field_infos);

    // -------------------
    // 3. Assembly Phase
    // -------------------
    Ok(quote! {
        // Compile-time validation for default literals
        #default_validations

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct #struct_ident {
         #column_fields   
        }

        #column_accessors
        #column_definitions
        #table_impls
        #model_definitions
        #json_impls
        #rusqlite_impls
    })
}
