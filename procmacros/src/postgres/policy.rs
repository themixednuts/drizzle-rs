use crate::paths::{core as core_paths, ddl::postgres as ddl_paths, postgres as postgres_paths};
use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{DeriveInput, Error, Ident, LitStr, Meta, Result, Token, Type, parse::Parse};

/// Attributes for the `PostgresPolicy` attribute macro.
///
/// Syntax:
/// `#[PostgresPolicy(AS = "PERMISSIVE", FOR = "SELECT", TO("public"), USING = "...")]`
#[derive(Default)]
pub struct PolicyAttributes {
    pub name: Option<String>,
    pub as_clause: Option<String>,
    pub for_clause: Option<String>,
    pub to: Vec<String>,
    pub using: Option<String>,
    pub with_check: Option<String>,
}

enum RoleArg {
    Ident(Ident),
    String(LitStr),
}

impl Parse for RoleArg {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            return input.parse().map(Self::String);
        }
        input.parse().map(Self::Ident)
    }
}

impl Parse for PolicyAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = Self::default();
        if input.is_empty() {
            return Ok(attrs);
        }

        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        for meta in metas {
            match meta {
                Meta::NameValue(nv) => {
                    let Some(ident) = nv.path.get_ident() else {
                        return Err(Error::new_spanned(
                            nv.path,
                            "expected policy attribute name",
                        ));
                    };
                    let key = ident.to_string().to_ascii_uppercase();
                    let syn::Expr::Lit(expr_lit) = &nv.value else {
                        return Err(Error::new_spanned(
                            nv.value,
                            "policy attribute values must be string literals",
                        ));
                    };
                    let syn::Lit::Str(value) = &expr_lit.lit else {
                        return Err(Error::new_spanned(
                            &expr_lit.lit,
                            "policy attribute values must be string literals",
                        ));
                    };
                    match key.as_str() {
                        "NAME" => attrs.name = Some(value.value()),
                        "AS" | "AS_CLAUSE" => {
                            let clause = normalize_as_clause(&value.value(), value)?;
                            attrs.as_clause = Some(clause);
                        }
                        "FOR" | "FOR_CLAUSE" => {
                            let clause = normalize_for_clause(&value.value(), value)?;
                            attrs.for_clause = Some(clause);
                        }
                        "TO" => attrs.to.push(value.value()),
                        "USING" => attrs.using = Some(value.value()),
                        "WITH_CHECK" => attrs.with_check = Some(value.value()),
                        _ => {
                            return Err(Error::new_spanned(
                                ident,
                                "unrecognized PostgresPolicy attribute; expected NAME, AS, FOR, TO, USING, or WITH_CHECK",
                            ));
                        }
                    }
                }
                Meta::List(list)
                    if list
                        .path
                        .get_ident()
                        .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("TO")) =>
                {
                    let roles =
                        Punctuated::<RoleArg, Token![,]>::parse_terminated.parse2(list.tokens)?;
                    for role in roles {
                        attrs.to.push(match role {
                            RoleArg::Ident(ident) => ident.to_string(),
                            RoleArg::String(lit) => lit.value(),
                        });
                    }
                }
                _ => {
                    return Err(Error::new_spanned(
                        meta,
                        "unrecognized PostgresPolicy attribute; expected NAME, AS, FOR, TO(...), USING, or WITH_CHECK",
                    ));
                }
            }
        }

        Ok(attrs)
    }
}

fn normalize_as_clause(value: &str, span: &LitStr) -> Result<String> {
    let upper = value.to_ascii_uppercase();
    match upper.as_str() {
        "PERMISSIVE" | "RESTRICTIVE" => Ok(upper),
        _ => Err(Error::new_spanned(
            span,
            "AS must be PERMISSIVE or RESTRICTIVE",
        )),
    }
}

fn normalize_for_clause(value: &str, span: &LitStr) -> Result<String> {
    let upper = value.to_ascii_uppercase();
    match upper.as_str() {
        "ALL" | "SELECT" | "INSERT" | "UPDATE" | "DELETE" => Ok(upper),
        _ => Err(Error::new_spanned(
            span,
            "FOR must be ALL, SELECT, INSERT, UPDATE, or DELETE",
        )),
    }
}

fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn role_sql(role: &str) -> String {
    if role.eq_ignore_ascii_case("public") {
        "PUBLIC".to_string()
    } else {
        quote_ident(role)
    }
}

fn extract_table_type(input: &DeriveInput) -> Result<Type> {
    match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().expect("one field");
                match &field.ty {
                    Type::Path(_) => Ok(field.ty.clone()),
                    _ => Err(Error::new_spanned(
                        field,
                        "PostgresPolicy must reference a table type, e.g. struct UsersPolicy(Users);",
                    )),
                }
            }
            _ => Err(Error::new_spanned(
                input,
                "PostgresPolicy must be applied to a tuple struct with one table reference",
            )),
        },
        _ => Err(Error::new_spanned(
            input,
            "PostgresPolicy can only be applied to tuple structs",
        )),
    }
}

