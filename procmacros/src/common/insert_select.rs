use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};

/// Records the column's table as its expression source.
pub fn generate_column_scope_impl(column: &impl ToTokens, table: &Ident) -> TokenStream {
    quote! {
        impl drizzle::core::expr::ExprSources for #column {
            type Sources = drizzle::core::Src<#table>;
        }
    }
}

pub fn generate_column_impl(column: &impl ToTokens, table: &Ident) -> TokenStream {
    quote! {
        impl drizzle::core::InsertColumn<#table> for #column {
            type Column = Self;
        }

        impl drizzle::core::InsertColumn<&#table> for #column {
            type Column = Self;
        }
    }
}

pub fn generate_table_impls(
    table: &Ident,
    insertable_columns: &[&TokenStream],
    required_columns: &[&TokenStream],
    insertable_names: &[&String],
    all_columns_insertable: bool,
) -> TokenStream {
    let columns = insertable_columns.iter().rev().fold(
        quote! { drizzle::core::Nil },
        |tail, column| quote! { drizzle::core::Cons<#column, #tail> },
    );
    let required = required_columns.iter().rev().fold(
        quote! { drizzle::core::Nil },
        |tail, column| quote! { drizzle::core::Cons<#column, #tail> },
    );
    let all_columns_impl = all_columns_insertable.then(|| {
        quote! {
            impl drizzle::core::InsertSelectAllColumns for #table {}
        }
    });

    quote! {
        impl drizzle::core::InsertSelectTable for #table {
            type Columns = #columns;
            type RequiredColumns = #required;

            const INSERT_COLUMNS: &'static [&'static str] = &[#(#insertable_names),*];
        }
        #all_columns_impl
    }
}
