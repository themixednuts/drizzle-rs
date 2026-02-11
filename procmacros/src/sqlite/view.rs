use crate::common::{
    count_primary_keys, make_uppercase_path, required_fields_pattern, struct_fields,
    table_name_from_attrs,
};
use crate::generators::generate_sql_table_info;
use crate::paths::{
    core as core_paths, ddl as ddl_paths, sqlite as sqlite_paths, std as std_paths,
};
use crate::sqlite::field::{FieldInfo, SQLiteType};
use crate::sqlite::generators::{
    generate_sql_schema, generate_sql_table, generate_sqlite_table, generate_sqlite_table_info,
    generate_to_sql,
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
                _ => {}
            }
            return Err(syn::Error::new(
                meta.span(),
                "Unrecognized view attribute.\n\
                 Supported attributes (case-insensitive):\n\
                 - NAME: Custom view name (e.g., #[SQLiteView(NAME = \"active_users\")])\n\
                 - DEFINITION: View definition SQL string or expression\n\
                   (e.g., #[SQLiteView(DEFINITION = \"SELECT ...\")])\n\
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
            "SQLiteView requires a DEFINITION unless marked as EXISTING",
        ));
    }

    let table_attrs = TableAttributes {
        name: Some(view_name.clone()),
        strict: false,
        without_rowid: false,
        crate_name: None,
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
        create_table_sql: String::new(),
        create_table_sql_runtime: None,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        attrs: &table_attrs,
        has_foreign_keys: false,
        is_composite_pk,
    };

    let (column_definitions, column_zst_idents) =
        column_definitions::generate_column_definitions(&ctx)?;
    let column_fields = column_definitions::generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors = column_definitions::generate_column_accessors(&ctx, &column_zst_idents)?;
    let model_definitions =
        models::generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let alias_definitions = alias::generate_aliased_table(&ctx)?;
    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = rusqlite::generate_rusqlite_impls(&ctx)?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote!();

    #[cfg(feature = "turso")]
    let turso_impls = turso::generate_turso_impls(&ctx)?;

    #[cfg(not(feature = "turso"))]
    let turso_impls = quote!();

    #[cfg(feature = "libsql")]
    let libsql_impls = libsql::generate_libsql_impls(&ctx)?;

    #[cfg(not(feature = "libsql"))]
    let libsql_impls = quote!();
    let view_marker_const = generate_view_marker_const(struct_ident, &attrs.marker_exprs);

    let view_name_lit = syn::LitStr::new(&view_name, proc_macro2::Span::call_site());
    let (definition_sql, definition_expr) = match &attrs.definition {
        Some(ViewDefinition::Literal(sql)) => (sql.clone(), None),
        Some(ViewDefinition::Expr(expr)) => (String::new(), Some(expr.clone())),
        None => (String::new(), None),
    };
    let definition_lit = syn::LitStr::new(&definition_sql, proc_macro2::Span::call_site());
    let has_definition_literal = matches!(attrs.definition, Some(ViewDefinition::Literal(_)));
    let is_existing = attrs.existing;

    let ddl_view_def = ddl_paths::sqlite::view_def();
    let mut ddl_view_expr = quote! { #ddl_view_def::new(#view_name_lit) };
    if has_definition_literal {
        ddl_view_expr = quote! { #ddl_view_expr.definition(#definition_lit) };
    }
    if attrs.existing {
        ddl_view_expr = quote! { #ddl_view_expr.existing() };
    }

    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_table_info = core_paths::sql_table_info();
    let sql_column_info = core_paths::sql_column_info();
    let sql_view = core_paths::sql_view();
    let sql_view_info = core_paths::sql_view_info();
    let sql_to_sql = core_paths::to_sql();
    let std_cow = std_paths::cow();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let _sqlite_table = sqlite_paths::sqlite_table();
    let sqlite_table_info = sqlite_paths::sqlite_table_info();
    let sqlite_column_info = sqlite_paths::sqlite_column_info();

    let columns_len = column_zst_idents.len();
    let sql_columns = quote! {
        #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
        #[allow(non_upper_case_globals)]
        static COLUMNS: [&'static dyn #sql_column_info; #columns_len] =
            [#(&#column_zst_idents,)*];
        &COLUMNS
    };
    let sqlite_columns = quote! {
        #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
        #[allow(non_upper_case_globals)]
        static SQLITE_COLUMNS: [&'static dyn #sqlite_column_info; #columns_len] =
            [#(&#column_zst_idents,)*];
        &SQLITE_COLUMNS
    };
    let sql_dependencies = quote! {
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sql_table_info; 0] = [];
        &DEPENDENCIES
    };
    let sqlite_dependencies = quote! {
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sqlite_table_info; 0] = [];
        &DEPENDENCIES
    };

    let sql_table_info_impl = generate_sql_table_info(
        struct_ident,
        quote! { Self::VIEW_NAME },
        quote! { ::std::option::Option::None },
        sql_columns,
        sql_dependencies,
    );
    let sqlite_table_info_impl = generate_sqlite_table_info(
        struct_ident,
        quote! { &<Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE },
        quote! { false },
        quote! { false },
        sqlite_columns,
        sqlite_dependencies,
    );

    let aliased_table_ident = format_ident!("Aliased{}", struct_ident);
    let select_model_ident = &ctx.select_model_ident;
    let insert_model_ident = &ctx.insert_model_ident;
    let update_model_ident = &ctx.update_model_ident;

    let sql_table_impl = generate_sql_table(
        struct_ident,
        quote! { #select_model_ident },
        quote! { #insert_model_ident<'a, T> },
        quote! { #update_model_ident<'a> },
        quote! { #aliased_table_ident },
    );
    let sqlite_table_impl = generate_sqlite_table(struct_ident, quote! { false }, quote! { false });
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

    Ok(quote! {
        #view_marker_const

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #struct_vis struct #struct_ident {
            #column_fields
        }

        impl #struct_ident {
            pub const VIEW_NAME: &'static str = #view_name_lit;
            pub const VIEW_DEFINITION_SQL: &'static str = #definition_lit;
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
        }

        #column_accessors
        #column_definitions
        #model_definitions
        #alias_definitions
        #rusqlite_impls
        #turso_impls
        #libsql_impls

        #sql_schema_impl
        #sql_table_impl
        #sql_table_info_impl
        #sqlite_table_info_impl
        #sqlite_table_impl
        #to_sql_impl
        #sql_view_impl
        #sql_view_info_impl
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
