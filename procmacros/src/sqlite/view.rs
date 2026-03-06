use crate::common::view_query::{self, ViewQuery};
use crate::common::{
    count_primary_keys, make_uppercase_path, required_fields_pattern, struct_fields,
    table_name_from_attrs,
};
use crate::generators::{DrizzleTableConfig, generate_drizzle_table};
use crate::paths::{
    core as core_paths, ddl as ddl_paths, sqlite as sqlite_paths, std as std_paths,
};
use crate::sqlite::field::{FieldInfo, SQLiteType};
use crate::sqlite::generators::{
    SQLTableConfig, generate_sql_schema, generate_sql_table, generate_sqlite_table, generate_to_sql,
};
#[cfg(feature = "libsql")]
use crate::sqlite::table::libsql;
#[cfg(feature = "rusqlite")]
use crate::sqlite::table::rusqlite;
#[cfg(feature = "turso")]
use crate::sqlite::table::turso;
use crate::sqlite::table::{
    alias, attributes::TableAttributes, column_definitions, context::MacroContext, models,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{DeriveInput, Expr, ExprPath, Meta, Result, parse::Parse};

#[derive(Default)]
pub struct ViewAttributes {
    pub(crate) name: Option<String>,
    pub(crate) definition: Option<ViewDefinition>,
    pub(crate) existing: bool,
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
                        if upper == "EXISTING" {
                            attrs.existing = true;
                            attrs
                                .marker_exprs
                                .push(make_uppercase_path(ident, "EXISTING"));
                            continue;
                        }
                    }
                }
            }
            return Err(syn::Error::new(
                meta.span(),
                "unrecognized view attribute.\n\
                 Supported attributes (case-insensitive):\n\
                 - NAME: Custom view name (e.g., #[SQLiteView(NAME = \"active_users\")])\n\
                 - DEFINITION: View definition SQL string or expression\n\
                   (e.g., #[SQLiteView(DEFINITION = \"SELECT ...\")])\n\
                 - query(...): Type-safe query DSL with compile-time SQL\n\
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

    let fields = struct_fields(&input, "SQLiteView")?;

    let primary_key_count = count_primary_keys(fields, |field| {
        Ok(FieldInfo::from_field(field, false)?.is_primary)
    })?;
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    let view_name = table_name_from_attrs(struct_ident, attrs.name.clone());

    if attrs.definition.is_none() && !attrs.existing {
        return Err(syn::Error::new(
            input.span(),
            "#[SQLiteView] requires a DEFINITION attribute unless marked as EXISTING.\n\
             Example: #[SQLiteView(DEFINITION = \"SELECT * FROM users WHERE active = 1\")]",
        ));
    }

    let table_attrs = TableAttributes {
        name: Some(view_name.clone()),
        strict: false,
        without_rowid: false,
        crate_name: None,
        composite_foreign_keys: Vec::new(),
        marker_exprs: Vec::new(),
    };

    let required_fields_pattern = required_fields_pattern(&field_infos, |info| {
        info.is_nullable
            || info.has_default
            || info.default_fn.is_some()
            || (info.is_primary
                && !table_attrs.without_rowid
                && !info.is_enum
                && matches!(info.column_type, SQLiteType::Integer))
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
        attrs: &table_attrs,
        is_composite_pk,
    };

    let (column_definitions, column_zst_idents) =
        column_definitions::generate_column_definitions(&ctx)?;
    let column_fields = column_definitions::generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors = column_definitions::generate_column_accessors(&ctx, &column_zst_idents)?;
    let model_definitions =
        models::generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let alias_definitions = alias::generate_aliased_table(&ctx)?;

    // Generate FK ZSTs and relation impls (logical-only, no SQL constraints in views)
    let sql_table_info_path = core_paths::sql_table_info();
    let sql_column_info_path = core_paths::sql_column_info();
    let dialect_types = crate::common::constraints::DialectTypes {
        sql_schema: core_paths::sql_schema(),
        schema_type: sqlite_paths::sqlite_schema_type(),
        value_type: sqlite_paths::sqlite_value(),
    };
    let (foreign_key_impls, _sql_foreign_keys, foreign_keys_type, _fk_idents) =
        crate::common::constraints::generate_foreign_keys(
            ctx.field_infos,
            &ctx.attrs.composite_foreign_keys,
            &ctx.table_name,
            struct_ident,
            struct_vis,
            &sql_table_info_path,
            &sql_column_info_path,
            &dialect_types,
        )?;
    let relations_impl = crate::common::constraints::generate_relations(
        ctx.field_infos,
        &ctx.attrs.composite_foreign_keys,
        ctx.struct_ident,
    )?;
    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = if crate::common::caller_has_dep("rusqlite") {
        rusqlite::generate_rusqlite_impls(&ctx)?
    } else {
        quote!()
    };

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote!();

    #[cfg(feature = "turso")]
    let turso_impls = if crate::common::caller_has_dep("turso") {
        turso::generate_turso_impls(&ctx)?
    } else {
        quote!()
    };

    #[cfg(not(feature = "turso"))]
    let turso_impls = quote!();

    #[cfg(feature = "libsql")]
    let libsql_impls = if crate::common::caller_has_dep("libsql") {
        libsql::generate_libsql_impls(&ctx)?
    } else {
        quote!()
    };

    #[cfg(not(feature = "libsql"))]
    let libsql_impls = quote!();
    let view_marker_const = generate_view_marker_const(struct_ident, &attrs.marker_exprs);

    let view_name_lit = syn::LitStr::new(&view_name, proc_macro2::Span::call_site());
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
            view_query::Dialect::SQLite,
        )?)
    } else {
        None
    };

    // For query definitions, generate a validation block
    let query_validation = if let Some(q) = query_def {
        Some(view_query::generate_validation(
            q,
            ctx.field_infos.len(),
            view_query::Dialect::SQLite,
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

    let ddl_view_def = ddl_paths::sqlite::view_def();
    let mut ddl_view_expr = quote! { #ddl_view_def::new(#view_name_lit) };
    if has_definition_literal {
        ddl_view_expr = quote! { #ddl_view_expr.definition(#definition_lit) };
    }
    // Query definitions: DDL definition is set at runtime from the const SQL
    if attrs.existing {
        ddl_view_expr = quote! { #ddl_view_expr.existing() };
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
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let columns_len = column_zst_idents.len();

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
            let sql_type = f.column_type.to_sql_type();
            let not_null = !f.is_nullable;
            let primary_key = f.is_primary;
            let unique = f.is_unique;
            let has_default = f.has_default;
            let autoincrement = f.is_autoincrement;
            quote! {
                #column_ref_path {
                    table: Self::VIEW_NAME,
                    name: #col_name,
                    sql_type: #sql_type,
                    not_null: #not_null,
                    primary_key: #primary_key,
                    unique: #unique,
                    has_default: #has_default,
                    dialect: #column_dialect_path::SQLite { autoincrement: #autoincrement },
                }
            }
        })
        .collect();
    let view_col_names: Vec<&String> = ctx.field_infos.iter().map(|f| &f.column_name).collect();
    let view_table_ref_const = quote! {
        const TABLE_REF: #table_ref_path = #table_ref_path {
            name: Self::VIEW_NAME,
            column_names: &[#(#view_col_names),*],
            schema: ::core::option::Option::None,
            qualified_name: Self::VIEW_NAME,
            columns: &[#(#view_column_ref_literals),*],
            primary_key: ::core::option::Option::None,
            foreign_keys: &[],
            constraints: &[],
            dependency_names: &[],
            dialect: #table_dialect_path::SQLite { without_rowid: false, strict: false },
        };
    };

    let drizzle_table_impl = generate_drizzle_table(DrizzleTableConfig {
        struct_ident,
        name: quote! { Self::VIEW_NAME },
        qualified_name: quote! { Self::VIEW_NAME },
        schema: quote! { ::std::option::Option::None },
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
    let sqlite_table_impl = generate_sqlite_table(struct_ident, quote! { false }, quote! { false });
    let view_const_sql = if has_definition_literal {
        // Literal definition: build the entire CREATE VIEW SQL at proc-macro time
        let sql = format!("CREATE VIEW \"{}\" AS {}", view_name, definition_sql);
        quote! { #sql }
    } else if let Some(ref select_sql) = query_const_select_sql {
        // Query DSL: build CREATE VIEW using concatcp! with the generated SELECT SQL
        let create_prefix = format!("CREATE VIEW \"{}\" AS ", view_name);
        quote! {
            ::drizzle::const_format::concatcp!(#create_prefix, #select_sql)
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
                #sqlite_schema_type::View(&VIEW_INSTANCE)
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
                <Self as #sql_view<'_, #sqlite_schema_type, #sqlite_value<'_>>>::definition(self)
                    .sql()
            )
        }
    } else {
        quote! { #std_cow::Borrowed(Self::VIEW_DEFINITION_SQL) }
    };
    let sql_view_impl = quote! {
        impl<'a> #sql_view<'a, #sqlite_schema_type, #sqlite_value<'a>> for #struct_ident {
            fn definition(&self) -> #sql<'a, #sqlite_value<'a>> {
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

            fn is_existing(&self) -> bool {
                #is_existing
            }
        }
    };

    // Generate query API code (relation ZSTs, accessors, FromJsonValue)
    #[cfg(feature = "query")]
    let query_api_impls = crate::sqlite::table::generate_query_api_impls(&ctx)?;
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
            pub const VIEW_DEFINITION_SQL: &'static str = #view_definition_sql_const;
            pub const DDL_VIEW: #ddl_view_def = #ddl_view_expr;

            pub fn create_view_sql() -> ::std::string::String {
                let mut view = <drizzle::ddl::sqlite::ddl::View as ::std::convert::From<
                    drizzle::ddl::sqlite::ddl::ViewDef,
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
                let sql = <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::SQL;
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
        #rusqlite_impls
        #turso_impls
        #libsql_impls

        #sql_schema_impl
        #sql_table_impl
        #drizzle_table_impl
        #sqlite_table_impl
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
        /// This enables IDE hover documentation for `#[SQLiteView(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: () = {
            #( let _ = #marker_exprs; )*
        };
    }
}
