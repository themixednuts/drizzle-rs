#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "turso")]
pub mod turso;

use super::field::FieldInfo;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::spanned::Spanned;
use syn::Visibility;
use syn::{Data, DeriveInput, Expr, Meta, Result, parse::Parse};

// Common SQLite documentation URLs for error messages and macro docs
const SQLITE_JSON_URL: &str = "https://sqlite.org/json1.html";

// Enhanced context struct to hold all the necessary information for generation.
// This provides helper methods to reduce code duplication and improve maintainability.
pub(crate) struct MacroContext<'a> {
    struct_ident: &'a Ident,
    struct_vis: &'a Visibility,
    table_name: String,
    create_table_sql: String,
    create_table_sql_runtime: Option<TokenStream>, // For tables with foreign keys
    field_infos: &'a [FieldInfo<'a>],
    select_model_ident: Ident,
    select_model_partial_ident: Ident,
    insert_model_ident: Ident,
    update_model_ident: Ident,
    without_rowid: bool,
    strict: bool,
    has_foreign_keys: bool,
}

impl<'a> MacroContext<'a> {
    // ============================================================================
    // Core Field Analysis Methods - Single Source of Truth
    // ============================================================================

    /// Determines if a field can auto-increment (INTEGER PRIMARY KEY in regular tables, excluding enums)
    fn can_field_autoincrement(&self, field: &FieldInfo) -> bool {
        if !field.is_primary || self.without_rowid || field.is_enum {
            return false;
        }
        
        use crate::sqlite::field::SQLiteType;
        matches!(field.column_type, SQLiteType::Integer)
    }


    /// Determines if a field should be optional in the Insert model
    fn is_field_optional_in_insert(&self, field: &FieldInfo) -> bool {
        // Nullable fields are always optional
        if field.is_nullable {
            return true;
        }
        
        // Fields with explicit defaults (SQL or runtime) are optional  
        if field.has_default || field.default_fn.is_some() {
            return true;
        }
        
        // Primary key fields that can auto-increment are optional
        self.can_field_autoincrement(field)
    }


