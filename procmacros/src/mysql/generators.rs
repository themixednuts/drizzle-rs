use crate::common::MySQLDialect;
use crate::common::generators as common_gen;
use crate::paths::mysql as mysql_paths;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn generate_to_sql(struct_ident: &Ident, body: &TokenStream) -> TokenStream {
    common_gen::generate_to_sql::<MySQLDialect>(struct_ident, body)
}

#[allow(clippy::too_many_arguments)]
pub fn generate_sql_column(
    struct_ident: &Ident,
    table: &TokenStream,
    table_type: &TokenStream,
    foreign_keys: &TokenStream,
    r#type: &TokenStream,
    primary_key: &TokenStream,
    not_null: &TokenStream,
    unique: &TokenStream,
    default: &TokenStream,
    default_fn: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_column::<MySQLDialect>(
        struct_ident,
        table,
        table_type,
        foreign_keys,
        r#type,
        primary_key,
        not_null,
        unique,
        default,
        default_fn,
    )
}

pub fn generate_mysql_column(
    struct_ident: &Ident,
    ddl_name: &TokenStream,
    auto_increment: &TokenStream,
    charset: &TokenStream,
    collate: &TokenStream,
) -> TokenStream {
    let mysql_column = mysql_paths::mysql_column();
    quote! {
        impl<'a> #mysql_column<'a> for #struct_ident {
            const DDL_NAME: &'static str = #ddl_name;
            const AUTO_INCREMENT: bool = #auto_increment;
            const CHARSET: ::core::option::Option<&'static str> = #charset;
            const COLLATE: ::core::option::Option<&'static str> = #collate;
        }
    }
}

pub fn generate_mysql_table(struct_ident: &Ident, ddl_qualified_name: &TokenStream) -> TokenStream {
    let mysql_table = mysql_paths::mysql_table();
    quote! {
        impl<'a> #mysql_table<'a> for #struct_ident {
            const DDL_QUALIFIED_NAME: &'static str = #ddl_qualified_name;
        }
    }
}

pub use common_gen::SQLTableConfig;

pub fn generate_sql_table(config: SQLTableConfig<'_>) -> TokenStream {
    common_gen::generate_sql_table::<MySQLDialect>(config)
}

pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: &TokenStream,
    r#type: &TokenStream,
    const_sql: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema::<MySQLDialect>(struct_ident, name, r#type, const_sql)
}

pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: &TokenStream,
    r#type: &TokenStream,
    sql: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema_field::<MySQLDialect>(struct_ident, name, r#type, sql)
}
