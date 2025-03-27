mod attribute;

use super::field::{FieldAttributes, TableField};
use attribute::TableAttributes;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, Type};

pub(crate) fn table_macro(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let table_attributes = TableAttributes::try_from(&input.attrs)?;

    let table_name = table_attributes
        .name
        .unwrap_or_else(|| struct_name.to_string().to_lowercase());

    let strict = table_attributes.strict;
    let without_rowid = table_attributes.without_rowid;

    let fields = if let syn::Data::Struct(ref data) = input.data {
        data.fields
            .iter()
            .map(|field| {
                let ident = field
                    .ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(&field, "No field name."))?;

                let field_attributes = FieldAttributes::try_from(&field.attrs)?;

                Ok(TableField {
                    ident,
                    attrs: field_attributes,
                    field,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?
    } else {
            return Err(syn::Error::new_spanned(
                &input,
            "SQLiteTable can only be derived for structs.",
        ));
    };

    // Collect primary key columns
    let primary_key_fields: Vec<_> = fields
        .iter()
        .filter(|field| field.attrs.primary_key.is_some())
        .collect();

    // Validate autoincrement is only used on primary key fields
    for field in &fields {
        if let Some(ref auto) = field.attrs.autoincrement {
            if field.attrs.primary_key.is_none() {
                return Err(syn::Error::new_spanned(
                    auto,
                    "drizzle: 'auto increment' can only be assigned to 'primary key'.",
                ));
            }
        }
    }

    // Generate field definitions
    let field_definitions = generate_field_accessors(struct_name, &table_name, strict, without_rowid, &fields, &primary_key_fields);

    // Generate relationship methods
    // let relationship_methods = generate_relationship_methods(struct_name, &fields);

    Ok(quote! {
        #field_definitions
        // #table_impl
        // #relationship_methods
    })
}

fn generate_field_accessors(struct_name: &Ident,
	table_name: &str,
	strict: bool,
	without_rowid: bool,
	 fields: &[TableField],
	 primary_key_fields: &[&TableField],
	) -> TokenStream {
    let (field_defs, field_sqls): (Vec<_>, Vec<_>) = fields.iter().map(|field| {
        let field_name = field.ident;
        let field_type = &field.field.ty;

        // Get column type and validation check based on field attributes
        let ( data_type, validation_check) = match field.attrs.column_type.as_deref() {
            Some("integer") => {
                let type_check = generate_integer_validation_check(field_type);
                (
                    quote! { crate::prelude::Integer },
                    type_check,
                )
            }
            Some("text") => {
                let type_check = generate_text_validation_check(field_type);
                (
                    quote! { crate::prelude::Text },
                    type_check,
                )
            }
            Some("blob") => {
                let type_check = generate_blob_validation_check(field_type);
                (
                    quote! { crate::prelude::Blob },
                    type_check,
                )
            }
            Some("real") => {
                let type_check = generate_real_validation_check(field_type);
                (
                    quote! { crate::prelude::Real },
                    type_check,
                )
            }
            _ => {
                let type_check = generate_text_validation_check(field_type);
                (
                    quote! { crate::prelude::Text },
                    type_check,
                )
            }
        };
      

        let column_name = match &field.attrs.name {
            Some(name) => name.clone(),
            None => field_name.to_string(),
        };

				let is_primary = field.attrs.primary_key.is_some();
				let is_autoincrement = field.attrs.autoincrement.is_some();
				let is_unique = field.attrs.unique.is_some();
				let is_nullable = super::field::is_option_type(&field.field.ty);

				// Create column definition
				let mut sql = format!(
				    "{} {}",
				    column_name,
				    match field.attrs.column_type.as_deref() {
				        Some("integer") => "INTEGER",
				        Some("real") => "REAL",
				        Some("text") => "TEXT",
				        Some("blob") => "BLOB",
				        Some("number") => "NUMERIC",
				        _ => "TEXT", // Default to TEXT
				    }
				);

				// Add column constraints
				if is_primary && primary_key_fields.len() <= 1 {
				    sql.push_str(" PRIMARY KEY");
				    if is_autoincrement {
				        sql.push_str(" AUTOINCREMENT");
				    }
				}

				if !is_nullable {
				    sql.push_str(" NOT NULL");
				}

				if is_unique {
				    sql.push_str(" UNIQUE");
				}

				// Add default value
				if let Some(default) = &field.attrs.default_value {
				    // For simple cases, format the default value
				    if let syn::Expr::Lit(expr_lit) = default {
				        match &expr_lit.lit {
				            syn::Lit::Int(i) => sql.push_str(&format!(" DEFAULT {}", i)),
				            syn::Lit::Float(f) => sql.push_str(&format!(" DEFAULT {}", f)),
				            syn::Lit::Bool(b) => {
				                sql.push_str(&format!(" DEFAULT {}", if b.value { 1 } else { 0 }))
				            }
				            syn::Lit::Str(s) => sql.push_str(&format!(" DEFAULT '{}'", s.value())),
				            _ => {}
				        }
				    } 
			  }

				let default_fn = if let Some(default) = &field.attrs.default_fn {
					quote! {
						Some(#default)
					}
				} else {
					quote! { None }
				};

        (
            quote! {
			#[allow(non_upper_case_globals, non_snake_case, dead_code)]
            pub const #field_name: crate::prelude::SQLiteColumn<#data_type, #struct_name, fn() -> #field_type, #field_type> = 
                crate::prelude::SQLiteColumn::new(
                    #column_name,
                    #sql,
                    #default_fn
                );
            },
            sql
        )
    }).unzip();

    // Build the CREATE TABLE SQL string
    let mut create_table_sql = format!("CREATE TABLE {} (", table_name);
    
    // Add field definitions
    create_table_sql.push_str(&field_sqls.join(", "));

    // Add composite primary key if needed
    if primary_key_fields.len() > 1 {
        let primary_key_cols: Vec<_> = primary_key_fields
            .iter()
            .map(|field| {
                field.attrs.name.as_ref()
                    .unwrap_or(&field.ident.to_string())
                    .clone()
            })
            .collect();
        create_table_sql.push_str(&format!(", PRIMARY KEY ({})", primary_key_cols.join(", ")));
    }

    create_table_sql.push(')');

    // Add STRICT if specified
    if strict {
        create_table_sql.push_str(" STRICT");
    }

    // Add WITHOUT ROWID if specified
    if without_rowid {
        create_table_sql.push_str(" WITHOUT ROWID");
    }

    create_table_sql.push(';');

    quote! {
        #[allow(dead_code)]
        impl #struct_name {
            #(#field_defs)*
        }

		impl crate::prelude::SQLiteTableSchema for #struct_name {
	        const NAME: &'static str = #table_name;
	        const TYPE: crate::prelude::SQLiteTableType = crate::prelude::SQLiteTableType::Table;
            const SQL: &'static str = #create_table_sql;
	    }
	}
}

fn generate_relationship_methods(struct_name: &Ident, fields: &[TableField]) -> TokenStream {
    let methods = fields.iter().filter_map(|field| {
        let field_ident = field.ident;

        // Handle path-based references
        if let Some(references_path) = &field.attrs.references_path {
            let path_expr = references_path;

            Some(quote! {
                pub fn #field_ident(&self) -> impl crate::prelude::query_builder::ForeignKey {
                    crate::prelude::query_builder::SQLChunk::new()
                        .add(crate::prelude::query_builder::SQL::Raw(format!(
                            "{}.{} REFERENCES {}.{}",
                            self.name(),
                            stringify!(#field_ident),
                            <#path_expr as crate::sqlite::prelude::Column>::table().name(),
                            <#path_expr as crate::sqlite::prelude::Column>::name()
                        )))
                }
            })
        }
        // Handle string-based references
        else if let Some(ref reference) = field.attrs.references {
            let ref_val = reference.value();
            if let Some((table, column)) = ref_val.split_once('.') {
                Some(quote! {
                    pub fn #field_ident(&self) -> impl crate::prelude::query_builder::ForeignKey {
                        crate::query_builder::SQLChunk::new()
                            .add(crate::sqlite::prelude::query_builder::SQL::Raw(format!(
                                "{}.{} REFERENCES {}.{}",
                                self.name(),
                                stringify!(#field_ident),
                                #table,
                                #column
                            )))
                    }
                })
            } else {
                None
            }
        } else {
            None
        }
    });

    quote! {
        impl #struct_name {
            #(#methods)*
        }
    }
}

// Helper function to check if a type is an enum
fn is_enum_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            // Just check if the type name exists and is potentially an enum
            // We can't check the definition since we don't have access to the enum definition
            return true;
        }
    }
    false
}

