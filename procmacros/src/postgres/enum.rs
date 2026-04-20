use crate::common::enum_utils::{has_integer_repr, resolve_discriminants};
use crate::paths::{core as core_paths, postgres as postgres_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DataEnum, Ident};

// Generate implementation for PostgreSQL enum representation following SQLite pattern
pub fn generate_enum_impl(
    name: &Ident,
    data: &DataEnum,
    attrs: &[Attribute],
) -> syn::Result<TokenStream> {
    // Get paths for fully-qualified types
    let _sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_enum_info = core_paths::sql_enum_info();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let type_set_cons = core_paths::type_set_cons();
    let row_column_list = core_paths::row_column_list();
    let drizzle_error = core_paths::drizzle_error();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let value_type_for_dialect = core_paths::value_type_for_dialect();
    let postgres_dialect = core_paths::postgres_dialect();
    let core_expr = core_paths::expr();
    let postgres_types = postgres_paths::types();
    let postgres_enum_trait = postgres_paths::postgres_enum_trait();
    let postgres_row = postgres_paths::row();

    let Some(first_variant) = data.variants.first().map(|v| &v.ident) else {
        return Err(syn::Error::new_spanned(
            name,
            "#[derive(PostgresEnum)] requires at least one variant",
        ));
    };
    let variant_idents: Vec<_> = data.variants.iter().map(|v| &v.ident).collect();

    // Build the CREATE TYPE SQL at macro time as a string literal
    let variants_sql = variant_idents
        .iter()
        .map(|v| format!("'{v}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let create_type_sql = format!("CREATE TYPE {name} AS ENUM ({variants_sql})");
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

    // Detect storage format: INTEGER if has #[repr(iN)]
    let is_integer_storage = has_integer_repr(attrs);
    let drizzle_postgres_column = postgres_paths::drizzle_postgres_column();

    // Common base: integer and string conversions, Display, FromStr, etc.
    let common_impls = quote! {

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
    };

    // Generate DrizzlePostgresColumn impl (feature-gated on postgres driver availability)
    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    let drizzle_postgres_column_impl = if is_integer_storage {
        // Integer-stored enum: read as i32, write as Integer
        quote! {
            impl #drizzle_postgres_column for #name {
                const SQL_TYPE: &'static str = "integer";
                const NEEDS_CREATE_TYPE: bool = false;

                fn from_postgres_row(row: &drizzle::postgres::Row, idx: usize) -> ::std::result::Result<Self, #drizzle_error> {
                    let v: i32 = row.get::<_, i32>(idx);
                    <#name as ::std::convert::TryFrom<i32>>::try_from(v)
                        .map_err(|_| #drizzle_error::ConversionError(
                            ::std::format!("Failed to convert {} to {}", v, stringify!(#name)).into()
                        ))
                }

                fn to_postgres_value(&self) -> #postgres_value<'static> {
                    let integer: i64 = self.into();
                    #postgres_value::Integer(integer as i32)
                }
            }
        }
    } else {
        // Native enum: read via FromSql, write as Enum variant
        quote! {
            impl #drizzle_postgres_column for #name {
                const SQL_TYPE: &'static str = stringify!(#name);
                const NEEDS_CREATE_TYPE: bool = true;

                fn from_postgres_row(row: &drizzle::postgres::Row, idx: usize) -> ::std::result::Result<Self, #drizzle_error> {
                    let v: #name = row.get::<_, #name>(idx);
                    ::std::result::Result::Ok(v)
                }

                fn to_postgres_value(&self) -> #postgres_value<'static> {
                    #postgres_value::Enum(::std::boxed::Box::new(self.clone()))
                }
            }
        }
    };

    #[cfg(not(any(feature = "postgres-sync", feature = "tokio-postgres")))]
    let drizzle_postgres_column_impl = if !is_integer_storage {
        // Native enum without driver: no from_postgres_row method
        quote! {
            impl #drizzle_postgres_column for #name {
                const SQL_TYPE: &'static str = stringify!(#name);
                const NEEDS_CREATE_TYPE: bool = true;

                fn to_postgres_value(&self) -> #postgres_value<'static> {
                    #postgres_value::Enum(::std::boxed::Box::new(self.clone()))
                }
            }
        }
    } else {
        // Integer-stored enum without driver: no from_postgres_row method
        quote! {
            impl #drizzle_postgres_column for #name {
                const SQL_TYPE: &'static str = "integer";
                const NEEDS_CREATE_TYPE: bool = false;

                fn to_postgres_value(&self) -> #postgres_value<'static> {
                    let integer: i64 = self.into();
                    #postgres_value::Integer(integer as i32)
                }
            }
        }
    };

    // Native enum impls (PostgresEnum trait, SQLEnumInfo, SQLSchema, CREATE TYPE SQL, Expr with Enum type)
    // Only generated for non-integer-repr enums
    let native_enum_impls = if is_integer_storage {
        // Integer-repr enum: stored as integer column, no CREATE TYPE needed
        quote! {
            // Implement new() - returns the default variant
            impl #name {
                pub const fn new() -> Self {
                    #name::#first_variant
                }
            }

            // Implement Expr trait for type-safe comparisons — integer type
            impl<'a> #core_expr::Expr<'a, #postgres_value<'a>> for #name {
                type SQLType = #postgres_types::Integer;
                type Nullable = #core_expr::NonNull;
                type Aggregate = #core_expr::Scalar;
            }

            // DrizzlePostgresColumn for integer-stored enum
            #drizzle_postgres_column_impl

            // From<Enum> for PostgresValue (owned)
            impl<'a> ::std::convert::From<#name> for #postgres_value<'a> {
                fn from(value: #name) -> Self {
                    let integer: i64 = value.into();
                    #postgres_value::Integer(integer as i32)
                }
            }

            // From<&Enum> for PostgresValue (reference)
            impl<'a> ::std::convert::From<&#name> for #postgres_value<'a> {
                fn from(value: &#name) -> Self {
                    let integer: i64 = value.into();
                    #postgres_value::Integer(integer as i32)
                }
            }

            impl #value_type_for_dialect<#postgres_dialect> for #name {
                type SQLType = #postgres_types::Integer;
            }

            impl #value_type_for_dialect<#postgres_dialect> for &#name {
                type SQLType = #postgres_types::Integer;
            }

            // TryFrom<PostgresValue> for the enum (read path)
            impl<'a> ::std::convert::TryFrom<#postgres_value<'a>> for #name {
                type Error = #drizzle_error;

                fn try_from(value: #postgres_value<'a>) -> ::std::result::Result<Self, Self::Error> {
                    match value {
                        #postgres_value::Integer(i) => <#name as ::std::convert::TryFrom<i32>>::try_from(i),
                        #postgres_value::Bigint(i) => <#name as ::std::convert::TryFrom<i64>>::try_from(i),
                        _ => ::std::result::Result::Err(#drizzle_error::ConversionError(
                            ::std::format!("Cannot convert {:?} to {}", value, stringify!(#name)).into(),
                        )),
                    }
                }
            }
        }
    } else {
        quote! {
            // Implement PostgresEnum trait for native PostgreSQL enum support
            impl #postgres_enum_trait for #name {
                fn enum_type_name(&self) -> &'static str {
                    stringify!(#name)
                }

                fn as_enum(&self) -> &dyn #postgres_enum_trait {
                    self
                }

                fn variant_name(&self) -> &'static str {
                    match self {
                        #(#to_str_variants,)*
                    }
                }

                fn into_boxed(&self) -> ::std::boxed::Box<dyn #postgres_enum_trait> {
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
                    // Reuse the compile-time const SQL instead of rebuilding at runtime
                    <#name as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL.to_string()
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
                const SQL: &'static str = #create_type_sql_literal;
            }

            // Implement new() for schema integration - returns the default variant
            impl #name {
                /// Creates a new instance of this enum with its default variant.
                /// Used by PostgresSchema for schema initialization.
                pub const fn new() -> Self {
                    #name::#first_variant
                }
            }

            // Implement Expr trait for type-safe comparisons — native enum type
            impl<'a> #core_expr::Expr<'a, #postgres_value<'a>> for #name {
                type SQLType = #postgres_types::Enum;
                type Nullable = #core_expr::NonNull;
                type Aggregate = #core_expr::Scalar;
            }

            // DrizzlePostgresColumn for native enum
            #drizzle_postgres_column_impl

            // From<Enum> for PostgresValue (owned)
            impl<'a> ::std::convert::From<#name> for #postgres_value<'a> {
                fn from(value: #name) -> Self {
                    #postgres_value::Enum(::std::boxed::Box::new(value))
                }
            }

            // From<&Enum> for PostgresValue (reference)
            impl<'a> ::std::convert::From<&#name> for #postgres_value<'a> {
                fn from(value: &#name) -> Self {
                    #postgres_value::Enum(::std::boxed::Box::new((*value).clone()))
                }
            }

            impl #value_type_for_dialect<#postgres_dialect> for #name {
                type SQLType = #postgres_types::Enum;
            }

            impl #value_type_for_dialect<#postgres_dialect> for &#name {
                type SQLType = #postgres_types::Enum;
            }

            // TryFrom<PostgresValue> for the enum (read path)
            impl<'a> ::std::convert::TryFrom<#postgres_value<'a>> for #name {
                type Error = #drizzle_error;

                fn try_from(value: #postgres_value<'a>) -> ::std::result::Result<Self, Self::Error> {
                    match value {
                        #postgres_value::Text(cow) => <#name as ::std::str::FromStr>::from_str(cow.as_ref()),
                        #postgres_value::Enum(boxed) => {
                            <#name as ::std::str::FromStr>::from_str(boxed.variant_name())
                        }
                        _ => ::std::result::Result::Err(#drizzle_error::ConversionError(
                            ::std::format!("Cannot convert {:?} to {}", value, stringify!(#name)).into(),
                        )),
                    }
                }
            }
        }
    };

    // ToSQL implementation (delegates to From)
    let to_sql_trait = core_paths::to_sql_trait();
    let sql_path = core_paths::sql();

    Ok(quote! {
        #common_impls
        #native_enum_impls

        // ToSQL implementation (delegates to From)
        impl<'a> #to_sql_trait<'a, #postgres_value<'a>> for #name {
            fn to_sql(&self) -> #sql_path<'a, #postgres_value<'a>> {
                <#postgres_value<'_> as ::std::convert::From<&#name>>::from(self).into()
            }
        }

        #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
        impl #row_column_list<#postgres_row> for #name {
            type Columns = #type_set_cons<#name, #type_set_nil>;
        }

        impl #schema_item_tables for #name {
            type Tables = #type_set_nil;
        }
    })
}
