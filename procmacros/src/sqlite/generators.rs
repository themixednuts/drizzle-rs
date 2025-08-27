use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate SQLite ToSQL trait implementation
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                #body
            }
        }
    }
}

/// Generate SQLite SQLColumn trait implementation
pub fn generate_sql_column(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLiteColumnInfo trait implementation
pub fn generate_sqlite_column_info(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::sqlite::traits::SQLiteColumnInfo for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLiteColumn trait implementation
pub fn generate_sqlite_column(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::sqlite::traits::SQLiteColumn<'a> for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLiteTableInfo trait implementation
pub fn generate_sqlite_table_info(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::sqlite::traits::SQLiteTableInfo for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLiteTable trait implementation
pub fn generate_sqlite_table(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::sqlite::traits::SQLiteTable<'a> for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLTable trait implementation
pub fn generate_sql_table(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLSchema trait implementation
pub fn generate_sql_schema(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// Generate SQLite SQLSchema for fields trait implementation
pub fn generate_sql_schema_field(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            #body
        }
    }
}

/// Generate method that forwards to original field
pub fn generate_forward_method(
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