/// Generates a compile-time check to ensure that a type can be stored as INTEGER
///
/// SQLite INTEGER columns can store:
/// - Signed integers: i8, i16, i32, i64, isize
/// - Unsigned integers: u8, u16, u32, u64, usize
/// - Boolean: bool (stored as 0/1)
/// - Enum types with Display and FromStr implementations
///
/// This function generates code that will fail to compile if the field type
/// is not one of the supported types for INTEGER columns.
fn generate_integer_validation_check(ty: &syn::Type) -> TokenStream {
    let is_option = super::field::is_option_type(ty);
    let inner_ty = if is_option {
        super::field::get_option_inner_type(ty).unwrap_or(ty)
    } else {
        ty
    };

    let is_enum = is_enum_type(inner_ty);

    // If it's an enum, we need to ensure it implements Display and FromStr
    if is_enum {
        return quote! {
            {
                trait __SQLiteEnumCompatible {
                    fn __validate_enum_compatibility() {}
                }

                // Enums should implement Display and FromStr
                impl<T> __SQLiteEnumCompatible for T
                where
                    T: ::std::fmt::Display + ::std::str::FromStr + 'static
                {
                    fn __validate_enum_compatibility() {}
                }

                // This will fail to compile if T doesn't satisfy constraints
                <#inner_ty as __SQLiteEnumCompatible>::__validate_enum_compatibility();
            }
        };
    }

    quote! {
        {
            trait __SQLiteIntegerCompatible {
                fn __validate_integer_compatibility() {}
            }

            // Add back the primitive implementations and validation check:
            // Integer types
            impl __SQLiteIntegerCompatible for i8 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for i16 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for i32 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for i64 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for isize { fn __validate_integer_compatibility() {} }

            // Unsigned integer types (SQLite has no unsigned, but these can be converted)
            impl __SQLiteIntegerCompatible for u8 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for u16 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for u32 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for u64 { fn __validate_integer_compatibility() {} }
            impl __SQLiteIntegerCompatible for usize { fn __validate_integer_compatibility() {} }

            // Boolean is stored as 0/1 in SQLite INTEGER columns
            impl __SQLiteIntegerCompatible for bool { fn __validate_integer_compatibility() {} }

            // This will fail to compile if T isn't compatible with INTEGER
            <#inner_ty as __SQLiteIntegerCompatible>::__validate_integer_compatibility();
        }
    }
}

