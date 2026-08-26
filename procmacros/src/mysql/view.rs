use crate::common::ref_gen::ColumnRefFlags;
use crate::common::view_query::{self, ViewQuery};
use crate::common::{
    make_uppercase_path, required_fields_pattern, struct_fields, table_name_from_attrs,
};
use crate::generators::{DrizzleTableConfig, generate_drizzle_table};
use crate::mysql::field::FieldInfo;
use crate::mysql::generators::{
    SQLTableConfig, generate_mysql_table, generate_sql_schema, generate_sql_table, generate_to_sql,
};
use crate::mysql::table::{
    alias, attributes::TableAttributes, column_definitions, context::MacroContext, models,
};
use crate::paths::{core as core_paths, mysql as mysql_paths, std as std_paths};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{DeriveInput, Expr, ExprPath, Lit, Meta, Result, parse::Parse};

#[derive(Default)]
pub struct ViewAttributes {
    name: Option<String>,
    database: Option<String>,
    definition: Option<ViewDefinition>,
    existing: bool,
    algorithm: Option<ViewAlgorithm>,
    sql_security: Option<ViewSqlSecurity>,
    check_option: Option<ViewCheckOption>,
    marker_exprs: Vec<ExprPath>,
}

enum ViewDefinition {
    Literal(String),
    Expr(Expr),
    Query(ViewQuery),
}

#[derive(Clone, Copy)]
enum ViewAlgorithm {
    Undefined,
    Merge,
    Temptable,
}

#[derive(Clone, Copy)]
enum ViewSqlSecurity {
    Definer,
    Invoker,
}

#[derive(Clone, Copy)]
enum ViewCheckOption {
    Cascaded,
    Local,
}

impl Parse for ViewAttributes {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let mut attrs = Self::default();
        let metas = input.parse_terminated(Meta::parse, syn::Token![,])?;

        for meta in metas {
            match &meta {
                Meta::NameValue(value) => {
                    let Some(ident) = value.path.get_ident() else {
                        return Err(unrecognized(&meta));
                    };
                    let upper = ident.to_string().to_ascii_uppercase();
                    match upper.as_str() {
                        "NAME" => attrs.name = Some(string_value(value, "NAME")?),
                        "DATABASE" | "SCHEMA" => {
                            attrs.database = Some(string_value(value, upper.as_str())?)
                        }
                        "DEFINITION" => {
                            attrs.definition = Some(match &value.value {
                                Expr::Lit(lit) if matches!(lit.lit, Lit::Str(_)) => {
                                    let Lit::Str(literal) = &lit.lit else {
                                        unreachable!()
                                    };
                                    ViewDefinition::Literal(literal.value())
                                }
                                expression => ViewDefinition::Expr(expression.clone()),
                            });
                        }
                        "ALGORITHM" => {
                            attrs.algorithm = Some(parse_algorithm(value)?);
                        }
                        "SQL_SECURITY" | "SECURITY" => {
                            attrs.sql_security = Some(parse_security(value)?);
                        }
                        "CHECK_OPTION" | "WITH_CHECK_OPTION" => {
                            attrs.check_option = Some(parse_check_option(value)?);
                        }
                        _ => return Err(unrecognized(&meta)),
                    }
                    let marker = match upper.as_str() {
                        "SCHEMA" => "DATABASE",
                        "SECURITY" => "SQL_SECURITY",
                        "WITH_CHECK_OPTION" => "CHECK_OPTION",
                        other => other,
                    };
                    attrs.marker_exprs.push(make_uppercase_path(ident, marker));
                }
                Meta::List(list)
                    if list
                        .path
                        .get_ident()
                        .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("query")) =>
                {
                    if attrs.definition.is_some() {
                        return Err(syn::Error::new(
                            list.span(),
                            "cannot use both `query(...)` and `DEFINITION`",
                        ));
                    }
                    attrs.definition =
                        Some(ViewDefinition::Query(syn::parse2(list.tokens.clone())?));
                }
                Meta::Path(path) => {
                    let Some(ident) = path.get_ident() else {
                        return Err(unrecognized(&meta));
                    };
                    match ident.to_string().to_ascii_uppercase().as_str() {
                        "EXISTING" => {
                            attrs.existing = true;
                            attrs
                                .marker_exprs
                                .push(make_uppercase_path(ident, "EXISTING"));
                        }
                        "CHECK_OPTION" | "WITH_CHECK_OPTION" => {
                            attrs.check_option = Some(ViewCheckOption::Cascaded);
                            attrs
                                .marker_exprs
                                .push(make_uppercase_path(ident, "CHECK_OPTION"));
                        }
                        _ => return Err(unrecognized(&meta)),
                    }
                }
                _ => return Err(unrecognized(&meta)),
            }
        }

        if attrs.existing && attrs.definition.is_some() {
            return Err(input.error("EXISTING views cannot also specify DEFINITION or query(...)"));
        }
        if attrs.definition.is_none() && !attrs.existing {
            return Err(input
                .error("#[MySQLView] requires DEFINITION or query(...) unless marked EXISTING"));
        }

        Ok(attrs)
    }
}