    /// Gets the appropriate field type for a specific model
    fn get_field_type_for_model(&self, field: &FieldInfo, model_type: ModelType) -> TokenStream {
        let base_type = field.base_type;
        match model_type {
            ModelType::Insert => {
                // All insert fields use InsertValue for three-state handling
                quote!(::drizzle_rs::sqlite::InsertValue<#base_type>)
            },
            ModelType::Update => quote!(Option<#base_type>),
            ModelType::PartialSelect => quote!(Option<#base_type>),
        }
    }


    /// Gets the default value expression for insert model
    fn get_insert_default_value(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        
        // Handle runtime function defaults (default_fn)  
        if let Some(f) = &field.default_fn {
            return quote! { #name: ::drizzle_rs::sqlite::InsertValue::Value((#f)()) };
        }
        
        // Handle compile-time SQL defaults (default = literal) or any other case
        // Default to Omit so database can handle defaults
        quote! { #name: ::drizzle_rs::sqlite::InsertValue::Omit }
    }

    /// Generates field conversion for insert ToSQL
    fn get_insert_field_conversion(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        
        let value_conversion = if field.is_enum {
            quote! { val.clone().into() }
        } else {
            quote! { val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null) }
        };
        
        // Handle the three states of InsertValue
        if field.default_fn.is_some() {
            // For runtime defaults, we always include the field (either default or user value)
            quote! {
                match &self.#name {
                    ::drizzle_rs::sqlite::InsertValue::Omit => {
                        // Use runtime default for omitted values
                        let default_val = self.#name.clone(); // This should never be Omit due to default logic
                        #value_conversion
                    },
                    ::drizzle_rs::sqlite::InsertValue::Null => ::drizzle_rs::sqlite::SQLiteValue::Null,
                    ::drizzle_rs::sqlite::InsertValue::Value(val) => #value_conversion,
                }
            }
        } else {
            // For compile-time defaults or no defaults, we may omit the field
            quote! {
                match &self.#name {
                    ::drizzle_rs::sqlite::InsertValue::Omit => {
                        // This field will be omitted from the column list entirely
                        continue;
                    },
                    ::drizzle_rs::sqlite::InsertValue::Null => ::drizzle_rs::sqlite::SQLiteValue::Null,
                    ::drizzle_rs::sqlite::InsertValue::Value(val) => #value_conversion,
                }
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
        
        // Default conversion for all other fields (including enums with generated From implementations)
        let conversion = if field.is_enum {
            quote! { val.clone().into() }
        } else {
            quote! { val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null) }
        };
        
        quote! {
            if let Some(val) = &self.#name {
                assignments.push((#column_name, #conversion));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ModelType {
    Insert,
    Update,
    PartialSelect,
}

/// Helper struct for generating convenience methods in a DRY manner
struct ConvenienceMethodGenerator;

impl ConvenienceMethodGenerator {
    /// Generates a convenience method for a field based on its type
    fn generate_method(field: &FieldInfo, model_type: ModelType, _ctx: &MacroContext) -> TokenStream {
        let field_name = field.ident;
        let base_type = field.base_type;
        let method_name = format_ident!("with_{}", field_name);

        let assignment = match model_type {
            ModelType::Insert => quote! { self.#field_name = value.into(); },
            ModelType::Update => quote! { self.#field_name = Some(value); },
            ModelType::PartialSelect => quote! { self.#field_name = Some(value); },
        };

        // Generate type-specific convenience methods using modern pattern matching
        match model_type {
            ModelType::Insert => {
                // For insert models, use Into<InsertValue<T>> for clean API
                let type_string = base_type.to_token_stream().to_string();
                match (field.is_uuid, type_string.as_str()) {
                    (true, _) => quote! {
                        pub fn #method_name<V: Into<::drizzle_rs::sqlite::InsertValue<::uuid::Uuid>>>(mut self, value: V) -> Self {
                            #assignment
                            self
                        }
                    },
                    (_, s) if s.contains("String") => quote! {
                        pub fn #method_name<V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>>(mut self, value: V) -> Self {
                            #assignment
                            self
                        }
                    },
                    (_, s) if s.contains("Vec") && s.contains("u8") => quote! {
                        pub fn #method_name<V: Into<::drizzle_rs::sqlite::InsertValue<::std::vec::Vec<u8>>>>(mut self, value: V) -> Self {
                            #assignment
                            self
                        }
                    },
                    _ => quote! {
                        pub fn #method_name<V: Into<::drizzle_rs::sqlite::InsertValue<#base_type>>>(mut self, value: V) -> Self {
                            #assignment
                            self
                        }
                    },
                }
            },
            _ => {
                // For other models, keep the existing logic
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
    }
}

/// Helper struct for generating constructor parameters in a DRY manner
struct ConstructorGenerator;

impl ConstructorGenerator {

    /// Generates constructor parameter and assignment for required fields only
    fn generate_required_param_and_assignment(field: &FieldInfo, _ctx: &MacroContext) -> (TokenStream, TokenStream) {
        let field_name = field.ident;
        let base_type = field.base_type;
        let type_string = base_type.to_token_stream().to_string();

        // Required parameters - convert directly to InsertValue::Value
        match (field.is_uuid, type_string.as_str()) {
            (true, _) => (
                quote! { #field_name: impl Into<::uuid::Uuid> },
                quote! { #field_name: ::drizzle_rs::sqlite::InsertValue::Value(#field_name.into()) }
            ),
            (false, s) if s.contains("String") => (
                quote! { #field_name: impl Into<::std::string::String> },
                quote! { #field_name: ::drizzle_rs::sqlite::InsertValue::Value(#field_name.into()) }
            ),
            (false, s) if s.contains("Vec") && s.contains("u8") => (
                quote! { #field_name: impl Into<::std::vec::Vec<u8>> },
                quote! { #field_name: ::drizzle_rs::sqlite::InsertValue::Value(#field_name.into()) }
            ),
            (false, _) => (
                quote! { #field_name: #base_type },
                quote! { #field_name: ::drizzle_rs::sqlite::InsertValue::Value(#field_name) }
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

/// Generates runtime code to build CREATE TABLE SQL with foreign key support.
fn generate_create_table_sql_runtime(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> TokenStream {
    let column_defs: Vec<TokenStream> = field_infos
        .iter()
        .map(|info| {
            let base_def = &info.sql_definition;
            
            if let Some(ref fk) = info.foreign_key {
                // Generate runtime code to build foreign key constraint
                let table_ident = &fk.table_ident;
                let column_ident = &fk.column_ident;
                
                quote! {
                    {
                        let base_def = #base_def;
                        let table_name = #table_ident::NAME.to_string();
                        let column_name = #table_ident::#column_ident.name().to_string();
                        format!("{} REFERENCES {}({})", base_def, table_name, column_name)
                    }
                }
            } else {
                quote! { #base_def.to_string() }
            }
        })
        .collect();

    let table_name_str = table_name;
    let composite_pk_code = if is_composite_pk {
        let pk_columns: Vec<&String> = field_infos
            .iter()
            .filter(|info| info.is_primary)
            .map(|info| &info.column_name)
            .collect();
        
        quote! {
            column_defs_str.push_str(", PRIMARY KEY (");
            column_defs_str.push_str(&[#(#pk_columns),*].join(", "));
            column_defs_str.push_str(")");
        }
    } else {
        quote! {}
    };

    let without_rowid_code = if without_rowid {
        quote! { sql.push_str(" WITHOUT ROWID"); }
    } else {
        quote! {}
    };

    let strict_code = if strict {
        quote! { sql.push_str(" STRICT"); }
    } else {
        quote! {}
    };

    quote! {
        {
            let column_defs = vec![#(#column_defs),*];
            let mut column_defs_str = column_defs.join(", ");
            #composite_pk_code
            let mut sql = format!("CREATE TABLE \"{}\" ({})", #table_name_str, column_defs_str);
            #without_rowid_code
            #strict_code
            sql.push(';');
            sql
        }
    }
}

/// Generates the static `CREATE TABLE` SQL string (for tables without foreign keys).
fn generate_create_table_sql(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> String {
    let column_defs: Vec<_> = field_infos
        .iter()
        .map(|info| info.sql_definition.clone())
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

    // Don't add extra closing paren since it's already in the format string
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
            pub const fn new() -> Self {
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
    let MacroContext {struct_ident, struct_vis, field_infos, .. } = ctx;


    for info in *field_infos {
        let field_pascal_case = info.ident.to_string().to_upper_camel_case();
        let zst_ident = format_ident!("{}{}", ctx.struct_ident, field_pascal_case);
        column_zst_idents.push(zst_ident.clone());

        let (value_type, rust_type) = (&info.base_type, &info.field_type);
        let (is_primary, is_not_null, is_unique, is_autoincrement, has_default) = (
            info.is_primary,
            !info.is_nullable,
            info.is_unique,
            info.is_autoincrement,
            info.has_default || info.default_fn.is_some(),
        );

        // Only use default_fn for Rust DEFAULT constant, not SQL default literals
        let default_const = quote! { None };

        let default_fn_body = info.default_fn.as_ref().map_or_else(
            || quote! { None::<fn() -> Self::Type> },
            |func| quote! { Some(#func) },
        );

        let sql = &info.sql_definition;

        let name = &info.column_name;
        let col_type = &info.column_type.to_sql_type();

        // Generate direct From implementations for enum fields
        let enum_impl = if info.is_enum {
            let (conversion, reference_conversion) = match info.column_type {
                crate::sqlite::field::SQLiteType::Integer => {
                    (
                        quote! { 
                            let integer: i64 = value.into();
                            ::drizzle_rs::sqlite::SQLiteValue::Integer(integer)
                        },
                        quote! {
                            let integer: i64 = value.into();
                            ::drizzle_rs::sqlite::SQLiteValue::Integer(integer)
                        }
                    )
                },
                crate::sqlite::field::SQLiteType::Text => {
                    (
                        quote! { 
                            let text: &str = value.into();
                            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
                        },
                        quote! {
                            let text: &str = value.into();
                            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
                        }
                    )
                },
                _ =>  return Err(syn::Error::new_spanned(info.ident, "Enum is only supported in text or integer column types")), // Default to Text for other types
            };
            
            {
                let rusqlite_impl = if cfg!(feature = "rusqlite") {
                    match info.column_type {
                        crate::sqlite::field::SQLiteType::Integer => quote! {
                            // rusqlite::FromSql and ToSql for integer enums
                            impl ::rusqlite::types::FromSql for #value_type {
                                fn column_result(value: ::rusqlite::types::ValueRef<'_>) -> ::rusqlite::types::FromSqlResult<Self> {
                                    match value {
                                        ::rusqlite::types::ValueRef::Integer(i) => {
                                            Self::try_from(i).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                                        },
                                        _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                                    }
                                }
                            }
                            
                            impl ::rusqlite::types::ToSql for #value_type {
                                fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                                    let val: i64 = self.into();
                                    Ok(::rusqlite::types::ToSqlOutput::Owned(::rusqlite::types::Value::Integer(val)))
                                }
                            }
                        },
                        crate::sqlite::field::SQLiteType::Text => quote! {
                            // rusqlite::FromSql and ToSql for text enums
                            impl ::rusqlite::types::FromSql for #value_type {
                                fn column_result(value: ::rusqlite::types::ValueRef<'_>) -> ::rusqlite::types::FromSqlResult<Self> {
                                    match value {
                                        ::rusqlite::types::ValueRef::Text(s) => {
                                            let s_str = ::std::str::from_utf8(s).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)?;
                                            Self::try_from(s_str).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                                        },
                                        _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                                    }
                                }
                            }
                            
                            impl ::rusqlite::types::ToSql for #value_type {
                                fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                                    let val: &str = self.into();
                                    Ok(::rusqlite::types::ToSqlOutput::Borrowed(::rusqlite::types::ValueRef::Text(val.as_bytes())))
                                }
                            }
                        },
                        _ => quote! {}
                    }
                } else {
                    quote! {}
                };

                quote! {
                    // Generate From implementations for enum values
                    impl<'a> ::std::convert::From<#value_type> for ::drizzle_rs::sqlite::SQLiteValue<'a> {
                        fn from(value: #value_type) -> Self {
                            #conversion
                        }
                    }

                    impl<'a> ::std::convert::From<&'a #value_type> for ::drizzle_rs::sqlite::SQLiteValue<'a> {
                        fn from(value: &'a #value_type) -> Self {
                            #reference_conversion
                        }
                    }

                    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #value_type {
                        fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                            let value = self;
                            #conversion.into()
                        }
                    }
                    

                    // Include rusqlite implementations
                    #rusqlite_impl
                }
            }
        } else {
            quote! {}
        };

        let column_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #zst_ident;

            impl #zst_ident {
                pub const fn new() -> #zst_ident {
                    #zst_ident {}
                }
            }

            impl <'a> ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a> > for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(#sql);
            }

            impl ::drizzle_rs::core::SQLColumnInfo for #zst_ident {

                fn name(&self) -> &str {
                    Self::NAME
                }
                fn r#type(&self) -> &str {
                    Self::TYPE
                }
                fn is_primary_key(&self) -> bool {
                    Self::PRIMARY_KEY
                }
                fn is_not_null(&self) -> bool {
                    Self::NOT_NULL
                }
                fn is_unique(&self) -> bool {
                    Self::UNIQUE
                }
                fn has_default(&self) -> bool {
                    #has_default
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
            
            // Include enum implementation if this is an enum field
            #enum_impl
        };
        all_column_code.extend(column_code);
    }
    Ok((all_column_code, column_zst_idents))
}

/// Generates the `SQLSchema` and `SQLTable` implementations.
fn generate_table_impls(ctx: &MacroContext, column_zst_idents: &[Ident]) -> Result<TokenStream> {
    let MacroContext { strict, without_rowid, .. } = &ctx;
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    

    // Generate SQL implementation based on whether table has foreign keys
    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        if let Some(ref runtime_sql) = ctx.create_table_sql_runtime {
            (
                quote! {
                    const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text("-- Runtime SQL generation required");
                },
                quote! {
                    fn sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                        let runtime_sql = #runtime_sql;
                        ::drizzle_rs::core::SQL::raw(runtime_sql)
                    }
                }
            )
        } else {
            // Fallback to static SQL
            let create_table_sql = &ctx.create_table_sql;
            (
                quote! {
                    const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(#create_table_sql);
                },
                quote! {}
            )
        }
    } else {
        // Use static SQL for tables without foreign keys
        let create_table_sql = &ctx.create_table_sql;
        (
            quote! {
                const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(#create_table_sql);
            },
            quote! {}
        )
    };


    Ok(quote! {
        impl<'a> ::drizzle_rs::core::SQLSchema<'a, ::drizzle_rs::core::SQLSchemaType, ::drizzle_rs::sqlite::SQLiteValue<'a> > for #struct_ident {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
            #sql_const
            #sql_method
        }

        impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_ident {
            type Select = #select_model;
            type Insert = #insert_model;
            type Update = #update_model;
        }

        impl ::drizzle_rs::core::SQLTableInfo for #struct_ident {
            fn name(&self) -> &str {
                Self::NAME
            }
            fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
                Self::TYPE
            }
            fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            
                Box::new([#(#column_zst_idents.as_column(),)*])
            }
            fn strict(&self) -> bool {
                #strict
            }
            fn without_rowid(&self) -> bool {
                #without_rowid
            }
        }

        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
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
                let base_type_tokens = &info.base_type; // already a syn::Type
                let base_type: proc_macro2::TokenStream = if base_type_tokens.to_token_stream().to_string() == "String" {
                    quote! { &str }
                } else {
                    quote! { #base_type_tokens }
                };
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
fn generate_model_definitions(ctx: &MacroContext, column_zst_idents: &[Ident]) -> Result<TokenStream> {
    let select_model = generate_select_model(ctx)?;
    let insert_model = generate_insert_model(ctx)?;
    let update_model = generate_update_model(ctx)?;
    let model_impls = generate_model_trait_impls(ctx, column_zst_idents)?;

    Ok(quote! {
        #select_model
        #insert_model  
        #update_model
        #model_impls
    })
}

/// Generates the Select model and its partial variant
fn generate_select_model(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {select_model_ident, select_model_partial_ident, struct_vis, field_infos,  ..} = ctx;

    let mut select_fields = Vec::new();
    let mut partial_select_fields = Vec::new();
    let mut select_column_names = Vec::new();
    let mut select_field_names = Vec::new();
    let mut partial_convenience_methods = Vec::new();

    for info in *field_infos {
        let name = info.ident;
        let select_type = info.get_select_type();
        let base_type = info.base_type;
        let column_name = &info.column_name;

        select_fields.push(quote! { pub #name: #select_type });
        partial_select_fields.push(quote! { pub #name: Option<#base_type> });
        select_column_names.push(quote! { #column_name });
        select_field_names.push(name);

        // Generate convenience methods for partial select
        partial_convenience_methods.push(
            ConvenienceMethodGenerator::generate_method(info, ModelType::PartialSelect, ctx)
        );
    }

    Ok(quote! {
        // Select Model
        #[derive(Debug, Clone, PartialEq, Default)]
        #struct_vis struct #select_model_ident { #(#select_fields,)* }
        
        // Partial Select Model - all fields are optional for selective querying
        #[derive(Debug, Clone, PartialEq, Default)]
        #struct_vis struct #select_model_partial_ident { #(#partial_select_fields,)* }

        impl #select_model_partial_ident {
            // Convenience methods for setting fields
            #(#partial_convenience_methods)*
        }

        // Implement SQLPartial trait for SelectModel
        impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_ident {
            type Partial = #select_model_partial_ident;
        }
        
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_partial_ident {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                unimplemented!()
                // Only include columns that are Some() for selective querying
                // let mut selected_columns = Vec::new();
                // #(
                //     if self.#select_field_names.is_some() {
                //         selected_columns.push(#select_column_names);
                //     }
                // )*
                
                // if selected_columns.is_empty() {
                //     unimplemented!()
                //     // If no fields selected, default to all columns
                //     // const ALL_COLUMNS: &'static [&'static str] = &[#(#select_column_names,)*];
                //     // ::drizzle_rs::core::SQL::join(ALL_COLUMNS, ", ")
                // } else {
                //     ::drizzle_rs::core::SQL::join(&selected_columns, ", ")
                // }
            }
        }
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_ident {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                unimplemented!()
                // Generate column list for SELECT
                // const COLUMN_NAMES: &'static [&'static str] = &[#(#select_column_names,)*];
                // ::drizzle_rs::core::SQL::join(COLUMN_NAMES, ", ")
            }
        }
    })
}

/// Generates the Insert model with convenience methods and constructor
fn generate_insert_model(ctx: &MacroContext) -> Result<TokenStream> {
    let insert_model = &ctx.insert_model_ident;
    let struct_ident = &ctx.struct_ident;

    let mut insert_fields = Vec::new();
    let mut insert_default_fields = Vec::new();
    let mut insert_field_conversions = Vec::new();
    let mut insert_column_names = Vec::new();
    let mut insert_field_names = Vec::new();
    let mut insert_field_indices = Vec::new();
    let mut insert_convenience_methods = Vec::new();
    
    // Separate required and optional fields for constructor
    let mut required_constructor_params = Vec::new();
    let mut required_constructor_assignments = Vec::new();

    for (field_index, info) in ctx.field_infos.iter().enumerate() {
        let name = info.ident;
        let field_type = ctx.get_field_type_for_model(info, ModelType::Insert);
        let is_optional = ctx.is_field_optional_in_insert(info);

        // Generate field definition
        insert_fields.push(quote! { pub #name: #field_type });

        // Generate default value
        insert_default_fields.push(ctx.get_insert_default_value(info));

        // Generate field conversion for ToSQL
        let column_name = &info.column_name;
        insert_column_names.push(quote! { #column_name });
        insert_field_names.push(name);
        insert_field_indices.push(quote! { #field_index });
        insert_field_conversions.push(ctx.get_insert_field_conversion(info));

        insert_convenience_methods.push(
            ConvenienceMethodGenerator::generate_method(info, ModelType::Insert, ctx)
        );

        // Generate constructor parameters only for required fields
        if !is_optional {
            let (param, assignment) = ConstructorGenerator::generate_required_param_and_assignment(info, ctx);
            required_constructor_params.push(param);
            required_constructor_assignments.push(assignment);
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
            pub fn new(#(#required_constructor_params),*) -> Self {
                Self {
                    #(#required_constructor_assignments,)*
                    ..Self::default()
                }
            }

            // Convenience methods for setting fields
            #(#insert_convenience_methods)*
        }
        
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                let mut values = Vec::new();
                
                // Process each field and add to values if not omitted
                #(
                    match &self.#insert_field_names {
                        ::drizzle_rs::sqlite::InsertValue::Omit => {
                            // Skip omitted fields - they won't be included in INSERT
                        },
                        ::drizzle_rs::sqlite::InsertValue::Null => {
                            values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                        },
                        ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                            values.push(val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null));
                        },
                    }
                )*
                
                ::drizzle_rs::core::SQL::parameters(values)
            }
        }

        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model {
            fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                // For insert model, return only non-omitted columns to match values()
                static TABLE: #struct_ident = #struct_ident::new();
                let all_columns = TABLE.columns();
                let mut result_columns = Vec::new();
                
                #(
                    if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.#insert_field_names {
                        // Skip omitted fields
                    } else {
                        // Include this column
                        result_columns.push(all_columns[#insert_field_indices]);
                    }
                )*
                
                result_columns.into_boxed_slice()
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                let mut values = Vec::new();
                
                #(
                    match &self.#insert_field_names {
                        ::drizzle_rs::sqlite::InsertValue::Omit => {
                            // Skip omitted fields
                        }
                        ::drizzle_rs::sqlite::InsertValue::Null => {
                            values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                        }
                        ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                            values.push(val.clone().try_into().unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null));
                        }
                    }
                )*
                
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
            ConvenienceMethodGenerator::generate_method(info, ModelType::Update, ctx)
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
fn generate_model_trait_impls(ctx: &MacroContext, _column_zst_idents: &[Ident]) -> Result<TokenStream> {
    let (select_model, select_model_partial, update_model) = (
        &ctx.select_model_ident,
        &ctx.select_model_partial_ident,
        &ctx.update_model_ident,
    );

    let struct_ident = &ctx.struct_ident;

    // Collect field names for update model
    let mut update_field_names = Vec::new();

    for info in ctx.field_infos.iter() {
        let name = info.ident;
        update_field_names.push(name);
    }

    Ok(quote! {
        // SQLModel implementations
        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model {
            fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                // For select model, return all columns
                static INSTANCE: #struct_ident = #struct_ident::new();
                INSTANCE.columns()
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                ::drizzle_rs::core::SQL::empty()
            }
        }

        impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                // For update model, return all columns (same as other models)
                static INSTANCE: #struct_ident = #struct_ident::new();
                INSTANCE.columns()
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
            fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                // For partial select model, return all columns (same as other models)
                static INSTANCE: #struct_ident = #struct_ident::new();
                INSTANCE.columns()
            }

            fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                ::drizzle_rs::core::SQL::empty()
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

    // Check that serde feature is enabled for JSON fields
    if !cfg!(feature = "serde") {
        let first_json_field = json_fields.first().unwrap();
        return Err(syn::Error::new_spanned(
            first_json_field.ident, 
            format!("The 'serde' feature must be enabled to use JSON fields.\n\
             Add to Cargo.toml: drizzle-rs = {{ version = \"*\", features = [\"serde\"] }}\n\
             See: {SQLITE_JSON_URL}")
        ));
    }

    // Track JSON type to SQLite storage type mapping and detect conflicts
    use std::collections::HashMap;
    use crate::sqlite::field::SQLiteType;
    
    let mut json_type_storage: HashMap<String, (SQLiteType, &FieldInfo)> = HashMap::new();

    // Check for conflicts and build the mapping
    for info in json_fields {
        let base_type_str = info.base_type.to_token_stream().to_string();
        
        if let Some((existing_storage, existing_field)) = json_type_storage.get(&base_type_str) {
            // Check if the storage type conflicts
            if *existing_storage != info.column_type {
                return Err(syn::Error::new_spanned(
                    info.ident,
                    format!(
                        "JSON type '{}' is used with conflicting storage types. \
                         Field '{}' uses {:?}, but field '{}' uses {:?}. \
                         Each JSON type must use the same storage type (either TEXT or BLOB) throughout the codebase.",
                        base_type_str,
                        existing_field.ident,
                        existing_storage,
                        info.ident,
                        info.column_type
                    )
                ));
            }
        } else {
            // First occurrence of this JSON type
            json_type_storage.insert(base_type_str, (info.column_type.clone(), info));
        }
    }

    // Generate core SQLiteValue implementations (needed for all drivers)
    let core_impls = if json_type_storage.is_empty() { 
        vec![] 
    } else {
        json_type_storage.iter().map(|(_, (storage_type, info))| {
            let struct_name = info.base_type;
            let core_conversion = match storage_type {
                SQLiteType::Text => quote! {
                    let json = serde_json::to_string(&self)?;
                    Ok(::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Owned(json)))
                },
                SQLiteType::Blob => quote! {
                    let json = serde_json::to_vec(&self)?;
                    Ok(::drizzle_rs::sqlite::SQLiteValue::Blob(::std::borrow::Cow::Owned(json)))
                },
                _ => return Err(syn::Error::new_spanned(
                    info.ident, 
                    "JSON fields must use either TEXT or BLOB column types"
                )),
            };

            Ok(quote! {
                // Core TryInto implementation for SQLiteValue (needed for all drivers)
                impl<'a> ::std::convert::TryInto<::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_name {
                    type Error = serde_json::Error;

                    fn try_into(self) -> Result<::drizzle_rs::sqlite::SQLiteValue<'a>, Self::Error> {
                        #core_conversion
                    }
                }
            })
        }).collect::<Result<Vec<_>>>()?
    };

    // Generate rusqlite-specific implementations
    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = if json_type_storage.is_empty() { 
        vec![] 
    } else {
        json_type_storage.iter().map(|(_, (storage_type, info))| {
            let struct_name = info.base_type;
            let (from_impl, to_impl) = match storage_type {
                SQLiteType::Text => (
                    quote! {
                        match value {
                            rusqlite::types::ValueRef::Text(items) => serde_json::from_slice(items)
                                .map_err(|_| rusqlite::types::FromSqlError::InvalidType),
                            _ => Err(rusqlite::types::FromSqlError::InvalidType),
                        }
                    },
                    quote! {
                        let json = serde_json::to_string(self)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                        Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(json)))
                    }
                ),
                SQLiteType::Blob => (
                    quote! {
                        match value {
                            rusqlite::types::ValueRef::Blob(items) => serde_json::from_slice(items)
                                .map_err(|_| rusqlite::types::FromSqlError::InvalidType),
                            _ => Err(rusqlite::types::FromSqlError::InvalidType),
                        }
                    },
                    quote! {
                        let json = serde_json::to_vec(self)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                        Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Blob(json)))
                    }
                ),
                _ => return Err(syn::Error::new_spanned(
                    info.ident, 
                    "JSON fields must use either TEXT or BLOB column types"
                )),
            };

            Ok(quote! {
                impl rusqlite::types::FromSql for #struct_name {
                    fn column_result(
                        value: rusqlite::types::ValueRef<'_>,
                    ) -> rusqlite::types::FromSqlResult<Self> {
                        #from_impl
                    }
                }

                impl rusqlite::types::ToSql for #struct_name {
                    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                        #to_impl
                    }
                }
            })
        }).collect::<Result<Vec<_>>>()?
    };
    
    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls: Vec<TokenStream> = vec![];

    let json_types_impl = quote! {
        #(#core_impls)*
        #(#rusqlite_impls)*
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
    let struct_vis = &input.vis;
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

    // Check if any field has foreign keys
    let has_foreign_keys = field_infos.iter().any(|info| info.foreign_key.is_some());
    
    let (create_table_sql, create_table_sql_runtime) = if has_foreign_keys {
        // For tables with foreign keys, generate runtime SQL generation code
        let runtime_sql = generate_create_table_sql_runtime(
            &table_name,
            &field_infos,
            is_composite_pk,
            attrs.strict,
            attrs.without_rowid,
        );
        // Provide a placeholder static SQL for compile-time usage
        ("-- Runtime SQL generation required for foreign keys".to_string(), Some(runtime_sql))
    } else {
        // For tables without foreign keys, use static SQL generation
        let static_sql = generate_create_table_sql(
            &table_name,
            &field_infos,
            is_composite_pk,
            attrs.strict,
            attrs.without_rowid,
        );
        (static_sql, None)
    };

    let ctx = MacroContext {
        struct_ident,
        struct_vis: &input.vis,
        table_name,
        create_table_sql,
        create_table_sql_runtime,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        without_rowid: attrs.without_rowid,
        strict: attrs.strict,
        has_foreign_keys,
    };

    // -------------------
    // 2. Generation Phase
    // -------------------
    let (column_definitions, column_zst_idents) = generate_column_definitions(&ctx)?;
    let column_fields = generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors =
        generate_column_accessors(&ctx, &column_zst_idents)?;
    let table_impls = generate_table_impls(&ctx, &column_zst_idents)?;
    let model_definitions = generate_model_definitions(&ctx, &column_zst_idents)?;
    let json_impls = generate_json_impls(&ctx)?;

    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = rusqlite::generate_rusqlite_impls(&ctx)?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote!();

    #[cfg(feature = "turso")]
    // let turso_impls = quote! {};
    let turso_impls = turso::generate_turso_impls(&ctx)?;

    #[cfg(not(feature = "turso"))]
    let turso_impls = quote!();

    // Generate compile-time validation for default literals
    let default_validations = generate_default_validations(&field_infos);

    // -------------------
    // 3. Assembly Phase
    // -------------------
    Ok(quote! {
        // Compile-time validation for default literals
        #default_validations

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
         #struct_vis struct #struct_ident {
         #column_fields   
        }

        #column_accessors
        #column_definitions
        #table_impls
        #model_definitions
        #json_impls
        #rusqlite_impls
        #turso_impls
    })
}
