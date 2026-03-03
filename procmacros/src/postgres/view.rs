use crate::common::view_query::{self, ViewQuery};
use crate::common::{
    count_primary_keys, make_uppercase_path, required_fields_pattern, struct_fields,
    table_name_from_attrs,
};
use crate::generators::{DrizzleTableConfig, generate_drizzle_table};
use crate::paths::{
    core as core_paths, ddl as ddl_paths, postgres as postgres_paths, std as std_paths,
};
use crate::postgres::field::FieldInfo;
use crate::postgres::generators::{
    SQLTableConfig, generate_postgres_table, generate_sql_schema, generate_sql_table,
    generate_to_sql,
};
use crate::postgres::table::{
    alias, attributes::TableAttributes, column_definitions, context::MacroContext, drivers, models,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{DeriveInput, Expr, ExprPath, Meta, Result, parse::Parse};

#[derive(Default)]
pub struct ViewAttributes {
    pub(crate) name: Option<String>,
    pub(crate) schema: Option<String>,
    pub(crate) definition: Option<ViewDefinition>,
    pub(crate) materialized: bool,
    pub(crate) existing: bool,
    pub(crate) with_no_data: bool,
    pub(crate) using: Option<String>,
    pub(crate) tablespace: Option<String>,
    pub(crate) with_options: Option<Expr>,
    /// Original marker paths for IDE hover documentation
    pub(crate) marker_exprs: Vec<ExprPath>,
}

pub enum ViewDefinition {
    Literal(String),
    Expr(Expr),
    Query(ViewQuery),
}

impl Parse for ViewAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = ViewAttributes::default();
        let metas = input.parse_terminated(Meta::parse, syn::Token![,])?;

        for meta in metas {
            match &meta {
                Meta::NameValue(nv) => {
                    if let Some(ident) = nv.path.get_ident() {
                        let upper = ident.to_string().to_ascii_uppercase();
                        match upper.as_str() {
                            "NAME" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.name = Some(str_lit.value());
                                    attrs.marker_exprs.push(make_uppercase_path(ident, "NAME"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "NAME requires a string literal, e.g. NAME = \"active_users\"",
                                ));
                            }
                            "SCHEMA" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.schema = Some(str_lit.value());
                                    attrs
                                        .marker_exprs
                                        .push(make_uppercase_path(ident, "SCHEMA"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "SCHEMA requires a string literal, e.g. SCHEMA = \"auth\"",
                                ));
                            }
                            "DEFINITION" => {
                                let value = nv.clone().value;
                                if let syn::Expr::Lit(lit) = value.clone()
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.definition =
                                        Some(ViewDefinition::Literal(str_lit.value()));
                                    attrs
                                        .marker_exprs
                                        .push(make_uppercase_path(ident, "DEFINITION"));
                                    continue;
                                }
                                attrs.definition = Some(ViewDefinition::Expr(value));
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "DEFINITION"));
                                continue;
                            }
                            "USING" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.using = Some(str_lit.value());
                                    attrs.marker_exprs.push(make_uppercase_path(ident, "USING"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "USING requires a string literal, e.g. USING = \"heap\"",
                                ));
                            }
                            "TABLESPACE" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.tablespace = Some(str_lit.value());
                                    attrs
                                        .marker_exprs
                                        .push(make_uppercase_path(ident, "TABLESPACE"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "TABLESPACE requires a string literal, e.g. TABLESPACE = \"my_tablespace\"",
                                ));
                            }
                            "WITH" | "WITH_OPTIONS" => {
                                attrs.with_options = Some(nv.clone().value);
                                let marker = if upper == "WITH" {
                                    "WITH"
                                } else {
                                    "WITH_OPTIONS"
                                };
                                attrs.marker_exprs.push(make_uppercase_path(ident, marker));
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                Meta::List(list) => {
                    if let Some(ident) = list.path.get_ident() {
                        let lower = ident.to_string().to_ascii_lowercase();
                        if lower == "query" {
                            if attrs.definition.is_some() {
                                return Err(syn::Error::new(
                                    ident.span(),
                                    "cannot use both `query(...)` and `DEFINITION`",
                                ));
                            }
                            let query: ViewQuery = syn::parse2(list.tokens.clone())?;
                            attrs.definition = Some(ViewDefinition::Query(query));
                            continue;
                        }
                    }
                }
                Meta::Path(path) => {
                    if let Some(ident) = path.get_ident() {
                        let upper = ident.to_string().to_ascii_uppercase();
                        match upper.as_str() {
                            "MATERIALIZED" => {
                                attrs.materialized = true;
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "MATERIALIZED"));
                                continue;
                            }
                            "EXISTING" => {
                                attrs.existing = true;
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "EXISTING"));
                                continue;
                            }
                            "WITH_NO_DATA" => {
                                attrs.with_no_data = true;
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "WITH_NO_DATA"));
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
            }
            return Err(syn::Error::new(
                meta.span(),
                "unrecognized view attribute.\n\
                 Supported attributes (case-insensitive):\n\
                 - NAME: Custom view name (e.g., #[PostgresView(NAME = \"active_users\")])\n\
                 - SCHEMA: Custom schema name (e.g., #[PostgresView(SCHEMA = \"auth\")])\n\
                 - DEFINITION: View definition SQL string or expression\n\
                   (e.g., #[PostgresView(DEFINITION = \"SELECT ...\")])\n\
                 - query(...): Type-safe query DSL with compile-time SQL\n\
                 - MATERIALIZED: Mark as materialized view\n\
                 - WITH/WITH_OPTIONS: ViewWithOptionDef expression (e.g., #[PostgresView(WITH = ViewWithOptionDef::new().security_barrier())])\n\
                 - WITH_NO_DATA: Create materialized view WITH NO DATA\n\
                 - USING: USING clause for materialized views\n\
                 - TABLESPACE: Tablespace for materialized views\n\
                 - EXISTING: Mark view as existing (skip creation)\n\
                 ",
            ));
        }

        Ok(attrs)
    }
}

