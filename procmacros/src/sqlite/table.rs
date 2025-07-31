#[cfg(feature = "rusqlite")]
pub mod rusqlite;

use super::field::FieldInfo;
use super::field::SQLiteType;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use rusqlite::generate_rusqlite_from_to_sql;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, Meta, Result, Type};

#[derive(Default)]
pub(crate) struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) strict: bool,
    pub(crate) without_rowid: bool,
}

impl Parse for TableAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = TableAttributes::default();

        // If the input is empty, return the default attributes
        if input.is_empty() {
            return Ok(attrs);
        }

        // Parse a comma-separated list of attributes
        let punctuated = input.parse_terminated(syn::Meta::parse, syn::Token![,])?;

        // Process each attribute
        for meta in punctuated {
            match meta {
                // Handle name="value" attribute
                syn::Meta::NameValue(name_value) if name_value.path.is_ident("name") => {
                    if let syn::Expr::Lit(expr_lit) = name_value.value {
                        if let syn::Lit::Str(lit_str) = expr_lit.lit {
                            attrs.name = Some(lit_str.value());
                        }
                    }
                }
                // Handle "strict" flag
                syn::Meta::Path(path) if path.is_ident("strict") => {
                    attrs.strict = true;
                }
                // Handle "without_rowid" flag
                syn::Meta::Path(path) if path.is_ident("without_rowid") => {
                    attrs.without_rowid = true;
                }
                // Ignore unrecognized attributes
                _ => return Err(syn::Error::new(meta.span(), "unrecognized attribute")),
            }
        }

        Ok(attrs)
    }
}

/// Generate SQLite dialect-specific column constants for a table
pub(crate) fn generate_sqlite_column_consts(
    field_info: &[(
        &Ident, // field name
        &Type,  // field type (original, e.g., Option<T>)
        String, // column name
        String, // base SQL definition (from core)
        bool,   // is_autoincrement
        bool,   // is_primary
    )],
) -> Result<TokenStream> {
    // Add validation logic here
    for (field_name, _field_type, _column_name, _sql, is_autoincrement, is_primary) in
        field_info.iter()
    {
        if *is_autoincrement && !is_primary {
            return Err(syn::Error::new_spanned(
                field_name, // Span the error on the field name
                "drizzle: 'autoincrement' can only be assigned to a field that is also 'primary key'.",
            ));
        }
    }

    let column_consts = field_info.iter().map(
        |(field_name, field_type, column_name, sql_base, is_autoincrement, is_primary)| {
            // Add AUTOINCREMENT to SQL definition if needed
            // Note: This assumes single-column PKs for AUTOINCREMENT as multi-col PK logic is in core
            let final_sql = if *is_autoincrement && *is_primary {
                format!("{} AUTOINCREMENT", sql_base)
            } else {
                sql_base.clone()
            };

            quote! {
                #[allow(non_upper_case_globals, dead_code)]
                // Use the original field_type here for the SQLiteColumn generic
                pub const #field_name: ::drizzle_rs::sqlite::SQLiteColumn<'a, #field_type, Self> =
                    ::drizzle_rs::sqlite::SQLiteColumn::new(
                        #column_name,
                        #final_sql, // Use the potentially modified SQL
                    );
            }
        },
    );

    Ok(quote! {
        #(#column_consts)*
    })
}

// --- IntoParams Generation --- //