fn string_value(value: &syn::MetaNameValue, name: &str) -> Result<String> {
    if let Expr::Lit(lit) = &value.value
        && let Lit::Str(literal) = &lit.lit
    {
        Ok(literal.value())
    } else {
        Err(syn::Error::new(
            value.span(),
            format!("{name} requires a string literal"),
        ))
    }
}

fn normalized_option(value: &syn::MetaNameValue, name: &str) -> Result<String> {
    Ok(string_value(value, name)?
        .to_ascii_uppercase()
        .replace([' ', '-'], "_"))
}

fn parse_algorithm(value: &syn::MetaNameValue) -> Result<ViewAlgorithm> {
    match normalized_option(value, "ALGORITHM")?.as_str() {
        "UNDEFINED" => Ok(ViewAlgorithm::Undefined),
        "MERGE" => Ok(ViewAlgorithm::Merge),
        "TEMPTABLE" | "TEMP_TABLE" => Ok(ViewAlgorithm::Temptable),
        _ => Err(syn::Error::new(
            value.span(),
            "ALGORITHM must be UNDEFINED, MERGE, or TEMPTABLE",
        )),
    }
}

fn parse_security(value: &syn::MetaNameValue) -> Result<ViewSqlSecurity> {
    match normalized_option(value, "SQL_SECURITY")?.as_str() {
        "DEFINER" => Ok(ViewSqlSecurity::Definer),
        "INVOKER" => Ok(ViewSqlSecurity::Invoker),
        _ => Err(syn::Error::new(
            value.span(),
            "SQL_SECURITY must be DEFINER or INVOKER",
        )),
    }
}

fn parse_check_option(value: &syn::MetaNameValue) -> Result<ViewCheckOption> {
    match normalized_option(value, "CHECK_OPTION")?.as_str() {
        "CASCADED" => Ok(ViewCheckOption::Cascaded),
        "LOCAL" => Ok(ViewCheckOption::Local),
        _ => Err(syn::Error::new(
            value.span(),
            "CHECK_OPTION must be CASCADED or LOCAL",
        )),
    }
}

fn unrecognized(meta: &Meta) -> syn::Error {
    syn::Error::new(
        meta.span(),
        "unrecognized MySQL view attribute; supported attributes are NAME, DATABASE/SCHEMA, \
         DEFINITION, query(...), EXISTING, ALGORITHM, SQL_SECURITY, and CHECK_OPTION/WITH_CHECK_OPTION",
    )
}

