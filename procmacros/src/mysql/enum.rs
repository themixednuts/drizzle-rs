use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DataEnum, Error, Fields, Ident};

/// Generate the driver-neutral implementation for an inline MySQL `ENUM`.
pub fn generate_enum_impl(
    name: &Ident,
    data: &DataEnum,
    attrs: &[Attribute],
) -> syn::Result<TokenStream> {
    if data.variants.is_empty() {
        return Err(Error::new_spanned(
            name,
            "MySQLEnum requires at least one variant",
        ));
    }

    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new_spanned(
                variant,
                "MySQLEnum only supports fieldless variants",
            ));
        }
        if variant.discriminant.is_some() {
            return Err(Error::new_spanned(
                variant,
                "MySQLEnum variants cannot have explicit discriminants",
            ));
        }
    }

    if attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("repr"))
    {
        return Err(Error::new_spanned(
            name,
            "MySQLEnum does not support #[repr(...)]; MySQL ENUM values are textual labels",
        ));
    }

    let variants: Vec<_> = data.variants.iter().map(|variant| &variant.ident).collect();
    let variant_names: Vec<_> = variants
        .iter()
        .map(|variant| variant.to_string().trim_start_matches("r#").to_owned())
        .collect();
    if let Some(label) = variant_names
        .iter()
        .find(|label| label.contains(['\'', '\\']))
    {
        return Err(Error::new_spanned(
            name,
            format!(
                "MySQLEnum label `{label}` contains a quote or backslash whose meaning depends on MySQL SQL mode"
            ),
        ));
    }
    let sql_type = format!(
        "ENUM({})",
        variant_names
            .iter()
            .map(|variant| format!("'{}'", escape_mysql_string(variant)))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(quote! {
        impl drizzle::mysql::traits::MySQLEnum for #name {
            const SQL_TYPE: &'static str = #sql_type;
            const VARIANTS: &'static [&'static str] = &[#(#variant_names),*];

            fn variant_name(&self) -> &'static str {
                match self {
                    #(Self::#variants => #variant_names,)*
                }
            }

            fn try_from_str(value: &str) -> ::std::result::Result<Self, drizzle::error::DrizzleError> {
                match value {
                    #(#variant_names => ::std::result::Result::Ok(Self::#variants),)*
                    _ => ::std::result::Result::Err(
                        drizzle::error::DrizzleError::ConversionError(
                            ::std::format!(
                                "invalid {} value: {value}",
                                stringify!(#name),
                            )
                            .into(),
                        ),
                    ),
                }
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_str(
                    <Self as drizzle::mysql::traits::MySQLEnum>::variant_name(self),
                )
            }
        }

        impl ::std::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                <Self as drizzle::mysql::traits::MySQLEnum>::variant_name(self)
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = drizzle::error::DrizzleError;

            fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
                <Self as drizzle::mysql::traits::MySQLEnum>::try_from_str(value)
            }
        }

        impl ::std::convert::TryFrom<&str> for #name {
            type Error = drizzle::error::DrizzleError;

            fn try_from(value: &str) -> ::std::result::Result<Self, Self::Error> {
                <Self as drizzle::mysql::traits::MySQLEnum>::try_from_str(value)
            }
        }

        impl ::std::convert::TryFrom<::std::string::String> for #name {
            type Error = drizzle::error::DrizzleError;

            fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, Self::Error> {
                <Self as drizzle::mysql::traits::MySQLEnum>::try_from_str(&value)
            }
        }

        impl ::std::convert::TryFrom<&::std::string::String> for #name {
            type Error = drizzle::error::DrizzleError;

            fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, Self::Error> {
                <Self as drizzle::mysql::traits::MySQLEnum>::try_from_str(value)
            }
        }

        impl<'a> ::std::convert::From<#name> for drizzle::mysql::values::MySQLValue<'a> {
            fn from(value: #name) -> Self {
                Self::from(
                    <#name as drizzle::mysql::traits::MySQLEnum>::variant_name(&value),
                )
            }
        }

        impl<'a> ::std::convert::From<&#name> for drizzle::mysql::values::MySQLValue<'a> {
            fn from(value: &#name) -> Self {
                Self::from(
                    <#name as drizzle::mysql::traits::MySQLEnum>::variant_name(value),
                )
            }
        }

        impl<'a> ::std::convert::TryFrom<drizzle::mysql::values::MySQLValue<'a>> for #name {
            type Error = drizzle::error::DrizzleError;

            fn try_from(value: drizzle::mysql::values::MySQLValue<'a>) -> ::std::result::Result<Self, Self::Error> {
                match value {
                    drizzle::mysql::values::MySQLValue::Bytes(value) => {
                        let value = ::std::str::from_utf8(value.as_ref()).map_err(|error| {
                            drizzle::error::DrizzleError::ConversionError(error.to_string().into())
                        })?;
                        <Self as drizzle::mysql::traits::MySQLEnum>::try_from_str(value)
                    }
                    _ => ::std::result::Result::Err(
                        drizzle::error::DrizzleError::ConversionError(
                            ::std::format!(
                                "cannot convert non-text MySQL value to {}",
                                stringify!(#name),
                            )
                            .into(),
                        ),
                    ),
                }
            }
        }

        impl<'a> drizzle::core::ToSQL<'a, drizzle::mysql::values::MySQLValue<'a>> for #name {
            fn to_sql(&self) -> drizzle::core::SQL<'a, drizzle::mysql::values::MySQLValue<'a>> {
                let value: drizzle::mysql::values::MySQLValue<'a> = self.into();
                value.into()
            }
        }

        impl<'a> drizzle::core::expr::Expr<'a, drizzle::mysql::values::MySQLValue<'a>> for #name {
            type SQLType = drizzle::mysql::types::Enum;
            type Nullable = drizzle::core::expr::NonNull;
            type Aggregate = drizzle::core::expr::Scalar;
        }

        impl drizzle::core::ValueTypeForDialect<drizzle::mysql::MySQLDialect> for #name {
            type SQLType = drizzle::mysql::types::Enum;
        }

        impl drizzle::core::ValueTypeForDialect<drizzle::mysql::MySQLDialect> for &#name {
            type SQLType = drizzle::mysql::types::Enum;
        }

        impl<'__drizzle_row, __DrizzleRow: drizzle::mysql::driver::MySQLRowAccess + ?Sized>
            drizzle::core::FromDrizzleRow<
                drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>
            > for #name
        {
            const COLUMN_COUNT: usize = 1;

            fn from_row_at(
                row: &drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>,
                offset: usize,
            ) -> ::std::result::Result<Self, drizzle::error::DrizzleError> {
                let value = <::std::string::String as drizzle::core::FromDrizzleRow<
                    drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>
                >>::from_row_at(row, offset)?;
                <Self as drizzle::mysql::traits::MySQLEnum>::try_from_str(&value)
            }
        }
    })
}

fn escape_mysql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''")
}
