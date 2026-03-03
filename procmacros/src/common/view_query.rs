//! View query DSL — compile-time SQL generation for view definitions.
//!
//! Parses a `query(...)` attribute into an AST, then generates:
//! 1. A `concatcp!()` expression for compile-time SQL (`&'static str`)
//! 2. A `const _: () = { ... }` validation block for type-checking expressions

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Lit, Path, Result, Token, bracketed, parenthesized};

// =============================================================================
// AST TYPES
// =============================================================================

/// The full parsed query DSL.
pub struct ViewQuery {
    pub select: Vec<SelectItem>,
    pub from: Path,
    pub joins: Vec<JoinClause>,
    pub filter: Option<QueryExpr>,
    pub group_by: Vec<Path>,
    pub having: Option<QueryExpr>,
    pub order_by: Vec<QueryExpr>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// A single item in the SELECT list.
pub enum SelectItem {
    /// A bare column reference: `Table::col`
    Column(Path),
    /// An expression like `count(Table::col)` or `count_all()`
    Expr(QueryExpr),
}

/// A JOIN clause.
pub struct JoinClause {
    pub kind: JoinKind,
    pub table: Path,
    pub condition: Option<QueryExpr>,
}

#[derive(Clone, Copy)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// An expression in filter/join/having/order_by positions.
pub enum QueryExpr {
    Column(Path),
    Literal(Lit),
    BinaryOp {
        op: BinOp,
        left: Box<QueryExpr>,
        right: Box<QueryExpr>,
    },
    IsNull {
        expr: Box<QueryExpr>,
        negated: bool,
    },
    Between {
        expr: Box<QueryExpr>,
        low: Box<QueryExpr>,
        high: Box<QueryExpr>,
        negated: bool,
    },
    InArray {
        expr: Box<QueryExpr>,
        values: Vec<QueryExpr>,
    },
    And(Vec<QueryExpr>),
    Or(Vec<QueryExpr>),
    Not(Box<QueryExpr>),
    Aggregate {
        func: AggFunc,
        expr: Option<Box<QueryExpr>>,
    },
    Asc(Box<QueryExpr>),
    Desc(Box<QueryExpr>),
}

#[derive(Clone, Copy)]
pub enum BinOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    NotLike,
}

#[derive(Clone, Copy)]
pub enum AggFunc {
    Count,
    CountAll,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
}

/// SQL dialect for code generation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    SQLite,
    Postgres,
}

// =============================================================================
// PARSER
// =============================================================================

impl Parse for ViewQuery {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut select = None;
        let mut from = None;
        let mut joins = Vec::new();
        let mut filter = None;
        let mut group_by = None;
        let mut having = None;
        let mut order_by = None;
        let mut limit = None;
        let mut offset = None;

        // Parse comma-separated clause calls
        while !input.is_empty() {
            let clause_ident: Ident = input.parse()?;
            let clause_name = clause_ident.to_string();

            let content;
            parenthesized!(content in input);

            match clause_name.as_str() {
                "select" => {
                    if select.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `select` clause",
                        ));
                    }
                    let items = parse_comma_separated(&content, parse_select_item)?;
                    if items.is_empty() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "`select` must have at least one item",
                        ));
                    }
                    select = Some(items);
                }
                "from" => {
                    if from.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `from` clause",
                        ));
                    }
                    from = Some(content.parse::<Path>()?);
                }
                "join" | "inner_join" => {
                    joins.push(parse_join_clause(&content, JoinKind::Inner)?);
                }
                "left_join" => {
                    joins.push(parse_join_clause(&content, JoinKind::Left)?);
                }
                "right_join" => {
                    joins.push(parse_join_clause(&content, JoinKind::Right)?);
                }
                "full_join" => {
                    joins.push(parse_join_clause(&content, JoinKind::Full)?);
                }
                "cross_join" => {
                    let table = content.parse::<Path>()?;
                    joins.push(JoinClause {
                        kind: JoinKind::Cross,
                        table,
                        condition: None,
                    });
                }
                "filter" => {
                    if filter.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `filter` clause",
                        ));
                    }
                    filter = Some(parse_query_expr(&content)?);
                }
                "group_by" => {
                    if group_by.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `group_by` clause",
                        ));
                    }
                    let paths = parse_comma_separated(&content, |input| input.parse::<Path>())?;
                    group_by = Some(paths);
                }
                "having" => {
                    if having.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `having` clause",
                        ));
                    }
                    having = Some(parse_query_expr(&content)?);
                }
                "order_by" => {
                    if order_by.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `order_by` clause",
                        ));
                    }
                    order_by = Some(parse_comma_separated(&content, parse_query_expr)?);
                }
                "limit" => {
                    if limit.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `limit` clause",
                        ));
                    }
                    let lit: syn::LitInt = content.parse()?;
                    limit = Some(lit.base10_parse()?);
                }
                "offset" => {
                    if offset.is_some() {
                        return Err(syn::Error::new(
                            clause_ident.span(),
                            "duplicate `offset` clause",
                        ));
                    }
                    let lit: syn::LitInt = content.parse()?;
                    offset = Some(lit.base10_parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        clause_ident.span(),
                        format!("unknown query clause `{}`", other),
                    ));
                }
            }

            // Consume optional trailing comma
            let _ = input.parse::<Option<Token![,]>>();
        }

        let select = select.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "query requires a `select(...)` clause",
            )
        })?;
        let from = from.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "query requires a `from(...)` clause",
            )
        })?;

        Ok(ViewQuery {
            select,
            from,
            joins,
            filter,
            group_by: group_by.unwrap_or_default(),
            having,
            order_by: order_by.unwrap_or_default(),
            limit,
            offset,
        })
    }
}