pub fn view_attr_macro(input: DeriveInput, attrs: ViewAttributes) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;

    let fields = struct_fields(&input, "PostgresView")?;

    let primary_key_count = count_primary_keys(fields, |field| {
        Ok(FieldInfo::from_field(field, false)?.is_primary)
    })?;
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    let view_name = table_name_from_attrs(struct_ident, attrs.name.clone());
    let view_schema = attrs.schema.clone().unwrap_or_else(|| "public".to_string());

    if attrs.definition.is_none() && !attrs.existing {
        return Err(syn::Error::new(
            input.span(),
            "#[PostgresView] requires a DEFINITION attribute unless marked as EXISTING.\n\
             Example: #[PostgresView(DEFINITION = \"SELECT * FROM users WHERE active\")]",
        ));
    }

    let table_attrs = TableAttributes {
        name: Some(view_name.clone()),
        schema: Some(view_schema.clone()),
        unlogged: false,
        temporary: false,
        inherits: None,
        tablespace: None,
        composite_foreign_keys: Vec::new(),
        marker_exprs: Vec::new(),
    };

    let required_fields_pattern = required_fields_pattern(&field_infos, |info| {
        info.is_nullable || info.has_default || info.default_fn.is_some() || info.is_serial
    });

    let ctx = MacroContext {
        struct_ident,
        struct_vis: &input.vis,
        table_name: view_name.clone(),
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        is_composite_pk,
        attrs: &table_attrs,
    };

    let (column_definitions, column_zst_idents) =
        column_definitions::generate_column_definitions(&ctx)?;
    let column_fields = column_definitions::generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors = column_definitions::generate_column_accessors(&ctx, &column_zst_idents)?;
    let model_definitions =
        models::generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let alias_definitions = alias::generate_aliased_table(&ctx)?;
    let driver_impls = drivers::generate_all_driver_impls(&ctx)?;

    // Generate FK ZSTs and relation impls (logical-only, no SQL constraints in views)
    let sql_table_info_path = core_paths::sql_table_info();
    let sql_column_info_path = core_paths::sql_column_info();
    let dialect_types = crate::common::constraints::DialectTypes {
        sql_schema: core_paths::sql_schema(),
        schema_type: postgres_paths::postgres_schema_type(),
        value_type: postgres_paths::postgres_value(),
    };
    let (foreign_key_impls, _sql_foreign_keys, foreign_keys_type, _fk_idents) =
        crate::common::constraints::generate_foreign_keys(
            ctx.field_infos,
            &ctx.attrs.composite_foreign_keys,
            &ctx.table_name,
            struct_ident,
            &input.vis,
            &sql_table_info_path,
            &sql_column_info_path,
            &dialect_types,
        )?;
    let relations_impl = crate::common::constraints::generate_relations(
        ctx.field_infos,
        &ctx.attrs.composite_foreign_keys,
        ctx.struct_ident,
    )?;
    let view_marker_const = generate_view_marker_const(struct_ident, &attrs.marker_exprs);

    let view_name_lit = syn::LitStr::new(&view_name, proc_macro2::Span::call_site());
    let view_schema_lit = syn::LitStr::new(&view_schema, proc_macro2::Span::call_site());
    let (definition_sql, definition_expr, query_def) = match &attrs.definition {
        Some(ViewDefinition::Literal(sql)) => (sql.clone(), None, None),
        Some(ViewDefinition::Expr(expr)) => (String::new(), Some(expr.clone()), None),
        Some(ViewDefinition::Query(q)) => (String::new(), None, Some(q)),
        None => (String::new(), None, None),
    };

    // For query definitions, generate the concatcp! expression for the SELECT SQL
    let query_const_select_sql = if let Some(q) = query_def {
        let field_names: Vec<String> = ctx
            .field_infos
            .iter()
            .map(|f| f.column_name.clone())
            .collect();
        Some(view_query::generate_const_sql(
            q,
            &field_names,
            view_query::Dialect::Postgres,
        )?)
    } else {
        None
    };

    // For query definitions, generate a validation block
    let query_validation = if let Some(q) = query_def {
        Some(view_query::generate_validation(
            q,
            ctx.field_infos.len(),
            view_query::Dialect::Postgres,
        )?)
    } else {
        None
    };

    let definition_lit = syn::LitStr::new(&definition_sql, proc_macro2::Span::call_site());
    // The const for VIEW_DEFINITION_SQL: either a string literal or a concatcp! expression
    let view_definition_sql_const: TokenStream =
        if let Some(ref select_sql) = query_const_select_sql {
            select_sql.clone()
        } else {
            quote! { #definition_lit }
        };
    let has_definition_literal = matches!(attrs.definition, Some(ViewDefinition::Literal(_)));
    let is_existing = attrs.existing;
    let is_materialized = attrs.materialized;
    let with_no_data = attrs.with_no_data;
    let using_clause_tokens = if let Some(using_clause) = attrs.using.as_ref() {
        let using_lit = syn::LitStr::new(using_clause, proc_macro2::Span::call_site());
        quote! { ::std::option::Option::Some(#using_lit) }
    } else {
        quote! { ::std::option::Option::None }
    };
    let tablespace_tokens = if let Some(tablespace) = attrs.tablespace.as_ref() {
        let tablespace_lit = syn::LitStr::new(tablespace, proc_macro2::Span::call_site());
        quote! { ::std::option::Option::Some(#tablespace_lit) }
    } else {
        quote! { ::std::option::Option::None }
    };

    let ddl_view_def = ddl_paths::postgres::view_def();
    let mut ddl_view_expr = quote! { #ddl_view_def::new(#view_schema_lit, #view_name_lit) };
    if has_definition_literal {
        ddl_view_expr = quote! { #ddl_view_expr.definition(#definition_lit) };
    }
    if attrs.materialized {
        ddl_view_expr = quote! { #ddl_view_expr.materialized() };
    }
    if let Some(with_options) = attrs.with_options.as_ref() {
        ddl_view_expr = quote! { #ddl_view_expr.with_options(#with_options) };
    }
    if attrs.existing {
        ddl_view_expr = quote! { #ddl_view_expr.existing() };
    }
    if attrs.with_no_data {
        ddl_view_expr = quote! { #ddl_view_expr.with_no_data() };
    }
    if let Some(using_clause) = attrs.using.as_ref() {
        let using_lit = syn::LitStr::new(using_clause, proc_macro2::Span::call_site());
        ddl_view_expr = quote! { #ddl_view_expr.using(#using_lit) };
    }
    if let Some(tablespace) = attrs.tablespace.as_ref() {
        let tablespace_lit = syn::LitStr::new(tablespace, proc_macro2::Span::call_site());
        ddl_view_expr = quote! { #ddl_view_expr.tablespace(#tablespace_lit) };
    }

    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let _sql_table_info = core_paths::sql_table_info();
    let _sql_column_info = core_paths::sql_column_info();
    let sql_view = core_paths::sql_view();
    let sql_view_info = core_paths::sql_view_info();
    let no_primary_key = core_paths::no_primary_key();
    let no_constraint = core_paths::no_constraint();
    let schema_item_tables = core_paths::schema_item_tables();
    let type_set_nil = core_paths::type_set_nil();
    let sql_to_sql = core_paths::to_sql();
    let std_cow = std_paths::cow();
    let postgres_value = postgres_paths::postgres_value();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    let columns_len = column_zst_idents.len();

    let qualified_view_name = format!("{}.{}", view_schema, view_name);

    // Generate TABLE_REF for view (minimal metadata)
    let table_ref_path = core_paths::table_ref();
    let column_ref_path = core_paths::column_ref();
    let column_dialect_path = core_paths::column_dialect();
    let table_dialect_path = core_paths::table_dialect();
    let view_column_ref_literals: Vec<proc_macro2::TokenStream> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let col_name = &f.column_name;
            let pg_type = f.column_type.to_sql_type();
            let not_null = !f.is_nullable;
            let primary_key = f.is_primary;
            let unique = f.is_unique;
            let has_default = f.has_default;
            let is_serial = f.is_serial;
            let is_bigserial = false;
            let is_generated_identity = f.is_generated_identity;
            let is_identity_always = f
                .identity_mode
                .as_ref()
                .is_some_and(|m| matches!(m, crate::postgres::field::IdentityMode::Always));
            quote! {
                #column_ref_path {
                    table: Self::VIEW_NAME,
                    name: #col_name,
                    sql_type: #pg_type,
                    not_null: #not_null,
                    primary_key: #primary_key,
                    unique: #unique,
                    has_default: #has_default,
                    dialect: #column_dialect_path::PostgreSQL {
                        postgres_type: #pg_type,
                        is_serial: #is_serial,
                        is_bigserial: #is_bigserial,
                        is_generated_identity: #is_generated_identity,
                        is_identity_always: #is_identity_always,
                    },
                }
            }
        })
        .collect();
    let view_col_names: Vec<&String> = ctx.field_infos.iter().map(|f| &f.column_name).collect();
    let view_table_ref_const = quote! {
        const TABLE_REF: #table_ref_path = #table_ref_path {
            name: Self::VIEW_NAME,
            column_names: &[#(#view_col_names),*],
            schema: ::core::option::Option::Some(Self::VIEW_SCHEMA),
            qualified_name: #qualified_view_name,
            columns: &[#(#view_column_ref_literals),*],
            primary_key: ::core::option::Option::None,
            foreign_keys: &[],
            constraints: &[],
            dependency_names: &[],
            dialect: #table_dialect_path::PostgreSQL,
        };
    };

    let drizzle_table_impl = generate_drizzle_table(DrizzleTableConfig {
        struct_ident,
        name: quote! { Self::VIEW_NAME },
        qualified_name: quote! { #qualified_view_name },
        schema: quote! { ::std::option::Option::Some(Self::VIEW_SCHEMA) },
        dependency_names: quote! { &[] },
        table_ref_const: view_table_ref_const,
    });
    let alias_type_ident = format_ident!("{}Alias", struct_ident);
    let select_model_ident = &ctx.select_model_ident;
    let insert_model_ident = &ctx.insert_model_ident;
    let update_model_ident = &ctx.update_model_ident;
    let non_empty_marker = core_paths::non_empty_marker();

    let sql_table_impl = generate_sql_table(SQLTableConfig {
        struct_ident,
        select: quote! { #select_model_ident },
        insert: quote! { #insert_model_ident<'a, T> },
        update: quote! { #update_model_ident<'a, #non_empty_marker> },
        aliased: quote! { #alias_type_ident },
        foreign_keys: foreign_keys_type,
        primary_key: quote! { #no_primary_key },
        constraints: quote! { #no_constraint },
    });
    let postgres_table_impl = generate_postgres_table(struct_ident);
    let view_const_sql = if has_definition_literal && attrs.with_options.is_none() {
        // Literal definition without runtime WITH options: build the entire SQL at proc-macro time
        let create_kw = if attrs.materialized {
            "CREATE MATERIALIZED VIEW"
        } else {
            "CREATE VIEW"
        };

        let mut sql = if view_schema != "public" {
            format!("{} \"{}\".\"{}\"", create_kw, view_schema, view_name)
        } else {
            format!("{} \"{}\"", create_kw, view_name)
        };
        if let Some(ref using) = attrs.using {
            sql.push_str(&format!(" USING {}", using));
        }
        if let Some(ref tablespace) = attrs.tablespace {
            sql.push_str(&format!(" TABLESPACE {}", tablespace));
        }
        sql.push_str(&format!(" AS {}", definition_sql));
        if attrs.with_no_data {
            sql.push_str(" WITH NO DATA");
        }

        quote! { #sql }
    } else if query_const_select_sql.is_some() && attrs.with_options.is_none() {
        // Query DSL: build CREATE VIEW using concatcp! with the generated SELECT SQL
        let create_kw = if attrs.materialized {
            "CREATE MATERIALIZED VIEW"
        } else {
            "CREATE VIEW"
        };

        let mut prefix = if view_schema != "public" {
            format!("{} \"{}\".\"{}\"", create_kw, view_schema, view_name)
        } else {
            format!("{} \"{}\"", create_kw, view_name)
        };
        if let Some(ref using) = attrs.using {
            prefix.push_str(&format!(" USING {}", using));
        }
        if let Some(ref tablespace) = attrs.tablespace {
            prefix.push_str(&format!(" TABLESPACE {}", tablespace));
        }
        prefix.push_str(" AS ");

        let select_sql = query_const_select_sql.as_ref().unwrap();
        if attrs.with_no_data {
            quote! {
                ::drizzle::const_format::concatcp!(#prefix, #select_sql, " WITH NO DATA")
            }
        } else {
            quote! {
                ::drizzle::const_format::concatcp!(#prefix, #select_sql)
            }
        }
    } else {
        quote! { "" }
    };
    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        quote! { Self::VIEW_NAME },
        quote! {
            {
                #[allow(non_upper_case_globals)]
                static VIEW_INSTANCE: #struct_ident = #struct_ident::new();
                #postgres_schema_type::View(&VIEW_INSTANCE)
            }
        },
        view_const_sql,
    );
    let table_ref = core_paths::table_ref();
    let view_column_names: Vec<&String> = ctx.field_infos.iter().map(|f| &f.column_name).collect();
    let to_sql_impl = generate_to_sql(
        struct_ident,
        quote! {
            #sql::table(#table_ref::sql(Self::VIEW_NAME, &[#(#view_column_names),*]))
        },
    );

    let sql_view_definition = if let Some(definition_expr) = definition_expr.as_ref() {
        quote! { #sql_to_sql::to_sql(&#definition_expr) }
    } else {
        // Both literal and query definitions use VIEW_DEFINITION_SQL (which is const)
        quote! { #sql::raw(Self::VIEW_DEFINITION_SQL) }
    };
    let sql_view_definition_sql = if definition_expr.is_some() {
        quote! {
            #std_cow::Owned(
                <Self as #sql_view<'_, #postgres_schema_type, #postgres_value<'_>>>::definition(self)
                    .sql()
            )
        }
    } else {
        quote! { #std_cow::Borrowed(Self::VIEW_DEFINITION_SQL) }
    };
    let sql_view_impl = quote! {
        impl<'a> #sql_view<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            fn definition(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql_view_definition
            }

            fn is_existing(&self) -> bool {
                #is_existing
            }
        }
    };

    let sql_view_info_impl = quote! {
        impl #sql_view_info for #struct_ident {
            fn definition_sql(&self) -> #std_cow<'static, str> {
                #sql_view_definition_sql
            }

            fn is_materialized(&self) -> bool {
                #is_materialized
            }

            fn is_existing(&self) -> bool {
                #is_existing
            }

            fn with_no_data(&self) -> ::std::option::Option<bool> {
                if #with_no_data {
                    ::std::option::Option::Some(true)
                } else {
                    ::std::option::Option::None
                }
            }

            fn using_clause(&self) -> ::std::option::Option<&'static str> {
                #using_clause_tokens
            }

            fn tablespace(&self) -> ::std::option::Option<&'static str> {
                #tablespace_tokens
            }
        }
    };

    // Generate query API code (relation ZSTs, accessors, FromJsonValue)
    #[cfg(feature = "query")]
    let query_api_impls = crate::postgres::table::generate_query_api_impls(&ctx)?;
    #[cfg(not(feature = "query"))]
    let query_api_impls = quote!();

    Ok(quote! {
        #view_marker_const

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #struct_vis struct #struct_ident {
            #column_fields
        }

        impl #struct_ident {
            pub const VIEW_NAME: &'static str = #view_name_lit;
            pub const VIEW_SCHEMA: &'static str = #view_schema_lit;
            pub const VIEW_DEFINITION_SQL: &'static str = #view_definition_sql_const;
            pub const DDL_VIEW: #ddl_view_def = #ddl_view_expr;

            pub fn create_view_sql() -> ::std::string::String {
                let mut view = <drizzle::ddl::postgres::ddl::View as ::std::convert::From<
                    drizzle::ddl::postgres::ddl::ViewDef,
                >>::from(Self::DDL_VIEW);
                let view_instance = Self::default();
                let definition = <Self as #sql_view_info>::definition_sql(&view_instance);
                if !definition.is_empty() {
                    view.definition = ::std::option::Option::Some(definition);
                }
                view.create_view_sql()
            }

            /// Returns the DDL SQL for creating this view.
            pub fn ddl_sql() -> ::std::string::String {
                let sql = <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL;
                if sql.is_empty() {
                    Self::create_view_sql()
                } else {
                    sql.to_string()
                }
            }
        }

        #column_accessors
        #column_definitions
        #foreign_key_impls
        #model_definitions
        #alias_definitions
        #driver_impls

        #sql_schema_impl
        #sql_table_impl
        #drizzle_table_impl
        #postgres_table_impl
        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }
        #to_sql_impl
        #relations_impl
        #sql_view_impl
        #sql_view_info_impl
        #query_api_impls

        impl drizzle::core::HasSelectModel for #struct_ident {
            type SelectModel = #select_model_ident;
            const COLUMN_COUNT: usize = #columns_len;
        }
        impl drizzle::core::IntoSelectTarget for #struct_ident {
            type Marker = drizzle::core::SelectStar;
        }

        #query_validation

    })
}

fn generate_view_marker_const(
    struct_ident: &syn::Ident,
    marker_exprs: &[syn::ExprPath],
) -> TokenStream {
    if marker_exprs.is_empty() {
        return TokenStream::new();
    }

    let marker_const_name = format_ident!("_VIEW_ATTR_MARKERS_{}", struct_ident);

    quote! {
        /// Hidden const that references the original view attribute markers.
        /// This enables IDE hover documentation for `#[PostgresView(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: () = {
            #( let _ = #marker_exprs; )*
        };
    }
}
