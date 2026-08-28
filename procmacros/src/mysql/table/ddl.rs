use super::context::MacroContext;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

pub(super) fn quoted(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

fn action_sql(prefix: &str, value: &Option<String>) -> String {
    value
        .as_ref()
        .map_or_else(String::new, |value| format!(" {prefix} {value}"))
}

/// Build the authoritative compile-time CREATE TABLE statement.
pub fn generate_schema_sql_const(ctx: &MacroContext<'_>) -> TokenStream {
    let const_format = crate::common::paths::const_format();
    let qualified = ctx.attrs.database.as_ref().map_or_else(
        || quoted(&ctx.table_name),
        |database| format!("{}.{}", quoted(database), quoted(&ctx.table_name)),
    );
    let temporary = if ctx.attrs.temporary {
        "TEMPORARY "
    } else {
        ""
    };
    let mut parts = vec![
        quote!("CREATE "),
        quote!(#temporary),
        quote!("TABLE "),
        quote!(#qualified),
        quote!(" ("),
    ];
    let mut first = true;
    let mut push_entry = |entry: TokenStream| {
        if !first {
            parts.push(quote!(", "));
        }
        first = false;
        parts.push(entry);
    };

    for field in ctx.field_infos {
        push_entry(field.sql_definition_expr());
    }

    let primary_columns = ctx
        .field_infos
        .iter()
        .filter(|field| field.is_primary())
        .collect::<Vec<_>>();
    if primary_columns.len() > 1 {
        let names = primary_columns
            .iter()
            .map(|field| quoted(&field.column_name))
            .collect::<Vec<_>>()
            .join(", ");
        push_entry(quote!(#const_format::concatcp!("PRIMARY KEY (", #names, ")")));
    }

    for field in ctx.field_infos {
        if let Some(reference) = &field.foreign_key {
            let source = quoted(&field.column_name);
            let target = &reference.table;
            let target_column = &reference.column;
            let target_column_type = format_ident!(
                "{}{}",
                target,
                target_column.to_string().to_upper_camel_case(),
            );
            let actions = format!(
                "{}{}",
                action_sql("ON DELETE", &reference.on_delete),
                action_sql("ON UPDATE", &reference.on_update),
            );
            push_entry(quote! {
                #const_format::concatcp!(
                    "FOREIGN KEY (", #source, ") REFERENCES ",
                    <#target as drizzle::mysql::traits::MySQLTable<'static>>::DDL_QUALIFIED_NAME,
                    " (",
                    <#target_column_type as drizzle::mysql::traits::MySQLColumn<'static>>::DDL_NAME,
                    ")", #actions
                )
            });
        }
    }

    for foreign_key in &ctx.attrs.composite_foreign_keys {
        let source = foreign_key
            .source_columns
            .iter()
            .map(|ident| {
                ctx.field_infos
                    .iter()
                    .find(|field| field.ident == *ident)
                    .map_or_else(
                        || quoted(&ident.to_string()),
                        |field| quoted(&field.column_name),
                    )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let target = &foreign_key.target_table;
        let target_names = foreign_key
            .target_columns
            .iter()
            .map(|column| {
                let target_column_type =
                    format_ident!("{}{}", target, column.to_string().to_upper_camel_case(),);
                quote!(
                    <#target_column_type as drizzle::mysql::traits::MySQLColumn<'static>>::DDL_NAME
                )
            })
            .collect::<Vec<_>>();
        let mut target_parts = Vec::new();
        for (index, target_name) in target_names.iter().enumerate() {
            if index > 0 {
                target_parts.push(quote!(", "));
            }
            target_parts.push(quote!(#target_name));
        }
        let actions = format!(
            "{}{}",
            action_sql("ON DELETE", &foreign_key.on_delete),
            action_sql("ON UPDATE", &foreign_key.on_update)
        );
        push_entry(quote! {
            #const_format::concatcp!(
                "FOREIGN KEY (", #source, ") REFERENCES ",
                <#target as drizzle::mysql::traits::MySQLTable<'static>>::DDL_QUALIFIED_NAME,
                " (", #(#target_parts),*, ")", #actions
            )
        });
    }

    for unique in &ctx.attrs.unique_constraints {
        let columns = unique
            .columns
            .iter()
            .map(|ident| {
                ctx.field_infos
                    .iter()
                    .find(|field| field.ident == *ident)
                    .map_or_else(
                        || quoted(&ident.to_string()),
                        |field| quoted(&field.column_name),
                    )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let entry = unique.name.as_ref().map_or_else(
            || format!("UNIQUE ({columns})"),
            |name| format!("CONSTRAINT {} UNIQUE ({columns})", quoted(name)),
        );
        push_entry(quote!(#entry));
    }

    for check in &ctx.attrs.check_constraints {
        let entry = check.name.as_ref().map_or_else(
            || format!("CHECK ({})", check.expr),
            |name| format!("CONSTRAINT {} CHECK ({})", quoted(name), check.expr),
        );
        push_entry(quote!(#entry));
    }

    parts.push(quote!(")"));
    if let Some(engine) = &ctx.attrs.engine {
        parts.push(quote!(" ENGINE="));
        parts.push(quote!(#engine));
    }
    if let Some(charset) = &ctx.attrs.charset {
        parts.push(quote!(" DEFAULT CHARACTER SET="));
        parts.push(quote!(#charset));
    }
    if let Some(collate) = &ctx.attrs.collate {
        parts.push(quote!(" COLLATE="));
        parts.push(quote!(#collate));
    }
    if let Some(comment) = ctx.attrs.comment.as_ref().or(ctx.table_comment.as_ref()) {
        let escaped = comment.replace('\\', "\\\\").replace('\'', "''");
        parts.push(quote!(" COMMENT='"));
        parts.push(quote!(#escaped));
        parts.push(quote!("'"));
    }
    parts.push(quote!(";"));

    quote!(#const_format::concatcp!(#(#parts),*))
}

pub fn generate_const_ddl(ctx: &MacroContext<'_>, _columns: &[Ident]) -> TokenStream {
    let struct_ident = ctx.struct_ident;
    quote! {
        impl #struct_ident {
            #[must_use]
            pub fn create_table_sql() -> ::std::string::String {
                <Self as drizzle::core::SQLSchema<'_, drizzle::mysql::common::MySQLSchemaType, drizzle::mysql::values::MySQLValue<'_>>>::SQL.to_owned()
            }

            #[must_use]
            pub fn ddl_sql() -> &'static str {
                <Self as drizzle::core::SQLSchema<'_, drizzle::mysql::common::MySQLSchemaType, drizzle::mysql::values::MySQLValue<'_>>>::SQL
            }
        }
    }
}