/// Parse comma-separated items using a given parser function.
fn parse_comma_separated<T>(
    input: ParseStream,
    parser: fn(ParseStream) -> Result<T>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();
    while !input.is_empty() {
        items.push(parser(input)?);
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(items)
}

/// Parse a single SELECT item (column path or expression).
fn parse_select_item(input: ParseStream) -> Result<SelectItem> {
    // Peek: if the next token is an identifier that looks like a function name
    // (followed by parentheses), parse as expression; otherwise as column path.
    if input.peek(Ident) && input.peek2(syn::token::Paren) {
        let ident_str = input.fork().parse::<Ident>()?.to_string();
        if is_expr_function(&ident_str) {
            return Ok(SelectItem::Expr(parse_query_expr(input)?));
        }
    }
    Ok(SelectItem::Column(input.parse::<Path>()?))
}

/// Parse a JOIN clause: `(Table, condition_expr)`
fn parse_join_clause(input: ParseStream, kind: JoinKind) -> Result<JoinClause> {
    let table = input.parse::<Path>()?;
    let condition = if !input.is_empty() {
        input.parse::<Token![,]>()?;
        Some(parse_query_expr(input)?)
    } else {
        None
    };
    Ok(JoinClause {
        kind,
        table,
        condition,
    })
}

/// Parse a query expression (recursive).
fn parse_query_expr(input: ParseStream) -> Result<QueryExpr> {
    // Check for function call: `ident(...)`
    if input.peek(Ident) && input.peek2(syn::token::Paren) {
        let fork = input.fork();
        let ident: Ident = fork.parse()?;
        let ident_str = ident.to_string();

        if is_expr_function(&ident_str) {
            // Consume from real stream
            let ident: Ident = input.parse()?;
            let content;
            parenthesized!(content in input);
            return parse_function_expr(&ident, &content);
        }
    }

    // Check for literal
    if input.peek(Lit) {
        return Ok(QueryExpr::Literal(input.parse()?));
    }

    // Check for boolean literals (syn parses true/false as LitBool)
    if input.peek(syn::LitBool) {
        let lit_bool: syn::LitBool = input.parse()?;
        return Ok(QueryExpr::Literal(Lit::Bool(lit_bool)));
    }

    // Otherwise, parse as column path (Table::col)
    Ok(QueryExpr::Column(input.parse::<Path>()?))
}

/// Is this identifier a known expression function?
fn is_expr_function(name: &str) -> bool {
    matches!(
        name,
        "eq" | "neq"
            | "gt"
            | "gte"
            | "lt"
            | "lte"
            | "like"
            | "not_like"
            | "is_null"
            | "is_not_null"
            | "between"
            | "not_between"
            | "in_array"
            | "and"
            | "or"
            | "not"
            | "count"
            | "count_all"
            | "count_distinct"
            | "sum"
            | "avg"
            | "min"
            | "max"
            | "asc"
            | "desc"
    )
}

/// Parse a function call expression given its name and argument tokens.
fn parse_function_expr(ident: &Ident, content: ParseStream) -> Result<QueryExpr> {
    let name = ident.to_string();
    match name.as_str() {
        // Binary ops: op(left, right)
        "eq" => parse_binary(content, BinOp::Eq),
        "neq" => parse_binary(content, BinOp::Neq),
        "gt" => parse_binary(content, BinOp::Gt),
        "gte" => parse_binary(content, BinOp::Gte),
        "lt" => parse_binary(content, BinOp::Lt),
        "lte" => parse_binary(content, BinOp::Lte),
        "like" => parse_binary(content, BinOp::Like),
        "not_like" => parse_binary(content, BinOp::NotLike),

        // Null checks: is_null(expr), is_not_null(expr)
        "is_null" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::IsNull {
                expr: Box::new(expr),
                negated: false,
            })
        }
        "is_not_null" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::IsNull {
                expr: Box::new(expr),
                negated: true,
            })
        }

        // Between: between(expr, low, high)
        "between" => parse_between(content, false),
        "not_between" => parse_between(content, true),

        // In: in_array(expr, [v1, v2, ...])
        "in_array" => {
            let expr = parse_query_expr(content)?;
            content.parse::<Token![,]>()?;
            let inner;
            bracketed!(inner in content);
            let values = parse_comma_separated(&inner, parse_query_expr)?;
            Ok(QueryExpr::InArray {
                expr: Box::new(expr),
                values,
            })
        }

        // Logical: and(a, b) or and([a, b, c])
        "and" => parse_logical_list(content, true),
        "or" => parse_logical_list(content, false),
        "not" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Not(Box::new(expr)))
        }

        // Aggregates
        "count" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Aggregate {
                func: AggFunc::Count,
                expr: Some(Box::new(expr)),
            })
        }
        "count_all" => Ok(QueryExpr::Aggregate {
            func: AggFunc::CountAll,
            expr: None,
        }),
        "count_distinct" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Aggregate {
                func: AggFunc::CountDistinct,
                expr: Some(Box::new(expr)),
            })
        }
        "sum" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Aggregate {
                func: AggFunc::Sum,
                expr: Some(Box::new(expr)),
            })
        }
        "avg" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Aggregate {
                func: AggFunc::Avg,
                expr: Some(Box::new(expr)),
            })
        }
        "min" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Aggregate {
                func: AggFunc::Min,
                expr: Some(Box::new(expr)),
            })
        }
        "max" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Aggregate {
                func: AggFunc::Max,
                expr: Some(Box::new(expr)),
            })
        }

        // Ordering
        "asc" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Asc(Box::new(expr)))
        }
        "desc" => {
            let expr = parse_query_expr(content)?;
            Ok(QueryExpr::Desc(Box::new(expr)))
        }

        _ => Err(syn::Error::new(
            ident.span(),
            format!("unknown function `{}`", name),
        )),
    }
}