/// Generates the `impl IntoParams<SQLiteValue>` for the Insert model.
pub(crate) fn generate_sqlite_into_params_impl(
    insert_model_name: &Ident,
    field_info: &[FieldInfo<'_>],
) -> Result<TokenStream> {
    let param_pushes = field_info
        .iter()
        .map(|info| {
            let field_ident = info.ident;
            if !cfg!(feature = "serde") && info.is_json {
                return Err(syn::Error::new(
                    field_ident.span(),
                    "JSON field requires the 'serde' feature to be enabled",
                ));
            }

            if info.is_json {
                // JSON fields now have From implementations, so we can use the standard approach
                Ok(quote! {
                    params.push(::drizzle_rs::sqlite::SQLiteValue::from(self.#field_ident.clone()));
                })
            } else if info.is_enum {
                // For enum fields, we assume the type implements the SQLiteEnum trait
                Ok(quote! {
                    // Directly use the SQLiteEnum trait implementation for conversion
                    params.push(::drizzle_rs::sqlite::SQLiteValue::from(self.#field_ident.clone()));
                })
            } else {
                // Standard handling for non-JSON, non-enum fields
                Ok(quote! {
                    // Assumes the field type implements From/Into SQLiteValue
                    params.push(::drizzle_rs::sqlite::SQLiteValue::from(self.#field_ident.clone())); // Clone needed for Option<T> fields in InsertModel
                })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // The entire impl is generated only if the sqlite feature is on (implicitly via this module)
    Ok(quote! {
        // impl<'a> ::drizzle_rs::core::IntoParams<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #insert_model_name {
        //     fn into_params(self) -> ::std::result::Result<Vec<::drizzle_rs::sqlite::SQLiteValue<'a>>, ::drizzle_rs::core::DrizzleError> {
        //         let mut params = Vec::new();
        //         #(#param_pushes)*
        //         Ok(params)
        //     }
        // }
    })
}

/// Generate TryFrom<SQLiteValue> implementations for JSON fields
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
            let column_type_str = info.column_type_str().unwrap_or_else(|| "text".to_string());

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

/// Implementation of the SQLiteTable attribute macro
pub(crate) fn table_attr_macro(
    input: syn::DeriveInput,
    attrs: TableAttributes,
) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Use our extracted attribute values
    let table_name = attrs.name.unwrap_or_else(|| struct_name.to_string());
    let strict = attrs.strict;
    let without_rowid = attrs.without_rowid;

    // Extract fields directly using FieldInfo::from_field
    let fields_data = if let syn::Data::Struct(ref data) = input.data {
        &data.fields
    } else {
        return Err(syn::Error::new_spanned(
            &input,
            "SQLiteTable can only be applied to structs.",
        ));
    };

    // Collect fields with primary key to determine composite primary keys
    let primary_key_count = fields_data
        .iter()
        .filter(|field| {
            field.attrs.iter().any(|attr| {
                if let Some(ident) = attr.path().get_ident() {
                    if SQLiteType::all_attribute_names().contains(&ident.to_string().as_str()) {
                        if let Ok(meta) = attr.parse_args_with(
                            syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                        ) {
                            return meta.iter().any(|nested_meta| {
                                if let Meta::Path(path) = nested_meta {
                                    if let Some(ident) = path.get_ident() {
                                        return ident.to_string() == "primary_key"
                                            || ident.to_string() == "primary";
                                    }
                                }
                                false
                            });
                        }
                    }
                }
                false
            })
        })
        .count();

    let is_composite_pk = primary_key_count > 1;

    // Parse field information
    let field_infos: Vec<FieldInfo> = fields_data
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<syn::Result<Vec<_>>>()?;

    // Generate column constants - Always call the function
    let column_consts = {
        let sqlite_field_info: Vec<_> = field_infos
            .iter()
            .map(|info| {
                (
                    info.ident,
                    info.field_type,
                    info.column_name.clone(),
                    info.sql_definition.clone(),
                    info.is_autoincrement,
                    info.is_primary,
                )
            })
            .collect();

        // Call the dialect function (now returns Result)
        generate_sqlite_column_consts(&sqlite_field_info)? // Updated to use the local function directly
    };

    // Generate types for models
    let select_model_name = format_ident!("Select{}", struct_name);
    let insert_model_name = format_ident!("Insert{}", struct_name);
    let update_model_name = format_ident!("Update{}", struct_name);

    // Select model fields - use FieldInfo's get_select_type method
    let (select_model_fields_defs, insert_model_fields_defs, update_model_fields_defs): (
        Vec<TokenStream>,
        Vec<TokenStream>,
        Vec<TokenStream>,
    ) = field_infos
        .iter()
        .map(|info| {
            let name = info.ident;
            let select_type = info.get_select_type();
            let insert_type = info.get_insert_type();
            let update_type = info.get_update_type();
            (
                quote! { pub #name: #select_type },
                quote! { pub #name: #insert_type },
                quote! { pub #name: #update_type },
            )
        })
        .collect();

    // Generate CREATE TABLE SQL
    let mut create_table_sql = format!("CREATE TABLE '{}' (", table_name);
    if !field_infos.is_empty() {
        create_table_sql.push_str(&field_infos[0].sql_definition);
        for info in &field_infos[1..] {
            create_table_sql.push_str(", ");
            create_table_sql.push_str(&info.sql_definition);
        }
    }

    // Add composite primary key if needed
    if is_composite_pk {
        let primary_key_cols: Vec<_> = field_infos
            .iter()
            .filter(|info| info.is_primary)
            .map(|info| info.column_name.clone())
            .collect();

        create_table_sql.push_str(&format!(", PRIMARY KEY ({})", primary_key_cols.join(", ")));
    }

    create_table_sql.push(')');
    if strict {
        create_table_sql.push_str(" STRICT");
    }
    if without_rowid {
        create_table_sql.push_str(" WITHOUT ROWID");
    }
    create_table_sql.push(';');

    // Generate implementation for SQLSchema
    let sql_schema_impl = quote! {
        // Add <'a> back to impl block and use 'a for consts
        impl<'a> ::drizzle_rs::core::SQLSchema<'a> for #struct_name {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
            const SQL: &'a str = #create_table_sql;
        }
    };

    // Generate implementation for SQLTable
    let sql_table_impl = quote! {
        // Add <'a> back to impl block.
        impl<'a> ::drizzle_rs::core::SQLTable<'a> for #struct_name {
            type Select = #select_model_name;
            type Insert = #insert_model_name;
            type Update = #update_model_name;
        }
    };

    // Generate default implementation for Insert model
    let insert_model_default_fields = field_infos.iter().map(|info| {
        let name = info.ident;
        let base_type = info.base_type;

        // Special handling for default_fn
        if let Some(default_fn) = &info.default_fn {
            let is_required = info
                .get_insert_type()
                .to_string()
                .starts_with(&quote!(#base_type).to_string());

            if is_required {
                quote! { #name: #default_fn() }
            } else {
                quote! { #name: Some(#default_fn()) }
            }
        } else {
            let is_required = info
                .get_insert_type()
                .to_string()
                .starts_with(&quote!(#base_type).to_string());

            if is_required {
                // Required fields use default of base type
                quote! { #name: ::std::default::Default::default() }
            } else {
                // Optional fields default to None
                quote! { #name: None }
            }
        }
    });

    // Add Default implementation for the Insert model
    let insert_model_default_impl = quote! {
        impl Default for #insert_model_name {
            fn default() -> Self {
                Self {
                    #(#insert_model_default_fields),*
                }
            }
        }

        // Add builder methods for all fields
        impl #insert_model_name {
            /// Creates a new empty insert model
            pub fn new() -> Self {
                Self::default()
            }

        }
    };

    // Final assembly of generated code
    let mut expanded = quote! {
        #[derive(Default, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct #struct_name {
            // Empty struct, used for associated items
        }

        impl<'a> #struct_name {
            // Column constants
            #column_consts
        }

        // SQLSchema and SQLTable implementations
        #sql_schema_impl
        #sql_table_impl

        // Generate model for SELECT queries
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #select_model_name {
            #(#select_model_fields_defs),*
        }

        // Generate model for INSERT queries
        #[derive(Debug, Clone, PartialEq)]
        pub struct #insert_model_name {
            #(#insert_model_fields_defs),*
        }

        // Generate model for UPDATE queries
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #update_model_name {
            #(#update_model_fields_defs),*
        }

        // Add the insert model implementations (Default, builder)
        #insert_model_default_impl
    };

    // Generate JSON field implementations
    let json_impls = generate_json_impls(&field_infos)?;
    expanded.extend(json_impls);

    // Add SQLite specific implementations if feature is enabled
    #[cfg(feature = "sqlite")]
    {
        // --- Generate IntoParams impl --- //
        // let sqlite_into_params =
        //     generate_sqlite_into_params_impl(&insert_model_name, &field_infos)?;
        // expanded.extend(sqlite_into_params);
    }

    // Add rusqlite implementations if feature is enabled
    #[cfg(feature = "rusqlite")]
    {
        // Pass field_info directly to rusqlite::generate_rusqlite_impls
        let rusqlite_impls = rusqlite::generate_rusqlite_impls(
            &select_model_name,
            &insert_model_name,
            &update_model_name,
            &field_infos,
        )?;

        expanded.extend(rusqlite_impls);
    }

    Ok(expanded)
}
