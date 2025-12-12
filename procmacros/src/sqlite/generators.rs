use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate SQLite ToSQL trait implementation
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ToSQL<'a, SQLiteValue<'a>> for #struct_ident {
            fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
                #body
            }
        }
    }
}

/// Generate SQLite SQLColumn trait implementation
#[allow(clippy::too_many_arguments)]
pub fn generate_sql_column(
    struct_ident: &Ident,
    table: TokenStream,
    table_type: TokenStream,
    r#type: TokenStream,
    primary_key: TokenStream,
    not_null: TokenStream,
    unique: TokenStream,
    default: TokenStream,
    default_fn: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLColumn<'a, SQLiteValue<'a>> for #struct_ident {
            type Table = #table;
            type TableType = #table_type;
            type Type = #r#type;

            const PRIMARY_KEY: bool = #primary_key;
            const NOT_NULL: bool = #not_null;
            const UNIQUE: bool = #unique;
            const DEFAULT: Option<Self::Type> = #default;

            fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                #default_fn
            }
        }
    }
}

pub fn generate_sqlite_column_info(
    ident: &Ident,
    is_autoincrement: TokenStream,
    table: TokenStream,
    foreign_key: TokenStream,
) -> TokenStream {
    quote! {
        impl SQLiteColumnInfo for #ident {
            fn is_autoincrement(&self) -> bool {
                #is_autoincrement
            }

            fn table(&self) -> &dyn SQLiteTableInfo {
                #table
            }

            fn foreign_key(&self) -> Option<&'static dyn SQLiteColumnInfo> {
                #foreign_key
            }
        }
    }
}

/// Generate SQLite SQLiteColumn trait implementation
pub fn generate_sqlite_column(struct_ident: &Ident, is_autoincrement: TokenStream) -> TokenStream {
    quote! {
        impl<'a> SQLiteColumn<'a> for #struct_ident {
            const AUTOINCREMENT: bool = #is_autoincrement;
        }
    }
}

/// Generate SQLite SQLiteTableInfo trait implementation
pub fn generate_sqlite_table_info(
    struct_ident: &Ident,
    r#type: TokenStream,
    strict: TokenStream,
    without_rowid: TokenStream,
    columns: TokenStream,
) -> TokenStream {
    quote! {
        impl SQLiteTableInfo for #struct_ident {
            fn r#type(&self) -> &SQLiteSchemaType {
                #r#type
            }

            fn strict(&self) -> bool {
                #strict
            }
            fn without_rowid(&self) -> bool {
                #without_rowid
            }
            fn sqlite_columns(&self) -> &'static [&'static dyn SQLiteColumnInfo] {
                #columns
            }

            fn sqlite_dependencies(&self) -> Box<[&'static dyn SQLiteTableInfo]> {
                SQLiteTableInfo::sqlite_columns(self)
                    .iter()
                    .filter_map(|col| SQLiteColumnInfo::foreign_key(*col))
                    .map(|fk_col| SQLiteColumnInfo::table(fk_col))
                    .collect()
            }
        }
    }
}

/// Generate SQLite SQLiteTable trait implementation
pub fn generate_sqlite_table(
    struct_ident: &Ident,
    without_rowid: TokenStream,
    strict: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLiteTable<'a> for #struct_ident {
            const WITHOUT_ROWID: bool = #without_rowid;
            const STRICT: bool = #strict;
        }
    }
}

/// Generate SQLite SQLTable trait implementation
pub fn generate_sql_table(
    struct_ident: &Ident,
    select: TokenStream,
    insert: TokenStream,
    update: TokenStream,
    aliased: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>> for #struct_ident {
            type Select = #select;
            type Insert<T> = #insert;
            type Update = #update;
            type Aliased = #aliased;

            fn alias(name: &'static str) -> Self::Aliased {
                #aliased::new(name)
            }
        }
    }
}

/// Generate SQLite SQLSchema trait implementation
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    const_sql: TokenStream,
    runtime_sql: Option<TokenStream>,
) -> TokenStream {
    let fn_method = runtime_sql
        .map(|v| {
            quote! {
                fn sql(&self) -> SQL<'a, SQLiteValue<'a>> {
                    #v
                }
            }
        })
        .unwrap_or_else(|| quote! {});
    quote! {
        impl<'a> SQLSchema<'a, SQLiteSchemaType, SQLiteValue<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: SQLiteSchemaType = #r#type;
            const SQL: &'static str = #const_sql;
            #fn_method
        }
    }
}

/// Generate SQLite SQLSchema for fields trait implementation
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    sql: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLSchema<'a, &'a str, SQLiteValue<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: &'a str = #r#type;
            const SQL: &'static str = "";

            fn sql(&self) -> SQL<'a, SQLiteValue<'a>> {
                SQL::raw(#sql)
            }
        }
    }
}