/// Generates a compile-time check to ensure that a type can be stored as REAL
///
/// SQLite REAL columns can store:
/// - f32, f64 (floating point numbers)
///
/// This function generates code that will fail to compile if the field type
/// is not one of the supported types for REAL columns.
fn generate_real_validation_check(ty: &syn::Type) -> TokenStream {
    let is_option = super::field::is_option_type(ty);
    let inner_ty = if is_option {
        super::field::get_option_inner_type(ty).unwrap_or(ty)
    } else {
        ty
    };

    quote! {
        {
            trait __SQLiteRealCompatible {
                fn __validate_real_compatibility() {}
            }

            // Add back the primitive implementations and validation check:
            // Floating point types
            impl __SQLiteRealCompatible for f32 { fn __validate_real_compatibility() {} }
            impl __SQLiteRealCompatible for f64 { fn __validate_real_compatibility() {} }

            // This will fail to compile if T isn't compatible with REAL
            <#inner_ty as __SQLiteRealCompatible>::__validate_real_compatibility();
        }
    }
}

/// Generates a compile-time check to ensure that a type can be serialized to TEXT
/// and deserialized from TEXT
///
/// SQLite TEXT columns can store:
/// - String: Directly supported
/// - Any type implementing std::fmt::Display and std::str::FromStr
/// - Enum types with Display and FromStr implementations
///
/// This function generates code that will fail to compile if the field type
/// doesn't implement the required traits.
fn generate_text_validation_check(ty: &syn::Type) -> TokenStream {
    let is_option = super::field::is_option_type(ty);
    let inner_ty = if is_option {
        super::field::get_option_inner_type(ty).unwrap_or(ty)
    } else {
        ty
    };

    let is_enum = is_enum_type(inner_ty);

    // If it's an enum, provide specific validation logic
    if is_enum {
        return quote! {
            {
                trait __SQLiteEnumCompatible {
                    fn __validate_enum_compatibility() {}
                }

                // Enums should implement Display and FromStr
                impl<T> __SQLiteEnumCompatible for T
                where
                    T: ::std::fmt::Display + ::std::str::FromStr + 'static
                {
                    fn __validate_enum_compatibility() {}
                }

                // This will fail to compile if T doesn't satisfy constraints
                <#inner_ty as __SQLiteEnumCompatible>::__validate_enum_compatibility();
            }
        };
    }

    // For non-enum types, check for Display + FromStr
    quote! {
        {
            trait __SQLiteTextCompatible {
                fn __validate_text_compatibility() {}
            }

            // Type implements Display for conversion to TEXT and FromStr for parsing back
            impl<T> __SQLiteTextCompatible for T
            where
                T: ::std::fmt::Display + ::std::str::FromStr + 'static
            {
                fn __validate_text_compatibility() {}
            }

            // This will fail to compile if T doesn't implement Display + FromStr
            <#inner_ty as __SQLiteTextCompatible>::__validate_text_compatibility();
        }
    }
}

