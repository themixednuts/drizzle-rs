//! Shared field assignment generation for libsql and turso `FromRow` derive.
//!
//! Both drivers use `DrizzleRow::get_column` for unified type conversion via `FromSQLiteValue` trait.
//! Only JSON handling differs between them.

use crate::common::has_json_attribute;
use crate::paths;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;

/// Driver-specific JSON accessor generation
#[allow(dead_code)]
pub trait DriverJsonAccessor {
    /// Generate the JSON field accessor for this driver
    fn json_accessor(idx: TokenStream) -> TokenStream;

    /// Whether this driver can lookup row values by column name.
    fn supports_name_lookup() -> bool {
        false
    }

    /// Get the error type for this driver
    fn error_type() -> TokenStream;
}

/// Generate field assignment using the unified `DrizzleRow::get_column` interface.
///
/// This works for both libsql and turso since they both implement `DrizzleRow`.
pub fn generate_field_assignment<D: DriverJsonAccessor>(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    // Check for special attributes
    if has_json_attribute(field) {
        return handle_json_field::<D>(idx, field_name);
    }

    // All other types use DrizzleRow::get_column with FromSQLiteValue
    let field_type = &field.ty;
    let by_index_accessor = || {
        let drizzle_row = paths::sqlite::drizzle_row();
        quote! {
            {
                <_ as #drizzle_row>::get_column::<#field_type>(row, #idx)
            }
        }
    };
    let accessor = field_name.map_or_else(&by_index_accessor, |field_name| {
        if D::supports_name_lookup() {
            let field_name_str = field_name.to_string();
            quote! {
                {
                    use drizzle::sqlite::traits::DrizzleRowByName;
                    DrizzleRowByName::get_column_by_name::<#field_type>(row, #field_name_str)
                }
            }
        } else {
            by_index_accessor()
        }
    });

    field_name.map_or_else(
        || {
            quote! {
                #accessor?,
            }
        },
        |field_name| {
            quote! {
                #field_name: #accessor?,
            }
        },
    )
}

/// Handle JSON fields using driver-specific accessor
fn handle_json_field<D: DriverJsonAccessor>(idx: usize, name: Option<&syn::Ident>) -> TokenStream {
    let accessor = name.map_or_else(
        || D::json_accessor(quote!(#idx)),
        |field_name| {
            if D::supports_name_lookup() {
                let field_name_str = field_name.to_string();
                quote! {
                    {
                        use drizzle::sqlite::traits::DrizzleRowByName;
                        let json_str: String = DrizzleRowByName::get_column_by_name::<String>(row, #field_name_str)?;
                        serde_json::from_str(&json_str)
                            .map_err(|e| drizzle::error::DrizzleError::ConversionError(e.to_string().into()))
                    }
                }
            } else {
                D::json_accessor(quote!(#idx))
            }
        },
    );

    name.map_or_else(
        || {
            quote! {
                #accessor?,
            }
        },
        |field_name| {
            quote! {
                #field_name: #accessor?,
            }
        },
    )
}

/// Generate field assignment using an arbitrary index expression.
///
/// This is used for `FromDrizzleRow::from_row_at` where values are read from
/// `offset + idx`.
pub fn generate_field_assignment_with_index<D: DriverJsonAccessor>(
    idx_expr: TokenStream,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    if has_json_attribute(field) {
        let accessor = D::json_accessor(idx_expr);
        return field_name.map_or_else(
            || {
                quote! {
                    #accessor?,
                }
            },
            |field_name| {
                quote! {
                    #field_name: #accessor?,
                }
            },
        );
    }

    let field_type = &field.ty;
    let drizzle_row = paths::sqlite::drizzle_row();
    let accessor = quote! {
        {
            <_ as #drizzle_row>::get_column::<#field_type>(row, #idx_expr)
        }
    };

    field_name.map_or_else(
        || {
            quote! {
                #accessor?,
            }
        },
        |field_name| {
            quote! {
                #field_name: #accessor?,
            }
        },
    )
}
