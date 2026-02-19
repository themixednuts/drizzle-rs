use crate::common::{
    count_primary_keys, make_uppercase_path, required_fields_pattern, struct_fields,
    table_name_from_attrs,
};
use crate::generators::{SQLTableInfoConfig, generate_sql_table_info};
use crate::paths::{
    core as core_paths, ddl as ddl_paths, postgres as postgres_paths, std as std_paths,
};
use crate::postgres::field::FieldInfo;
use crate::postgres::generators::{
    SQLTableConfig, generate_postgres_table, generate_postgres_table_info, generate_sql_schema,
    generate_sql_table, generate_to_sql,
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

#[derive(Clone)]
pub enum ViewDefinition {
    Literal(String),
    Expr(Expr),
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
                                    "Expected a string literal for 'NAME'",
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
                                    "Expected a string literal for 'SCHEMA'",
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
                                    "Expected a string literal for 'USING'",
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
                                    "Expected a string literal for 'TABLESPACE'",
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
                _ => {}
            }
            return Err(syn::Error::new(
                meta.span(),
                "Unrecognized view attribute.\n\
                 Supported attributes (case-insensitive):\n\
                 - NAME: Custom view name (e.g., #[PostgresView(NAME = \"active_users\")])\n\
                 - SCHEMA: Custom schema name (e.g., #[PostgresView(SCHEMA = \"auth\")])\n\
                 - DEFINITION: View definition SQL string or expression\n\
                   (e.g., #[PostgresView(DEFINITION = \"SELECT ...\")])\n\
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
            "PostgresView requires a DEFINITION unless marked as EXISTING",
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
        create_table_sql: String::new(),
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        has_foreign_keys: false,
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
    let view_marker_const = generate_view_marker_const(struct_ident, &attrs.marker_exprs);

    let view_name_lit = syn::LitStr::new(&view_name, proc_macro2::Span::call_site());
    let view_schema_lit = syn::LitStr::new(&view_schema, proc_macro2::Span::call_site());
    let (definition_sql, definition_expr) = match &attrs.definition {
        Some(ViewDefinition::Literal(sql)) => (sql.clone(), None),
        Some(ViewDefinition::Expr(expr)) => (String::new(), Some(expr.clone())),
        None => (String::new(), None),
    };
    let definition_lit = syn::LitStr::new(&definition_sql, proc_macro2::Span::call_site());
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
    let sql_table_info = core_paths::sql_table_info();
    let sql_column_info = core_paths::sql_column_info();
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
    let postgres_table_info = postgres_paths::postgres_table_info();
    let postgres_column_info = postgres_paths::postgres_column_info();

    let columns_len = column_zst_idents.len();
    let sql_columns = quote! {
        #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
        #[allow(non_upper_case_globals)]
        static COLUMNS: [&'static dyn #sql_column_info; #columns_len] =
            [#(&#column_zst_idents,)*];
        &COLUMNS
    };
    let postgres_columns = quote! {
        #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
        #[allow(non_upper_case_globals)]
        static POSTGRES_COLUMNS: [&'static dyn #postgres_column_info; #columns_len] =
            [#(&#column_zst_idents,)*];
        &POSTGRES_COLUMNS
    };
    let sql_dependencies = quote! {
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sql_table_info; 0] = [];
        &DEPENDENCIES
    };
    let postgres_dependencies = quote! {
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #postgres_table_info; 0] = [];
        &DEPENDENCIES
    };

    let sql_table_info_impl = generate_sql_table_info(SQLTableInfoConfig {
        struct_ident,
        name: quote! { Self::VIEW_NAME },
        schema: quote! { ::std::option::Option::Some(Self::VIEW_SCHEMA) },
        columns: sql_columns,
        primary_key: quote! { ::std::option::Option::None },
        foreign_keys: quote! { &[] },
        constraints: quote! { &[] },
        dependencies: sql_dependencies,
    });
    let postgres_table_info_impl = generate_postgres_table_info(
        struct_ident,
        quote! { &<Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::TYPE },
        postgres_columns,
        postgres_dependencies,
    );

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
        foreign_keys: quote! { () },
        primary_key: quote! { #no_primary_key },
        constraints: quote! { #no_constraint },
    });
    let postgres_table_impl = generate_postgres_table(struct_ident);
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
        quote! { "" },
        Some(quote! { #sql::raw(Self::create_view_sql()) }),
    );
    let to_sql_impl = generate_to_sql(
        struct_ident,
        quote! {
            static INSTANCE: #struct_ident = #struct_ident::new();
            #sql::table(&INSTANCE)
        },
    );

    let sql_view_definition = if let Some(definition_expr) = definition_expr.as_ref() {
        quote! { #sql_to_sql::to_sql(&#definition_expr) }
    } else {
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

    Ok(quote! {
        #view_marker_const

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #struct_vis struct #struct_ident {
            #column_fields
        }

        impl #struct_ident {
            pub const VIEW_NAME: &'static str = #view_name_lit;
            pub const VIEW_SCHEMA: &'static str = #view_schema_lit;
            pub const VIEW_DEFINITION_SQL: &'static str = #definition_lit;
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
        }

        #column_accessors
        #column_definitions
        #model_definitions
        #alias_definitions
        #driver_impls

        #sql_schema_impl
        #sql_table_impl
        #sql_table_info_impl
        #postgres_table_info_impl
        #postgres_table_impl
        impl #schema_item_tables for #struct_ident {
            type Tables = #type_set_nil;
        }
        #to_sql_impl
        #sql_view_impl
        #sql_view_info_impl

        impl drizzle::core::HasSelectModel for #struct_ident {
            type SelectModel = #select_model_ident;
            const COLUMN_COUNT: usize = #columns_len;
        }
        impl drizzle::core::IntoSelectTarget for #struct_ident {
            type Marker = drizzle::core::SelectStar;
        }

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