/// Generates a compile-time check to ensure that a type can be serialized to BLOB
/// and deserialized from BLOB
///
/// SQLite BLOB columns can store:
/// - Vec<u8>: Directly supported
/// - Any type implementing AsRef<[u8]> and TryFrom<&[u8]>
///
/// This function generates code that will fail to compile if the field type
/// doesn't implement the required traits.
fn generate_blob_validation_check(ty: &syn::Type) -> TokenStream {
    let is_option = super::field::is_option_type(ty);
    let inner_ty = if is_option {
        super::field::get_option_inner_type(ty).unwrap_or(ty)
    } else {
        ty
    };

    // Check for Vec<u8> first as a special case
    let is_vec_u8 = match inner_ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(elem_path))) =
                            args.args.first()
                        {
                            if let Some(elem_segment) = elem_path.path.segments.last() {
                                if elem_segment.ident == "u8" {
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    };

    if is_vec_u8 {
        // If it's Vec<u8>, we can just return empty validation
        return quote! {};
    }

    // Generate trait checks for types that can be serialized to BLOB
    quote! {
        {
            trait __SQLiteBlobCompatible {
                fn __validate_blob_compatibility() {}
            }

            // Handle all types in a single impl with type-level conditionals
            impl<T> __SQLiteBlobCompatible for T
            where
                T: 'static,
                T: AsRef<[u8]> + for<'a> TryFrom<&'a [u8]> + 'static,
            {
                fn __validate_blob_compatibility() {}
            }

            // This will fail to compile if T doesn't implement AsRef<[u8]> + TryFrom<&[u8]>
            <#inner_ty as __SQLiteBlobCompatible>::__validate_blob_compatibility();
        }
    }
}
