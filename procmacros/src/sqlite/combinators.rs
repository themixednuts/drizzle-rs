use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// SQLite-specific ToSQL implementation combinator
pub fn to_sql_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                #body
            }
        }
    }
}

/// SQLite-specific SQLColumn implementation combinator
pub fn sql_column_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLiteColumnInfo implementation combinator
pub fn sqlite_column_info_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::sqlite::traits::SQLiteColumnInfo for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLiteColumn implementation combinator
pub fn sqlite_column_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::sqlite::traits::SQLiteColumn<'a> for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLiteTableInfo implementation combinator
pub fn sqlite_table_info_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::sqlite::traits::SQLiteTableInfo for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLiteTable implementation combinator
pub fn sqlite_table_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::sqlite::traits::SQLiteTable<'a> for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLTable implementation combinator
pub fn sql_table_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLSchema implementation combinator
pub fn sql_schema_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// SQLite-specific SQLSchema for fields implementation combinator
pub fn sql_schema_field_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// Combinator for generating method that forwards to original field
pub fn forward_to_original_field(
    method_name: &Ident,
    original_field_type: &Ident,
    return_type: TokenStream,
    trait_path: TokenStream,
) -> TokenStream {
    quote! {
        fn #method_name(&self) -> #return_type {
            static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
            <#original_field_type as #trait_path>::#method_name(&ORIGINAL_FIELD)
        }
    }
}