/// Generates the `PostgresPolicy` implementation.
pub fn postgres_policy_attr_macro(
    attr: &PolicyAttributes,
    input: &DeriveInput,
) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let table_type = extract_table_type(input)?;
    let policy_name = attr
        .name
        .clone()
        .unwrap_or_else(|| AsSnakeCase(struct_ident.to_string()).to_string());

    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_policy = core_paths::sql_policy();
    let drizzle_policy = core_paths::drizzle_policy();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let to_sql = core_paths::to_sql();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let policy_def = ddl_paths::policy_def();

    let as_clause = attr
        .as_clause
        .clone()
        .unwrap_or_else(|| "PERMISSIVE".to_string());
    let as_modifier = attr
        .as_clause
        .as_ref()
        .map_or_else(|| quote! {}, |clause| quote! { .as_clause(#clause) });
    let for_modifier = attr
        .for_clause
        .as_ref()
        .map_or_else(|| quote! {}, |clause| quote! { .for_clause(#clause) });
    let using_modifier = attr
        .using
        .as_ref()
        .map_or_else(|| quote! {}, |using| quote! { .using(#using) });
    let with_check_modifier = attr.with_check.as_ref().map_or_else(
        || quote! {},
        |with_check| quote! { .with_check(#with_check) },
    );
    let to_modifier = if attr.to.is_empty() {
        quote! {}
    } else {
        quote! { .to(Self::TO_ROLES) }
    };

    let role_values: Vec<_> = attr.to.iter().collect();
    let to_const = if attr.to.is_empty() {
        quote! { &[] }
    } else {
        quote! { &[#(#role_values),*] }
    };
    let as_const = attr.as_clause.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |clause| {
            quote! { ::core::option::Option::Some(#clause) }
        },
    );
    let for_const = attr.for_clause.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |clause| {
            quote! { ::core::option::Option::Some(#clause) }
        },
    );
    let using_const = attr.using.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |using| {
            quote! { ::core::option::Option::Some(#using) }
        },
    );
    let with_check_const = attr.with_check.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |with_check| {
            quote! { ::core::option::Option::Some(#with_check) }
        },
    );

    let role_sql = if attr.to.is_empty() {
        String::new()
    } else {
        format!(
            " TO {}",
            attr.to
                .iter()
                .map(|role| role_sql(role))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let for_sql = attr
        .for_clause
        .as_ref()
        .map_or_else(String::new, |clause| format!(" FOR {clause}"));
    let using_sql = attr
        .using
        .as_ref()
        .map_or_else(String::new, |using| format!(" USING ({using})"));
    let with_check_sql = attr
        .with_check
        .as_ref()
        .map_or_else(String::new, |with_check| {
            format!(" WITH CHECK ({with_check})")
        });
    let create_prefix = format!("CREATE POLICY {} ON \"", quote_ident(&policy_name));
    let dot_quote = "\".\"";
    let middle = format!("\" AS {as_clause}{for_sql}{role_sql}{using_sql}{with_check_sql};");
    let const_format = crate::common::paths::const_format();
    let const_sql = quote! {
        #const_format::concatcp!(
            #create_prefix,
            <#table_type>::DDL_TABLE.schema,
            #dot_quote,
            <#table_type>::DDL_TABLE.name,
            #middle
        )
    };

    Ok(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #struct_vis struct #struct_ident;

        impl #struct_ident {
            pub const TO_ROLES: &'static [&'static str] = #to_const;

            pub const DDL_POLICY: #policy_def = #policy_def::new(
                #table_type::DDL_TABLE.schema,
                #table_type::DDL_TABLE.name,
                #policy_name,
            )
            #as_modifier
            #for_modifier
            #to_modifier
            #using_modifier
            #with_check_modifier;

            pub const fn new() -> Self {
                Self
            }

            /// Generate CREATE POLICY SQL using the DDL definition.
            pub fn create_policy_sql() -> ::std::string::String {
                Self::DDL_POLICY.into_policy().create_policy_sql()
            }

            /// Returns the DDL SQL for creating this policy.
            pub fn ddl_sql() -> &'static str {
                <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL
            }
        }

        impl Default for #struct_ident {
            fn default() -> Self {
                Self::new()
            }
        }

        impl #drizzle_policy for #struct_ident {
            const POLICY_NAME: &'static str = #policy_name;
            const AS_CLAUSE: ::core::option::Option<&'static str> = #as_const;
            const FOR_CLAUSE: ::core::option::Option<&'static str> = #for_const;
            const TO: &'static [&'static str] = Self::TO_ROLES;
            const USING: ::core::option::Option<&'static str> = #using_const;
            const WITH_CHECK: ::core::option::Option<&'static str> = #with_check_const;

            fn table_ref() -> &'static drizzle::core::TableRef {
                &<#table_type as drizzle::core::DrizzleTable>::TABLE_REF
            }
        }

        impl<'a> #sql_policy<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            type Table = #table_type;
        }

        impl<'a> #sql_schema<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            const NAME: &'static str = #policy_name;
            const TYPE: #postgres_schema_type = {
                #[allow(non_upper_case_globals)]
                static POLICY_INSTANCE: #struct_ident = #struct_ident::new();
                #postgres_schema_type::Policy(&POLICY_INSTANCE)
            };
            const SQL: &'static str = #const_sql;
        }

        impl<'a> #to_sql<'a, #postgres_value<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::raw(Self::create_policy_sql())
            }
        }

        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }
    })
}