fn parse_binary(content: ParseStream, op: BinOp) -> Result<QueryExpr> {
    let left = parse_query_expr(content)?;
    content.parse::<Token![,]>()?;
    let right = parse_query_expr(content)?;
    Ok(QueryExpr::BinaryOp {
        op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn parse_between(content: ParseStream, negated: bool) -> Result<QueryExpr> {
    let expr = parse_query_expr(content)?;
    content.parse::<Token![,]>()?;
    let low = parse_query_expr(content)?;
    content.parse::<Token![,]>()?;
    let high = parse_query_expr(content)?;
    Ok(QueryExpr::Between {
        expr: Box::new(expr),
        low: Box::new(low),
        high: Box::new(high),
        negated,
    })
}

/// Parse `and(a, b)` or `and([a, b, c])` (same for `or`).
fn parse_logical_list(content: ParseStream, is_and: bool) -> Result<QueryExpr> {
    let items = if content.peek(syn::token::Bracket) {
        // Bracketed form: and([a, b, c])
        let inner;
        bracketed!(inner in content);
        parse_comma_separated(&inner, parse_query_expr)?
    } else {
        // Two-arg form: and(a, b)
        let a = parse_query_expr(content)?;
        content.parse::<Token![,]>()?;
        let b = parse_query_expr(content)?;
        // Consume optional trailing comma
        let _ = content.parse::<Token![,]>();
        vec![a, b]
    };
    if is_and {
        Ok(QueryExpr::And(items))
    } else {
        Ok(QueryExpr::Or(items))
    }
}

// =============================================================================
// TABLE EXTRACTION HELPERS
// =============================================================================

/// Extract the table type (first segment) from a `Table::column` path.
fn extract_table_from_column(path: &Path) -> Option<syn::Type> {
    if path.segments.len() >= 2 {
        let table_ident = &path.segments[0].ident;
        syn::parse_str::<syn::Type>(&table_ident.to_string()).ok()
    } else {
        None
    }
}

/// Convert a bare table path (e.g., `VqUser`) to a type.
fn path_to_table_type(path: &Path) -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: path.clone(),
    })
}

