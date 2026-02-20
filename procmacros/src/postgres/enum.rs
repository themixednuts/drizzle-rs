use crate::common::enum_utils::resolve_discriminants;
use crate::paths::{core as core_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Ident};

// Generate implementation for PostgreSQL enum representation following SQLite pattern
pub fn generate_enum_impl(name: &Ident, data: &DataEnum) -> syn::Result<TokenStream> {
    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_enum_info = core_paths::sql_enum_info();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let type_set_cons = core_paths::type_set_cons();
    let row_column_list = core_paths::row_column_list();
    let drizzle_error = core_paths::drizzle_error();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    let Some(first_variant) = data.variants.first().map(|v| &v.ident) else {
        return Err(syn::Error::new_spanned(
            name,
            "PostgresEnum cannot be derived for empty enums",
        ));
    };
    let variant_idents: Vec<_> = data.variants.iter().map(|v| &v.ident).collect();

    // Build the CREATE TYPE SQL at macro time as a string literal
    let variants_sql = variant_idents
        .iter()
        .map(|v| format!("'{}'", v))
        .collect::<Vec<_>>()
        .join(", ");
    let create_type_sql = format!("CREATE TYPE {} AS ENUM ({})", name, variants_sql);
    let create_type_sql_literal = create_type_sql.as_str();

    let display_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        quote! {
            #name::#variant_name => write!(f, #variant_str),
        }
    });

    let to_str_variants: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;

            quote! {
                #name::#ident => stringify!(#ident)
            }
        })
        .collect();

    let to_str_ref_variants: Box<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;

            quote! {
                &#name::#ident => stringify!(#ident)
            }
        })
        .collect();

    let from_str_variants: Box<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;

            quote! {
                stringify!(#ident) => #name::#ident,
            }
        })
        .collect();

    let boxed_variants: Box<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            quote! {
                #name::#ident => ::std::boxed::Box::new(#name::#ident)
            }
        })
        .collect();

    // Resolve discriminants with uniqueness validation
    let resolved = resolve_discriminants(data)?;

    let to_integer_variants: Box<[_]> = resolved
        .iter()
        .map(|(ident, value)| {
            quote! { #name::#ident => #value }
        })
        .collect();

    let to_integer_ref_variants: Box<[_]> = resolved
        .iter()
        .map(|(ident, value)| {
            quote! { &#name::#ident => #value }
        })
        .collect();

    let from_integer_variants: Box<[_]> = resolved
        .iter()
        .map(|(ident, value)| {
            quote! { i if i == #value => #name::#ident }
        })
        .collect();

    // Generate postgres FromSql/ToSql impls when postgres feature is enabled
    #[cfg(feature = "postgres")]
    let postgres_impls = {
        let name_str = name.to_string();
        quote! {
            // When tokio-postgres is enabled, impl against tokio_postgres::types
            #[cfg(feature = "tokio-postgres")]
            impl<'a> ::tokio_postgres::types::FromSql<'a> for #name {
                fn from_sql(
                    _ty: &::tokio_postgres::types::Type,
                    raw: &'a [u8],
                ) -> ::std::result::Result<Self, ::std::boxed::Box<dyn ::std::error::Error + ::core::marker::Sync + ::core::marker::Send>> {
                    let s = ::std::str::from_utf8(raw)?;
                    <#name as ::std::str::FromStr>::from_str(s).map_err(|_| {
                        ::std::format!("Failed to parse {} from '{}'", #name_str, s).into()
                    })
                }

                fn accepts(ty: &::tokio_postgres::types::Type) -> bool {
                    ty.name().eq_ignore_ascii_case(#name_str)
                        || *ty == ::tokio_postgres::types::Type::TEXT
                        || *ty == ::tokio_postgres::types::Type::VARCHAR
                }
            }

            #[cfg(feature = "tokio-postgres")]
            impl ::tokio_postgres::types::ToSql for #name {
                fn to_sql(
                    &self,
                    _ty: &::tokio_postgres::types::Type,
                    out: &mut ::bytes::BytesMut,
                ) -> ::std::result::Result<::tokio_postgres::types::IsNull, ::std::boxed::Box<dyn ::std::error::Error + ::core::marker::Sync + ::core::marker::Send>> {
                    let s: &str = self.into();
                    ::tokio_postgres::types::ToSql::to_sql(&s, _ty, out)?;
                    ::std::result::Result::Ok(::tokio_postgres::types::IsNull::No)
                }

                fn accepts(ty: &::tokio_postgres::types::Type) -> bool {
                    ty.name().eq_ignore_ascii_case(#name_str)
                        || *ty == ::tokio_postgres::types::Type::TEXT
                        || *ty == ::tokio_postgres::types::Type::VARCHAR
                }

                ::tokio_postgres::types::to_sql_checked!();
            }

            // When only postgres-sync is enabled (without tokio-postgres), impl against postgres::types
            #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
            impl<'a> ::postgres::types::FromSql<'a> for #name {
                fn from_sql(
                    _ty: &::postgres::types::Type,
                    raw: &'a [u8],
                ) -> ::std::result::Result<Self, ::std::boxed::Box<dyn ::std::error::Error + ::core::marker::Sync + ::core::marker::Send>> {
                    let s = ::std::str::from_utf8(raw)?;
                    <#name as ::std::str::FromStr>::from_str(s).map_err(|_| {
                        ::std::format!("Failed to parse {} from '{}'", #name_str, s).into()
                    })
                }

                fn accepts(ty: &::postgres::types::Type) -> bool {
                    ty.name().eq_ignore_ascii_case(#name_str)
                        || *ty == ::postgres::types::Type::TEXT
                        || *ty == ::postgres::types::Type::VARCHAR
                }
            }

            #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
            impl ::postgres::types::ToSql for #name {
                fn to_sql(
                    &self,
                    _ty: &::postgres::types::Type,
                    out: &mut ::bytes::BytesMut,
                ) -> ::std::result::Result<::postgres::types::IsNull, ::std::boxed::Box<dyn ::std::error::Error + ::core::marker::Sync + ::core::marker::Send>> {
                    let s: &str = self.into();
                    ::postgres::types::ToSql::to_sql(&s, _ty, out)?;
                    ::std::result::Result::Ok(::postgres::types::IsNull::No)
                }

                fn accepts(ty: &::postgres::types::Type) -> bool {
                    ty.name().eq_ignore_ascii_case(#name_str)
                        || *ty == ::postgres::types::Type::TEXT
                        || *ty == ::postgres::types::Type::VARCHAR
                }

                ::postgres::types::to_sql_checked!();
            }
        }
    };

    #[cfg(not(feature = "postgres"))]
    let postgres_impls = quote! {};

    Ok(quote! {

        #postgres_impls

        impl ::std::convert::From<#name> for i64 {
            fn from(value: #name) -> Self {
                match value {
                    #(#to_integer_variants,)*
                }
            }
        }

        impl ::std::convert::From<&#name> for i64 {
            fn from(value: &#name) -> Self {
                match value {
                    #(#to_integer_ref_variants,)*
                }
            }
        }

        impl ::std::convert::TryFrom<i64> for #name {
            type Error = #drizzle_error;

            fn try_from(value: i64) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(match value {
                    #(#from_integer_variants,)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{value}").into())),
                })
            }
        }

        impl ::std::convert::TryFrom<&i64> for #name {
            type Error = #drizzle_error;

            fn try_from(value: &i64) -> ::std::result::Result<Self, Self::Error> {
                let value = *value;
                ::std::result::Result::Ok(match value {
                    #(#from_integer_variants,)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{value}").into())),
                })
            }
        }

        // Generic Option implementation - works for any T that can convert to the enum
        impl<T> ::std::convert::TryFrom<::std::option::Option<T>> for #name
        where
            T: ::std::convert::TryInto<#name>,
            T::Error: ::std::convert::Into<#drizzle_error>,
        {
            type Error = #drizzle_error;

            fn try_from(value: ::std::option::Option<T>) -> ::std::result::Result<Self, Self::Error> {
                match value {
                    ::std::option::Option::Some(inner) => inner.try_into().map_err(::std::convert::Into::into),
                    ::std::option::Option::None => ::std::result::Result::Err(#drizzle_error::Mapping("Cannot convert None to enum".into())),
                }
            }
        }
        // Generic implementation for integer types that aren't explicitly covered
        impl ::std::convert::TryFrom<isize> for #name {
            type Error = #drizzle_error;

            fn try_from(value: isize) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl ::std::convert::TryFrom<usize> for #name {
            type Error = #drizzle_error;

            fn try_from(value: usize) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        // Generic implementation for integer types that aren't explicitly covered
        impl ::std::convert::TryFrom<i32> for #name {
            type Error = #drizzle_error;

            fn try_from(value: i32) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl ::std::convert::TryFrom<u32> for #name {
            type Error = #drizzle_error;

            fn try_from(value: u32) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl ::std::convert::TryFrom<i16> for #name {
            type Error = #drizzle_error;

            fn try_from(value: i16) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl ::std::convert::TryFrom<u16> for #name {
            type Error = #drizzle_error;

            fn try_from(value: u16) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl ::std::convert::TryFrom<i8> for #name {
            type Error = #drizzle_error;

            fn try_from(value: i8) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl ::std::convert::TryFrom<u8> for #name {
            type Error = #drizzle_error;

            fn try_from(value: u8) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        // Implement Display for the enum
        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_variants)*
                }
            }
        }

        impl ::std::convert::From<#name> for &str {
            fn from(value: #name) -> Self {
                match value {
                    #(#to_str_variants,)*
                }
            }
        }

        impl ::std::convert::From<&#name> for &str {
            fn from(value: &#name) -> Self {
                match value {
                    #(#to_str_ref_variants,)*
                }
            }
        }

        impl ::std::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
                    #(#to_str_variants,)*
                }
            }
        }

        impl ::std::convert::TryFrom<&str> for #name {
            type Error = #drizzle_error;

            fn try_from(value: &str) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(match value {
                    #(#from_str_variants)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{value}").into())),
                })
            }
        }

        impl ::std::convert::TryFrom<&::std::string::String> for #name {
            type Error = #drizzle_error;

            fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, Self::Error> {
                <#name as ::std::str::FromStr>::from_str(value)
            }
        }

        impl ::std::convert::TryFrom<::std::string::String> for #name {
            type Error = #drizzle_error;

            fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, Self::Error> {
                <#name as ::std::str::FromStr>::from_str(&value)
            }
        }


        // Implement FromStr for the enum with String as the error type
        impl ::std::str::FromStr for #name {
            type Err = #drizzle_error;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                ::std::result::Result::Ok(match s {
                    #(#from_str_variants)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{s}").into())),
                })
            }
        }

        // Implement PostgresEnum trait for native PostgreSQL enum support
        impl drizzle::postgres::traits::PostgresEnum for #name {
            fn enum_type_name(&self) -> &'static str {
                stringify!(#name)
            }

            fn as_enum(&self) -> &dyn drizzle::postgres::traits::PostgresEnum {
                self
            }

            fn variant_name(&self) -> &'static str {
                match self {
                    #(#to_str_variants,)*
                }
            }

            fn into_boxed(&self) -> ::std::boxed::Box<dyn drizzle::postgres::traits::PostgresEnum> {
                match self {
                    #(#boxed_variants,)*
                }
            }

            fn try_from_str(value: &str) -> ::std::result::Result<Self, #drizzle_error> {
                Self::try_from(value)
            }
        }

        // Implement SQLEnumInfo trait for schema integration
        impl #sql_enum_info for #name {
            fn name(&self) -> &'static str {
                stringify!(#name)
            }

            fn create_type_sql(&self) -> ::std::string::String {
                let variants = #sql_enum_info::variants(self);
                let variants_str = variants
                    .iter()
                    .map(|v| ::std::format!("'{}'", v))
                    .collect::<::std::vec::Vec<_>>()
                    .join(", ");
                ::std::format!("CREATE TYPE {} AS ENUM ({})", stringify!(#name), variants_str)
            }

            fn variants(&self) -> &'static [&'static str] {
                &[#(stringify!(#variant_idents),)*]
            }
        }

        // Implement SQLSchema trait for schema integration
        impl<'a> #sql_schema<'a, #postgres_schema_type, #postgres_value<'a>> for #name {
            const NAME: &'static str = stringify!(#name);
            const TYPE: #postgres_schema_type = {
                #[allow(non_upper_case_globals)]
                static ENUM_INSTANCE: #name = #name::#first_variant;
                #postgres_schema_type::Enum(&ENUM_INSTANCE)
            };
            const SQL: &'static str = "";

            fn ddl(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::raw(#create_type_sql_literal)
            }
        }

        // Implement new() for schema integration - returns the default variant
        impl #name {
            /// Creates a new instance of this enum with its default variant.
            /// Used by PostgresSchema for schema initialization.
            pub const fn new() -> Self {
                #name::#first_variant
            }
        }

        // Implement Expr trait for type-safe comparisons
        // Uses Any type since enums have their own SQL type
        // Note: &T impl is handled by blanket impl in drizzle_core
        impl<'a> drizzle::core::expr::Expr<'a, #postgres_value<'a>> for #name {
            type SQLType = drizzle::postgres::types::Any;
            type Nullable = drizzle::core::expr::NonNull;
            type Aggregate = drizzle::core::expr::Scalar;
        }

        #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
        impl #row_column_list<drizzle::postgres::Row> for #name {
            type Columns = #type_set_cons<#name, #type_set_nil>;
        }

        impl #schema_item_tables for #name {
            type Tables = #type_set_nil;
        }

    })
}