pub fn view_attr_macro(input: &DeriveInput, attrs: &ViewAttributes) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let fields = struct_fields(input, "MySQLView")?;
    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, false))
        .collect::<Result<Vec<_>>>()?;
    let view_name = table_name_from_attrs(struct_ident, attrs.name.clone());

    let table_attrs = TableAttributes {
        name: Some(view_name.clone()),
        database: attrs.database.clone(),
        temporary: false,
        engine: None,
        charset: None,
        collate: None,
        comment: None,
        composite_foreign_keys: Vec::new(),
        unique_constraints: Vec::new(),
        check_constraints: Vec::new(),
        marker_exprs: Vec::new(),
    };
    let required_fields_pattern = required_fields_pattern(&field_infos, |field| {
        MacroContext::is_field_optional_in_insert(field)
    });
    let ctx = MacroContext {
        struct_ident,
        struct_vis,
        table_name: view_name.clone(),
        table_comment: None,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        attrs: &table_attrs,
    };

    let (column_definitions, column_zst_idents) =
        column_definitions::generate_column_definitions(&ctx)?;
    let column_fields = column_definitions::generate_column_fields(&ctx, &column_zst_idents);
    let column_accessors = column_definitions::generate_column_accessors(&ctx, &column_zst_idents);
    let model_definitions =
        models::generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern);
    let alias_definitions = alias::generate_aliased_table(&ctx);
    let relations_impl = crate::common::constraints::generate_relations(
        ctx.field_infos,
        &ctx.attrs.composite_foreign_keys,
        struct_ident,
    )?;

    let view_marker_const = generate_view_marker_const(struct_ident, &attrs.marker_exprs);
    let view_name_lit = syn::LitStr::new(&view_name, proc_macro2::Span::call_site());
    let database_lit = attrs
        .database
        .as_ref()
        .map(|database| syn::LitStr::new(database, proc_macro2::Span::call_site()));
    let qualified_name = attrs.database.as_ref().map_or_else(
        || view_name.clone(),
        |database| format!("{database}.{view_name}"),
    );
    let ddl_qualified_name = attrs.database.as_ref().map_or_else(
        || quote_mysql_ident(&view_name),
        |database| {
            format!(
                "{}.{}",
                quote_mysql_ident(database),
                quote_mysql_ident(&view_name)
            )
        },
    );

    let (definition_sql, definition_expr, query_definition) = match &attrs.definition {
        Some(ViewDefinition::Literal(sql)) => (sql.clone(), None, None),
        Some(ViewDefinition::Expr(expression)) => (String::new(), Some(expression.clone()), None),
        Some(ViewDefinition::Query(query)) => (String::new(), None, Some(query)),
        None => (String::new(), None, None),
    };
    let query_const_sql = query_definition.map(|query| {
        let field_names = field_infos
            .iter()
            .map(|field| field.column_name.clone())
            .collect::<Vec<_>>();
        view_query::generate_const_sql(query, &field_names, view_query::Dialect::MySQL)
    });
    let query_validation = query_definition
        .map(|query| {
            view_query::generate_validation(query, field_infos.len(), view_query::Dialect::MySQL)
        })
        .transpose()?;
    let definition_lit = syn::LitStr::new(&definition_sql, proc_macro2::Span::call_site());
    let definition_const = query_const_sql
        .as_ref()
        .map_or_else(|| quote!(#definition_lit), Clone::clone);

    let algorithm = algorithm_tokens(attrs.algorithm);
    let sql_security = security_tokens(attrs.sql_security);
    let check_option = check_option_tokens(attrs.check_option);
    let database = database_lit.as_ref().map_or_else(
        || quote!(::core::option::Option::None),
        |database| quote!(::core::option::Option::Some(#database)),
    );
    let is_existing = attrs.existing;

    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_view = core_paths::sql_view();
    let sql_view_info = core_paths::sql_view_info();
    let no_primary_key = core_paths::no_primary_key();
    let no_constraint = core_paths::no_constraint();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let sql_to_sql = core_paths::to_sql();
    let std_cow = std_paths::cow();
    let mysql_value = mysql_paths::mysql_value();
    let mysql_schema_type = mysql_paths::mysql_schema_type();

    let table_ref = core_paths::table_ref();
    let column_ref = core_paths::column_ref();
    let column_flags = core_paths::column_flags();
    let column_dialect = core_paths::column_dialect();
    let table_dialect = core_paths::table_dialect();
    let column_names = field_infos
        .iter()
        .map(|field| &field.column_name)
        .collect::<Vec<_>>();
    let column_refs = field_infos.iter().map(|field| {
        let name = &field.column_name;
        let sql_type = field.sql_type_expr();
        let flags = ColumnRefFlags::new()
            .with(ColumnRefFlags::NOT_NULL, !field.is_nullable)
            .with(ColumnRefFlags::PRIMARY_KEY, field.is_primary())
            .with(ColumnRefFlags::UNIQUE, field.is_unique())
            .with(ColumnRefFlags::HAS_DEFAULT, field.has_default)
            .bits();
        let field_charset = option_str_tokens(field.charset.as_deref());
        let field_collation = option_str_tokens(field.collate.as_deref());
        quote! {
            #column_ref {
                table: Self::VIEW_NAME,
                name: #name,
                sql_type: #sql_type,
                flags: #column_flags::from_bits(#flags),
                dialect: #column_dialect::MySQL {
                    auto_increment: false,
                    default: ::core::option::Option::None,
                    generated_expression: ::core::option::Option::None,
                    generated_stored: false,
                    charset: #field_charset,
                    collate: #field_collation,
                    on_update: ::core::option::Option::None,
                },
            }
        }
    });
    let table_ref_const = quote! {
        const TABLE_REF: #table_ref = #table_ref {
            name: Self::VIEW_NAME,
            column_names: &[#(#column_names),*],
            schema: #database,
            qualified_name: #qualified_name,
            columns: &[#(#column_refs),*],
            primary_key: ::core::option::Option::None,
            foreign_keys: &[],
            constraints: &[],
            dependency_names: &[],
            dialect: #table_dialect::MySQL {
                is_temporary: false,
                engine: ::core::option::Option::None,
                charset: ::core::option::Option::None,
                collate: ::core::option::Option::None,
                comment: ::core::option::Option::None,
            },
        };
    };
    let drizzle_table_impl = generate_drizzle_table(DrizzleTableConfig {
        struct_ident,
        name: quote!(Self::VIEW_NAME),
        qualified_name: quote!(#qualified_name),
        schema: database.clone(),
        dependency_names: quote!(&[]),
        table_ref_const,
    });

    let select_model = &ctx.select_model_ident;
    let insert_model = &ctx.insert_model_ident;
    let update_model = &ctx.update_model_ident;
    let alias_type = format_ident!("{}Alias", struct_ident);
    let non_empty_marker = core_paths::non_empty_marker();
    let sql_table_impl = generate_sql_table(SQLTableConfig {
        struct_ident,
        select: quote!(#select_model),
        insert: quote!(#insert_model<'a, T>),
        update: quote!(#update_model<'a, #non_empty_marker>),
        aliased: quote!(#alias_type),
        foreign_keys: quote!((drizzle::core::NoForeignKey,)),
        primary_key: quote!(#no_primary_key),
        constraints: quote!(#no_constraint),
    });
    let mysql_table_impl = generate_mysql_table(struct_ident, &quote!(#ddl_qualified_name));
    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        &quote!(Self::VIEW_NAME),
        &quote!({
            static VIEW: #struct_ident = #struct_ident::new();
            #mysql_schema_type::View(&VIEW)
        }),
        &create_view_const_sql(
            attrs,
            &ddl_qualified_name,
            &definition_lit,
            query_const_sql.as_ref(),
        ),
    );
    let to_sql_impl = generate_to_sql(
        struct_ident,
        &quote!(#sql::table(<Self as drizzle::core::DrizzleTable>::TABLE_REF)),
    );

    let definition_body = definition_expr.as_ref().map_or_else(
        || quote!(#sql::raw(Self::VIEW_DEFINITION_SQL)),
        |expression| quote!(#sql_to_sql::to_sql(&#expression)),
    );
    let definition_info = if definition_expr.is_some() {
        quote! {
            #std_cow::Owned(
                <Self as #sql_view<'_, #mysql_schema_type, #mysql_value<'_>>>::definition(self)
                    .sql()
            )
        }
    } else {
        quote!(#std_cow::Borrowed(Self::VIEW_DEFINITION_SQL))
    };
    let columns_len = column_zst_idents.len();

    Ok(quote! {
        #view_marker_const

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #struct_vis struct #struct_ident {
            #column_fields
        }

        impl<'a> ::core::default::Default for &'a #struct_ident {
            fn default() -> Self {
                static VIEW: #struct_ident = #struct_ident::new();
                &VIEW
            }
        }

        impl #struct_ident {
            pub const VIEW_NAME: &'static str = #view_name_lit;
            pub const VIEW_DEFINITION_SQL: &'static str = #definition_const;

            #[must_use]
            pub fn create_view_sql() -> ::std::string::String {
                let view = Self::default();
                drizzle::mysql::common::create_view_sql(&view)
            }

            #[must_use]
            pub fn ddl_sql() -> ::std::string::String {
                let sql = <Self as #sql_schema<'_, #mysql_schema_type, #mysql_value<'_>>>::SQL;
                if sql.is_empty() { Self::create_view_sql() } else { sql.to_owned() }
            }
        }

        #column_accessors
        #column_definitions
        #model_definitions
        #alias_definitions
        #sql_schema_impl
        #sql_table_impl
        #drizzle_table_impl
        #mysql_table_impl
        #to_sql_impl
        #relations_impl

        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }

        impl<'a> #sql_view<'a, #mysql_schema_type, #mysql_value<'a>> for #struct_ident {
            fn definition(&self) -> #sql<'a, #mysql_value<'a>> {
                #definition_body
            }

            fn is_existing(&self) -> bool {
                #is_existing
            }
        }

        impl #sql_view_info for #struct_ident {
            fn definition_sql(&self) -> #std_cow<'static, str> {
                #definition_info
            }

            fn is_existing(&self) -> bool {
                #is_existing
            }
        }

        impl drizzle::mysql::common::MySQLViewInfo for #struct_ident {
            fn algorithm(&self) -> ::core::option::Option<drizzle::mysql::ViewAlgorithm> {
                #algorithm
            }

            fn sql_security(&self) -> ::core::option::Option<drizzle::mysql::ViewSqlSecurity> {
                #sql_security
            }

            fn check_option(&self) -> ::core::option::Option<drizzle::mysql::ViewCheckOption> {
                #check_option
            }

        }

        impl drizzle::mysql::index::__private::MySQLSchemaItemSealed for #struct_ident {}
        impl drizzle::mysql::index::MySQLSchemaItemMetadata for #struct_ident {}

        impl drizzle::core::HasSelectModel for #struct_ident {
            type SelectModel = #select_model;
            const COLUMN_COUNT: usize = #columns_len;
        }
        impl drizzle::core::IntoSelectTarget for #struct_ident {
            type Marker = drizzle::core::SelectStar;
        }

        #query_validation
    })
}

fn quote_mysql_ident(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

fn option_str_tokens(value: Option<&str>) -> TokenStream {
    value.map_or_else(
        || quote!(::core::option::Option::None),
        |value| quote!(::core::option::Option::Some(#value)),
    )
}

fn algorithm_tokens(value: Option<ViewAlgorithm>) -> TokenStream {
    match value {
        Some(ViewAlgorithm::Undefined) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewAlgorithm::Undefined
            ))
        }
        Some(ViewAlgorithm::Merge) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewAlgorithm::Merge
            ))
        }
        Some(ViewAlgorithm::Temptable) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewAlgorithm::Temptable
            ))
        }
        None => quote!(::core::option::Option::None),
    }
}