/// Collect all unique table types referenced in the query.
fn collect_tables(query: &ViewQuery) -> Vec<syn::Type> {
    let mut tables = Vec::new();
    let mut seen = std::collections::HashSet::new();

    fn add_type(
        ty: syn::Type,
        tables: &mut Vec<syn::Type>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let key = quote!(#ty).to_string();
        if seen.insert(key) {
            tables.push(ty);
        }
    }

    fn add_column_path(
        path: &Path,
        tables: &mut Vec<syn::Type>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if let Some(ty) = extract_table_from_column(path) {
            add_type(ty, tables, seen);
        }
    }

    fn add_table_path(
        path: &Path,
        tables: &mut Vec<syn::Type>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        add_type(path_to_table_type(path), tables, seen);
    }

    fn add_expr_tables(
        expr: &QueryExpr,
        tables: &mut Vec<syn::Type>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        collect_expr_tables(expr, &mut |p| add_column_path(p, tables, seen));
    }

    for item in &query.select {
        match item {
            SelectItem::Column(p) => add_column_path(p, &mut tables, &mut seen),
            SelectItem::Expr(e) => add_expr_tables(e, &mut tables, &mut seen),
        }
    }
    add_table_path(&query.from, &mut tables, &mut seen);
    for j in &query.joins {
        add_table_path(&j.table, &mut tables, &mut seen);
        if let Some(c) = &j.condition {
            add_expr_tables(c, &mut tables, &mut seen);
        }
    }
    if let Some(f) = &query.filter {
        add_expr_tables(f, &mut tables, &mut seen);
    }
    for p in &query.group_by {
        add_column_path(p, &mut tables, &mut seen);
    }
    if let Some(h) = &query.having {
        add_expr_tables(h, &mut tables, &mut seen);
    }
    for o in &query.order_by {
        add_expr_tables(o, &mut tables, &mut seen);
    }

    tables
}

fn collect_expr_tables(expr: &QueryExpr, add: &mut dyn FnMut(&Path)) {
    match expr {
        QueryExpr::Column(p) => add(p),
        QueryExpr::Literal(_) => {}
        QueryExpr::BinaryOp { left, right, .. } => {
            collect_expr_tables(left, add);
            collect_expr_tables(right, add);
        }
        QueryExpr::IsNull { expr, .. } => collect_expr_tables(expr, add),
        QueryExpr::Between {
            expr, low, high, ..
        } => {
            collect_expr_tables(expr, add);
            collect_expr_tables(low, add);
            collect_expr_tables(high, add);
        }
        QueryExpr::InArray { expr, values } => {
            collect_expr_tables(expr, add);
            for v in values {
                collect_expr_tables(v, add);
            }
        }
        QueryExpr::And(items) | QueryExpr::Or(items) => {
            for i in items {
                collect_expr_tables(i, add);
            }
        }
        QueryExpr::Not(e) => collect_expr_tables(e, add),
        QueryExpr::Aggregate { expr, .. } => {
            if let Some(e) = expr {
                collect_expr_tables(e, add);
            }
        }
        QueryExpr::Asc(e) | QueryExpr::Desc(e) => collect_expr_tables(e, add),
    }
}

// =============================================================================
// CONST SQL GENERATION (Phase 1)
// =============================================================================

/// Generate a `concatcp!()` expression that produces the SELECT SQL at compile time.
///
/// `field_names` are the struct field names used for AS aliases.
pub fn generate_const_sql(
    query: &ViewQuery,
    field_names: &[String],
    dialect: Dialect,
) -> Result<TokenStream> {
    let sql_schema_path = quote!(drizzle::core::SQLSchema);
    let (value_path, schema_type_path) = match dialect {
        Dialect::SQLite => (
            quote!(drizzle::sqlite::values::SQLiteValue),
            quote!(drizzle::sqlite::common::SQLiteSchemaType),
        ),
        Dialect::Postgres => (
            quote!(drizzle::postgres::values::PostgresValue),
            quote!(drizzle::postgres::common::PostgresSchemaType),
        ),
    };

    // Helper: generate a concatcp-compatible expression for a table name given a table TYPE
    let table_name_from_type = |ty: &syn::Type| -> TokenStream {
        quote! {
            <#ty as #sql_schema_path<'_, #schema_type_path, #value_path<'_>>>::NAME
        }
    };

    // Helper: generate a concatcp-compatible expression for a column name
    let column_name_expr = |col_path: &Path| -> TokenStream {
        quote! {
            {
                const fn column_name<'a, C: #sql_schema_path<'a, &'static str, #value_path<'a>>>(_: &C) -> &'a str {
                    C::NAME
                }
                column_name(&#col_path)
            }
        }
    };

    // Generate schema-qualified table ref parts from a bare table path (e.g., VqUser)
    let table_ref_from_table_path = |table_path: &Path| -> Vec<TokenStream> {
        let ty = path_to_table_type(table_path);
        if dialect == Dialect::Postgres {
            vec![
                quote! { "\"" },
                quote! { <#ty>::DDL_TABLE.schema },
                quote! { "\".\"" },
                table_name_from_type(&ty),
                quote! { "\"" },
            ]
        } else {
            vec![quote! { "\"" }, table_name_from_type(&ty), quote! { "\"" }]
        }
    };

    // Generate table ref parts from a Table::column path (extracts table from first segment)
    let table_ref_from_column_path = |col_path: &Path| -> Vec<TokenStream> {
        let ty = extract_table_from_column(col_path).unwrap();
        if dialect == Dialect::Postgres {
            vec![
                quote! { "\"" },
                quote! { <#ty>::DDL_TABLE.schema },
                quote! { "\".\"" },
                table_name_from_type(&ty),
                quote! { "\"" },
            ]
        } else {
            vec![quote! { "\"" }, table_name_from_type(&ty), quote! { "\"" }]
        }
    };

    // Build the SELECT items
    let mut parts: Vec<TokenStream> = Vec::new();
    parts.push(quote! { "SELECT " });

    for (i, (item, field_name)) in query.select.iter().zip(field_names.iter()).enumerate() {
        if i > 0 {
            parts.push(quote! { ", " });
        }
        match item {
            SelectItem::Column(path) => {
                // "table"."col" AS "field"
                parts.extend(table_ref_from_column_path(path));
                let col_name = column_name_expr(path);
                parts.push(quote! { ".\"" });
                parts.push(col_name);
                parts.push(quote! { "\"" });
            }
            SelectItem::Expr(expr) => {
                let expr_parts = expr_to_sql_parts(
                    expr,
                    &table_ref_from_column_path,
                    &column_name_expr,
                    dialect,
                );
                parts.extend(expr_parts);
            }
        }
        // AS alias
        let alias = format!(" AS \"{}\"", field_name);
        parts.push(quote! { #alias });
    }

    // FROM (bare table path)
    parts.push(quote! { " FROM " });
    parts.extend(table_ref_from_table_path(&query.from));

    // JOINs (bare table paths)
    for join in &query.joins {
        let join_kw = match join.kind {
            JoinKind::Inner => " JOIN ",
            JoinKind::Left => " LEFT JOIN ",
            JoinKind::Right => " RIGHT JOIN ",
            JoinKind::Full => " FULL JOIN ",
            JoinKind::Cross => " CROSS JOIN ",
        };
        parts.push(quote! { #join_kw });
        parts.extend(table_ref_from_table_path(&join.table));
        if let Some(cond) = &join.condition {
            parts.push(quote! { " ON " });
            parts.extend(expr_to_sql_parts(
                cond,
                &table_ref_from_column_path,
                &column_name_expr,
                dialect,
            ));
        }
    }

    // WHERE
    if let Some(filter) = &query.filter {
        parts.push(quote! { " WHERE " });
        parts.extend(expr_to_sql_parts(
            filter,
            &table_ref_from_column_path,
            &column_name_expr,
            dialect,
        ));
    }

    // GROUP BY (column paths)
    if !query.group_by.is_empty() {
        parts.push(quote! { " GROUP BY " });
        for (i, path) in query.group_by.iter().enumerate() {
            if i > 0 {
                parts.push(quote! { ", " });
            }
            parts.extend(table_ref_from_column_path(path));
            let col_name = column_name_expr(path);
            parts.push(quote! { ".\"" });
            parts.push(col_name);
            parts.push(quote! { "\"" });
        }
    }

    // HAVING
    if let Some(having) = &query.having {
        parts.push(quote! { " HAVING " });
        parts.extend(expr_to_sql_parts(
            having,
            &table_ref_from_column_path,
            &column_name_expr,
            dialect,
        ));
    }

    // ORDER BY
    if !query.order_by.is_empty() {
        parts.push(quote! { " ORDER BY " });
        for (i, expr) in query.order_by.iter().enumerate() {
            if i > 0 {
                parts.push(quote! { ", " });
            }
            parts.extend(expr_to_sql_parts(
                expr,
                &table_ref_from_column_path,
                &column_name_expr,
                dialect,
            ));
        }
    }

    // LIMIT
    if let Some(limit) = query.limit {
        let limit_lit = proc_macro2::Literal::usize_suffixed(limit);
        parts.push(quote! { " LIMIT " });
        parts.push(quote! { #limit_lit });
    }

    // OFFSET
    if let Some(offset) = query.offset {
        let offset_lit = proc_macro2::Literal::usize_suffixed(offset);
        parts.push(quote! { " OFFSET " });
        parts.push(quote! { #offset_lit });
    }

    Ok(quote! {
        ::drizzle::const_format::concatcp!(#(#parts),*)
    })
}

/// Convert a QueryExpr into a list of concatcp-compatible token streams.
fn expr_to_sql_parts(
    expr: &QueryExpr,
    table_ref_parts: &dyn Fn(&Path) -> Vec<TokenStream>,
    column_name_expr: &dyn Fn(&Path) -> TokenStream,
    dialect: Dialect,
) -> Vec<TokenStream> {
    match expr {
        QueryExpr::Column(path) => {
            let mut parts = table_ref_parts(path);
            let col_name = column_name_expr(path);
            parts.push(quote! { ".\"" });
            parts.push(col_name);
            parts.push(quote! { "\"" });
            parts
        }
        QueryExpr::Literal(lit) => {
            vec![literal_to_sql(lit, dialect)]
        }
        QueryExpr::BinaryOp { op, left, right } => {
            let op_str = match op {
                BinOp::Eq => " = ",
                BinOp::Neq => " <> ",
                BinOp::Gt => " > ",
                BinOp::Gte => " >= ",
                BinOp::Lt => " < ",
                BinOp::Lte => " <= ",
                BinOp::Like => " LIKE ",
                BinOp::NotLike => " NOT LIKE ",
            };
            let mut parts = expr_to_sql_parts(left, table_ref_parts, column_name_expr, dialect);
            parts.push(quote! { #op_str });
            parts.extend(expr_to_sql_parts(
                right,
                table_ref_parts,
                column_name_expr,
                dialect,
            ));
            parts
        }
        QueryExpr::IsNull { expr, negated } => {
            let suffix = if *negated { " IS NOT NULL" } else { " IS NULL" };
            let mut parts = expr_to_sql_parts(expr, table_ref_parts, column_name_expr, dialect);
            parts.push(quote! { #suffix });
            parts
        }
        QueryExpr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let kw = if *negated {
                " NOT BETWEEN "
            } else {
                " BETWEEN "
            };
            let mut parts = expr_to_sql_parts(expr, table_ref_parts, column_name_expr, dialect);
            parts.push(quote! { #kw });
            parts.extend(expr_to_sql_parts(
                low,
                table_ref_parts,
                column_name_expr,
                dialect,
            ));
            parts.push(quote! { " AND " });
            parts.extend(expr_to_sql_parts(
                high,
                table_ref_parts,
                column_name_expr,
                dialect,
            ));
            parts
        }
        QueryExpr::InArray { expr, values } => {
            let mut parts = expr_to_sql_parts(expr, table_ref_parts, column_name_expr, dialect);
            parts.push(quote! { " IN (" });
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    parts.push(quote! { ", " });
                }
                parts.extend(expr_to_sql_parts(
                    v,
                    table_ref_parts,
                    column_name_expr,
                    dialect,
                ));
            }
            parts.push(quote! { ")" });
            parts
        }
        QueryExpr::And(items) => {
            let mut parts = vec![quote! { "(" }];
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    parts.push(quote! { " AND " });
                }
                parts.extend(expr_to_sql_parts(
                    item,
                    table_ref_parts,
                    column_name_expr,
                    dialect,
                ));
            }
            parts.push(quote! { ")" });
            parts
        }
        QueryExpr::Or(items) => {
            let mut parts = vec![quote! { "(" }];
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    parts.push(quote! { " OR " });
                }
                parts.extend(expr_to_sql_parts(
                    item,
                    table_ref_parts,
                    column_name_expr,
                    dialect,
                ));
            }
            parts.push(quote! { ")" });
            parts
        }
        QueryExpr::Not(inner) => {
            let mut parts = vec![quote! { "NOT " }];
            parts.extend(expr_to_sql_parts(
                inner,
                table_ref_parts,
                column_name_expr,
                dialect,
            ));
            parts
        }
        QueryExpr::Aggregate { func, expr } => {
            let (prefix, suffix) = match func {
                AggFunc::Count => ("COUNT(", ")"),
                AggFunc::CountAll => ("COUNT(*", ")"),
                AggFunc::CountDistinct => ("COUNT(DISTINCT ", ")"),
                AggFunc::Sum => ("SUM(", ")"),
                AggFunc::Avg => ("AVG(", ")"),
                AggFunc::Min => ("MIN(", ")"),
                AggFunc::Max => ("MAX(", ")"),
            };
            let mut parts = vec![quote! { #prefix }];
            if let Some(e) = expr {
                parts.extend(expr_to_sql_parts(
                    e,
                    table_ref_parts,
                    column_name_expr,
                    dialect,
                ));
            }
            parts.push(quote! { #suffix });
            parts
        }
        QueryExpr::Asc(inner) => {
            let mut parts = expr_to_sql_parts(inner, table_ref_parts, column_name_expr, dialect);
            parts.push(quote! { " ASC" });
            parts
        }
        QueryExpr::Desc(inner) => {
            let mut parts = expr_to_sql_parts(inner, table_ref_parts, column_name_expr, dialect);
            parts.push(quote! { " DESC" });
            parts
        }
    }
}

/// Convert a literal to a SQL string token.
fn literal_to_sql(lit: &Lit, dialect: Dialect) -> TokenStream {
    match lit {
        Lit::Int(i) => {
            let s = i.to_string();
            quote! { #s }
        }
        Lit::Float(f) => {
            let s = f.to_string();
            quote! { #s }
        }
        Lit::Str(s) => {
            let val = format!("'{}'", s.value().replace('\'', "''"));
            quote! { #val }
        }
        Lit::Bool(b) => {
            let s = match (dialect, b.value) {
                (Dialect::SQLite, true) => "1",
                (Dialect::SQLite, false) => "0",
                (Dialect::Postgres, true) => "TRUE",
                (Dialect::Postgres, false) => "FALSE",
            };
            quote! { #s }
        }
        _ => {
            let s = quote!(#lit).to_string();
            quote! { #s }
        }
    }
}

// =============================================================================
// VALIDATION GENERATION (Phase 2)
// =============================================================================

/// Generate a `const _: () = { ... }` block that type-checks expressions
/// by calling the real `eq`, `gt`, etc. functions from the expression system.
///
/// This ensures the column types are compatible without any runtime cost.
pub fn generate_validation(
    query: &ViewQuery,
    field_count: usize,
    _dialect: Dialect,
) -> Result<TokenStream> {
    let select_count = query.select.len();
    if select_count != field_count {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "query has {} SELECT items but struct has {} fields",
                select_count, field_count
            ),
        ));
    }

    let tables = collect_tables(query);
    // Expression functions (eq, gt, etc.) are in drizzle::core::expr for all dialects
    let expr_mod = quote!(drizzle::core::expr);

    // Build table instantiation: `let table = Table::new();`
    let table_lets: Vec<TokenStream> = tables
        .iter()
        .map(|ty| {
            let var_name = syn::Ident::new(
                &format!("_tbl_{}", quote!(#ty).to_string().to_lowercase()),
                proc_macro2::Span::call_site(),
            );
            quote! { let #var_name = #ty::new(); }
        })
        .collect();

    // Build validation expressions
    let mut validation_stmts = Vec::new();

    // Validate select items
    for item in &query.select {
        match item {
            SelectItem::Column(path) => {
                validation_stmts.push(quote! { let _ = &#path; });
            }
            SelectItem::Expr(expr) => {
                if let Some(stmt) = generate_expr_validation(expr, &expr_mod) {
                    validation_stmts.push(stmt);
                }
            }
        }
    }

    // Validate from table exists
    let from_table = &query.from;
    validation_stmts.push(quote! { let _ = #from_table::new(); });

    // Validate join conditions
    for join in &query.joins {
        let table = &join.table;
        validation_stmts.push(quote! { let _ = #table::new(); });
        if let Some(cond) = &join.condition
            && let Some(stmt) = generate_expr_validation(cond, &expr_mod)
        {
            validation_stmts.push(stmt);
        }
    }

    // Validate filter
    if let Some(filter) = &query.filter
        && let Some(stmt) = generate_expr_validation(filter, &expr_mod)
    {
        validation_stmts.push(stmt);
    }

    // Validate having
    if let Some(having) = &query.having
        && let Some(stmt) = generate_expr_validation(having, &expr_mod)
    {
        validation_stmts.push(stmt);
    }

    // Validate group_by columns
    for path in &query.group_by {
        validation_stmts.push(quote! { let _ = &#path; });
    }

    // Validate order_by expressions
    for expr in &query.order_by {
        if let Some(stmt) = generate_expr_validation(expr, &expr_mod) {
            validation_stmts.push(stmt);
        }
    }

    Ok(quote! {
        const _: () = {
            #[allow(unused, non_snake_case)]
            fn _validate_view_query() {
                #(#table_lets)*
                #(#validation_stmts)*
            }
        };
    })
}

/// Generate a validation statement for a single expression.
fn generate_expr_validation(expr: &QueryExpr, expr_mod: &TokenStream) -> Option<TokenStream> {
    match expr {
        QueryExpr::Column(path) => Some(quote! { let _ = &#path; }),
        QueryExpr::Literal(_) => None,
        QueryExpr::BinaryOp { op, left, right } => {
            let func = match op {
                BinOp::Eq => quote!(#expr_mod::eq),
                BinOp::Neq => quote!(#expr_mod::neq),
                BinOp::Gt => quote!(#expr_mod::gt),
                BinOp::Gte => quote!(#expr_mod::gte),
                BinOp::Lt => quote!(#expr_mod::lt),
                BinOp::Lte => quote!(#expr_mod::lte),
                BinOp::Like => quote!(#expr_mod::like),
                BinOp::NotLike => quote!(#expr_mod::not_like),
            };
            let left_ts = expr_to_validation_expr(left, expr_mod);
            let right_ts = expr_to_validation_expr(right, expr_mod);
            Some(quote! { let _ = #func(#left_ts, #right_ts); })
        }
        QueryExpr::IsNull {
            expr: inner,
            negated,
        } => {
            let inner_ts = expr_to_validation_expr(inner, expr_mod);
            if *negated {
                Some(quote! { let _ = #expr_mod::is_not_null(#inner_ts); })
            } else {
                Some(quote! { let _ = #expr_mod::is_null(#inner_ts); })
            }
        }
        QueryExpr::Between {
            expr: inner,
            low,
            high,
            negated,
        } => {
            let inner_ts = expr_to_validation_expr(inner, expr_mod);
            let low_ts = expr_to_validation_expr(low, expr_mod);
            let high_ts = expr_to_validation_expr(high, expr_mod);
            if *negated {
                Some(quote! { let _ = #expr_mod::not_between(#inner_ts, #low_ts, #high_ts); })
            } else {
                Some(quote! { let _ = #expr_mod::between(#inner_ts, #low_ts, #high_ts); })
            }
        }
        QueryExpr::InArray {
            expr: inner,
            values,
        } => {
            let inner_ts = expr_to_validation_expr(inner, expr_mod);
            let value_ts: Vec<_> = values
                .iter()
                .map(|v| expr_to_validation_expr(v, expr_mod))
                .collect();
            Some(quote! { let _ = #expr_mod::in_array(#inner_ts, [#(#value_ts),*]); })
        }
        QueryExpr::And(items) => {
            let stmts: Vec<_> = items
                .iter()
                .filter_map(|i| generate_expr_validation(i, expr_mod))
                .collect();
            if stmts.is_empty() {
                None
            } else {
                Some(quote! { #(#stmts)* })
            }
        }
        QueryExpr::Or(items) => {
            let stmts: Vec<_> = items
                .iter()
                .filter_map(|i| generate_expr_validation(i, expr_mod))
                .collect();
            if stmts.is_empty() {
                None
            } else {
                Some(quote! { #(#stmts)* })
            }
        }
        QueryExpr::Not(inner) => generate_expr_validation(inner, expr_mod),
        QueryExpr::Aggregate { expr: inner, .. } => inner
            .as_ref()
            .and_then(|e| generate_expr_validation(e, expr_mod)),
        QueryExpr::Asc(inner) | QueryExpr::Desc(inner) => generate_expr_validation(inner, expr_mod),
    }
}

/// Convert a QueryExpr into a token stream usable as a function argument in validation.
fn expr_to_validation_expr(expr: &QueryExpr, expr_mod: &TokenStream) -> TokenStream {
    match expr {
        QueryExpr::Column(path) => quote! { #path },
        QueryExpr::Literal(lit) => quote! { #lit },
        QueryExpr::BinaryOp { op, left, right } => {
            let func = match op {
                BinOp::Eq => quote!(#expr_mod::eq),
                BinOp::Neq => quote!(#expr_mod::neq),
                BinOp::Gt => quote!(#expr_mod::gt),
                BinOp::Gte => quote!(#expr_mod::gte),
                BinOp::Lt => quote!(#expr_mod::lt),
                BinOp::Lte => quote!(#expr_mod::lte),
                BinOp::Like => quote!(#expr_mod::like),
                BinOp::NotLike => quote!(#expr_mod::not_like),
            };
            let left_ts = expr_to_validation_expr(left, expr_mod);
            let right_ts = expr_to_validation_expr(right, expr_mod);
            quote! { #func(#left_ts, #right_ts) }
        }
        QueryExpr::IsNull {
            expr: inner,
            negated,
        } => {
            let inner_ts = expr_to_validation_expr(inner, expr_mod);
            if *negated {
                quote! { #expr_mod::is_not_null(#inner_ts) }
            } else {
                quote! { #expr_mod::is_null(#inner_ts) }
            }
        }
        QueryExpr::Aggregate { func, expr: inner } => match func {
            AggFunc::CountAll => quote! { #expr_mod::count_all() },
            AggFunc::Count => {
                let inner_ts = inner
                    .as_ref()
                    .map(|e| expr_to_validation_expr(e, expr_mod))
                    .unwrap();
                quote! { #expr_mod::count(#inner_ts) }
            }
            AggFunc::CountDistinct => {
                let inner_ts = inner
                    .as_ref()
                    .map(|e| expr_to_validation_expr(e, expr_mod))
                    .unwrap();
                quote! { #expr_mod::count_distinct(#inner_ts) }
            }
            AggFunc::Sum => {
                let inner_ts = inner
                    .as_ref()
                    .map(|e| expr_to_validation_expr(e, expr_mod))
                    .unwrap();
                quote! { #expr_mod::sum(#inner_ts) }
            }
            AggFunc::Avg => {
                let inner_ts = inner
                    .as_ref()
                    .map(|e| expr_to_validation_expr(e, expr_mod))
                    .unwrap();
                quote! { #expr_mod::avg(#inner_ts) }
            }
            AggFunc::Min => {
                let inner_ts = inner
                    .as_ref()
                    .map(|e| expr_to_validation_expr(e, expr_mod))
                    .unwrap();
                quote! { #expr_mod::min(#inner_ts) }
            }
            AggFunc::Max => {
                let inner_ts = inner
                    .as_ref()
                    .map(|e| expr_to_validation_expr(e, expr_mod))
                    .unwrap();
                quote! { #expr_mod::max(#inner_ts) }
            }
        },
        _ => quote! { () },
    }
}