fn security_tokens(value: Option<ViewSqlSecurity>) -> TokenStream {
    match value {
        Some(ViewSqlSecurity::Definer) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewSqlSecurity::Definer
            ))
        }
        Some(ViewSqlSecurity::Invoker) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewSqlSecurity::Invoker
            ))
        }
        None => quote!(::core::option::Option::None),
    }
}

fn check_option_tokens(value: Option<ViewCheckOption>) -> TokenStream {
    match value {
        Some(ViewCheckOption::Cascaded) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewCheckOption::Cascaded
            ))
        }
        Some(ViewCheckOption::Local) => {
            quote!(::core::option::Option::Some(
                drizzle::mysql::ViewCheckOption::Local
            ))
        }
        None => quote!(::core::option::Option::None),
    }
}

fn create_view_const_sql(
    attrs: &ViewAttributes,
    qualified_name: &str,
    definition: &syn::LitStr,
    query_definition: Option<&TokenStream>,
) -> TokenStream {
    if attrs.existing || matches!(attrs.definition, Some(ViewDefinition::Expr(_))) {
        return quote!("");
    }
    let mut prefix = String::from("CREATE ");
    if let Some(algorithm) = attrs.algorithm {
        prefix.push_str(match algorithm {
            ViewAlgorithm::Undefined => "ALGORITHM=UNDEFINED ",
            ViewAlgorithm::Merge => "ALGORITHM=MERGE ",
            ViewAlgorithm::Temptable => "ALGORITHM=TEMPTABLE ",
        });
    }
    if let Some(security) = attrs.sql_security {
        prefix.push_str(match security {
            ViewSqlSecurity::Definer => "SQL SECURITY DEFINER ",
            ViewSqlSecurity::Invoker => "SQL SECURITY INVOKER ",
        });
    }
    prefix.push_str("VIEW ");
    prefix.push_str(qualified_name);
    prefix.push_str(" AS ");
    let suffix = attrs.check_option.map_or(";", |option| match option {
        ViewCheckOption::Cascaded => " WITH CASCADED CHECK OPTION;",
        ViewCheckOption::Local => " WITH LOCAL CHECK OPTION;",
    });
    if let Some(query) = query_definition {
        let const_format = crate::common::paths::const_format();
        quote!(#const_format::concatcp!(#prefix, #query, #suffix))
    } else {
        let sql = format!("{prefix}{}{suffix}", definition.value());
        quote!(#sql)
    }
}

fn generate_view_marker_const(struct_ident: &syn::Ident, markers: &[ExprPath]) -> TokenStream {
    if markers.is_empty() {
        return TokenStream::new();
    }
    let name = format_ident!("_VIEW_ATTR_MARKERS_{}", struct_ident);
    quote! {
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #name: () = {
            #(let _ = #markers;)*
        };
    }
}
