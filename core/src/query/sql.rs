//! SQL generation for the relational Query API.
//!
//! Renders typed relation structures into SQL with JSON subqueries.
//! Uses `V::DIALECT` to dispatch between `SQLite` and `PostgreSQL` syntax.

use core::fmt::Write;

use crate::SQLParam;
use crate::dialect::Dialect;
use crate::prelude::*;
use crate::relation::{CardWrap, JunctionMeta, RelationDef};

use super::builder::{AllColumns, PartialColumns, QueryTable};
use super::handle::RelationHandle;

/// Cardinality for runtime SQL generation decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelCardinality {
    /// `Vec<T>` — uses `json_group_array` / `json_agg`
    Many,
    /// `T` — uses `json_object` / `json_build_object` with `LIMIT 1`
    One,
    /// `Option<T>` — uses `json_object` / `json_build_object` with `LIMIT 1`
    OptionalOne,
}

/// Pre-rendered relation configuration for SQL generation.
///
/// Produced by `RenderRelations::render_into()` at query execution time.
pub struct RenderedRelation<V: SQLParam> {
    /// Target table name (e.g., "post").
    pub table_name: &'static str,
    /// Target table columns for SELECT (e.g., `["id", "content", "author_id"]`).
    pub column_names: Vec<&'static str>,
    /// Column names that store BLOB data and need `hex()` wrapping in JSON.
    pub blob_columns: &'static [&'static str],
    /// FK column pairs for the join condition.
    /// Each pair `(a, b)` generates `target_alias."a" = parent_alias."b"`.
    pub fk_columns: &'static [(&'static str, &'static str)],
    /// Cardinality (Many, One, `OptionalOne`).
    pub cardinality: RelCardinality,
    /// Relation name for the JSON alias (e.g., "posts", "author").
    pub rel_name: &'static str,
    /// Pre-rendered WHERE SQL fragment with placeholders (may be empty).
    pub where_sql: String,
    /// Bind param values for the WHERE clause, in placeholder order.
    pub where_params: Vec<V>,
    /// Pre-rendered ORDER BY SQL fragment (may be empty).
    pub order_by_sql: String,
    /// LIMIT value.
    pub limit: Option<u32>,
    /// OFFSET value.
    pub offset: Option<u32>,
    /// Nested rendered relations.
    pub nested: Vec<Self>,
    /// Junction table metadata for many-to-many relations.
    pub junction: Option<JunctionMeta>,
}

/// Converts a typed relation structure into `Vec<RenderedRelation<V>>`.
pub trait RenderRelations<V: SQLParam> {
    /// Appends rendered relations to `out`, consuming self.
    fn render_into(self, out: &mut Vec<RenderedRelation<V>>);
}

impl<V: SQLParam> RenderRelations<V> for () {
    #[inline]
    fn render_into(self, _out: &mut Vec<RenderedRelation<V>>) {}
}

// AllColumns: use all columns from QueryTable
impl<V, R, Nested, Rest, Cl> RenderRelations<V>
    for (RelationHandle<V, R, Nested, AllColumns, Cl>, Rest)
where
    V: SQLParam,
    R: RelationDef,
    Nested: RenderRelations<V>,
    Rest: RenderRelations<V>,
{
    fn render_into(self, out: &mut Vec<RenderedRelation<V>>) {
        let (handle, rest) = self;
        let mut nested = Vec::new();
        handle.nested.render_into(&mut nested);
        out.push(RenderedRelation {
            table_name: <R::Target as QueryTable>::TABLE_NAME,
            column_names: <R::Target as QueryTable>::COLUMN_NAMES.to_vec(),
            blob_columns: <R::Target as QueryTable>::BLOB_COLUMNS,
            fk_columns: R::fk_columns(),
            cardinality: <R::Card as CardWrap>::CARDINALITY,
            rel_name: R::NAME,
            where_sql: handle.where_clause,
            where_params: handle.where_params,
            order_by_sql: handle.order_by_clause,
            limit: handle.limit,
            offset: handle.offset,
            nested,
            junction: R::junction(),
        });
        rest.render_into(out);
    }
}

// PartialColumns: use filtered columns from the handle
impl<V, R, Nested, Rest, Cl> RenderRelations<V>
    for (RelationHandle<V, R, Nested, PartialColumns, Cl>, Rest)
where
    V: SQLParam,
    R: RelationDef,
    Nested: RenderRelations<V>,
    Rest: RenderRelations<V>,
{
    fn render_into(self, out: &mut Vec<RenderedRelation<V>>) {
        let (handle, rest) = self;
        let mut nested = Vec::new();
        handle.nested.render_into(&mut nested);
        out.push(RenderedRelation {
            table_name: <R::Target as QueryTable>::TABLE_NAME,
            column_names: handle.cols.columns,
            blob_columns: <R::Target as QueryTable>::BLOB_COLUMNS,
            fk_columns: R::fk_columns(),
            cardinality: <R::Card as CardWrap>::CARDINALITY,
            rel_name: R::NAME,
            where_sql: handle.where_clause,
            where_params: handle.where_params,
            order_by_sql: handle.order_by_clause,
            limit: handle.limit,
            offset: handle.offset,
            nested,
            junction: R::junction(),
        });
        rest.render_into(out);
    }
}

// =============================================================================
// SQL Generation
// =============================================================================

/// Generates the full SQL for a query with relations, returning the SQL string
/// and collected bind params from WHERE clauses.
///
/// When `wrap_base_json` is true, base columns are wrapped in a JSON object
/// (`json_object(...)` / `json_build_object(...)`) as a single `"__base"` column.
/// This is used for partial column selection.
///
/// Uses `V::DIALECT` to select the correct JSON functions and placeholder style.
#[allow(clippy::too_many_arguments)]
pub fn build_query_sql<'p, V: SQLParam>(
    table_name: &str,
    column_names: &[&str],
    blob_columns: &[&str],
    relations: &'p [RenderedRelation<V>],
    where_clause: &str,
    where_params: &'p [V],
    order_by: &str,
    limit: Option<u32>,
    offset: Option<u32>,
    wrap_base_json: bool,
) -> (String, Vec<&'p V>) {
    let mut sql = String::with_capacity(256);
    let mut params: Vec<&'p V> = Vec::new();
    let alias = "t0";
    let dialect = V::DIALECT;

    // SELECT base columns
    sql.push_str("SELECT ");

    if wrap_base_json {
        // Wrap base columns in json_object/json_build_object as "__base"
        write_json_object_open(dialect, &mut sql);
        for (i, c) in column_names.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('\'');
            sql.push_str(c);
            sql.push_str("', ");
            write_json_column(alias, c, blob_columns, dialect, &mut sql);
        }
        sql.push(')');
        if dialect == Dialect::PostgreSQL {
            sql.push_str("::text");
        }
        sql.push_str(" AS \"__base\"");
    } else {
        for (i, c) in column_names.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            write_qualified_column(alias, c, &mut sql);
        }
    }

    // Collect relation subquery params separately so root WHERE params
    // come first in the final params vec (matching $1, $2, ... numbering).
    let mut rel_params: Vec<&'p V> = Vec::new();

    // Add relation subqueries as additional SELECT columns
    let mut alias_counter = 1usize;
    let mut param_counter = where_params.len() + 1;
    for rel in relations {
        sql.push_str(", ");
        write_relation_subquery::<V>(
            rel,
            alias,
            &mut alias_counter,
            &mut param_counter,
            &mut sql,
            &mut rel_params,
        );
        // PostgreSQL returns json type — cast to text so the driver reads it as String
        if dialect == Dialect::PostgreSQL {
            sql.push_str("::text");
        }
        sql.push_str(" AS \"__rel_");
        sql.push_str(rel.rel_name);
        sql.push('"');
    }

    // FROM
    sql.push_str(" FROM \"");
    sql.push_str(table_name);
    sql.push_str("\" AS \"");
    sql.push_str(alias);
    sql.push('"');

    // Rewrite table references to use the alias
    if !where_clause.is_empty() || !order_by.is_empty() {
        let table_prefix = format!("\"{table_name}\".");
        let alias_prefix = format!("\"{alias}\".");

        if !where_clause.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause.replace(&table_prefix, &alias_prefix));
        }

        // ORDER BY
        if !order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_by.replace(&table_prefix, &alias_prefix));
        }
    }

    // Assemble params in the correct order for the dialect.
    //
    // PostgreSQL uses numbered `$N` placeholders — root WHERE params are
    // `$1..$N` and relation params are renumbered to `$N+1..`, so they
    // must appear in that order regardless of SQL text position.
    //
    // SQLite/MySQL use positional `?` — params must match the textual
    // order in the SQL string. Relation subqueries appear in the SELECT
    // clause (before WHERE), so their params must come first.
    if dialect == Dialect::PostgreSQL {
        params.extend(where_params.iter());
        params.extend(rel_params);
    } else {
        params.extend(rel_params);
        params.extend(where_params.iter());
    }

    // LIMIT
    if let Some(n) = limit {
        sql.push_str(" LIMIT ");
        let _ = write!(sql, "{n}");
    }

    // OFFSET
    if let Some(n) = offset {
        sql.push_str(" OFFSET ");
        let _ = write!(sql, "{n}");
    }

    (sql, params)
}

/// Writes the inner-subquery prefix (`[LATERAL ](SELECT cols FROM "`) used when
/// a Many relation needs a nested derived table (LIMIT/OFFSET/ORDER BY). The
/// table/alias/junction/WHERE suffix is emitted by the caller and shared with
/// the non-subquery path.
fn write_inner_subquery_prelude<V: SQLParam>(
    rel: &RenderedRelation<V>,
    target_table: &str,
    alias: &str,
    target_columns: &[&'static str],
    dialect: Dialect,
    sql: &mut String,
) {
    let _ = target_table;
    // When nested relations exist, their correlated subqueries reference FK
    // columns via `alias`. Ensure those columns appear in the inner derived
    // table even if the user excluded them via `.columns()` / `.omit()`.
    let mut extra_cols: Vec<&str> = Vec::new();
    for nested_rel in &rel.nested {
        if let Some(junction) = &nested_rel.junction {
            for (_, src_col) in junction.source_fk {
                if !target_columns.contains(src_col) && !extra_cols.contains(src_col) {
                    extra_cols.push(src_col);
                }
            }
        } else {
            for (_, tgt_col) in nested_rel.fk_columns {
                if !target_columns.contains(tgt_col) && !extra_cols.contains(tgt_col) {
                    extra_cols.push(tgt_col);
                }
            }
        }
    }

    // PostgreSQL requires LATERAL for derived tables that reference columns
    // from the outer query (the parent alias).
    if dialect == Dialect::PostgreSQL {
        sql.push_str("LATERAL ");
    }
    sql.push_str("(SELECT ");
    for (i, c) in target_columns.iter().enumerate() {
        if i > 0 {
            sql.push_str(", ");
        }
        write_qualified_column(alias, c, sql);
    }
    for c in &extra_cols {
        sql.push_str(", ");
        write_qualified_column(alias, c, sql);
    }
    sql.push_str(" FROM \"");
}

/// Writes the `json_object(...)` / `json_build_object(...)` body: first the
/// base columns with literal keys, then nested relations recursively rendered
/// as named subqueries. Emits the trailing `)` that closes the object.
fn write_json_object_body<'p, V: SQLParam>(
    rel: &'p RenderedRelation<V>,
    alias: &str,
    target_columns: &[&'static str],
    dialect: Dialect,
    ctx: &mut SubqueryCtx<'_, 'p, V>,
) {
    write_json_object_open(dialect, ctx.sql);
    let mut first_arg = true;
    for c in target_columns {
        if !first_arg {
            ctx.sql.push_str(", ");
        }
        first_arg = false;
        ctx.sql.push('\'');
        ctx.sql.push_str(c);
        ctx.sql.push_str("', ");
        write_json_column(alias, c, rel.blob_columns, dialect, ctx.sql);
    }

    // Nested relation subqueries as additional json_object args
    for nested_rel in &rel.nested {
        if !first_arg {
            ctx.sql.push_str(", ");
        }
        first_arg = false;
        ctx.sql.push('\'');
        ctx.sql.push_str(nested_rel.rel_name);
        ctx.sql.push_str("', ");
        write_relation_subquery::<V>(
            nested_rel,
            alias,
            ctx.alias_counter,
            ctx.param_counter,
            ctx.sql,
            ctx.params,
        );
    }

    ctx.sql.push(')'); // close json_object / json_build_object
}

/// Allocates a fresh `"tN"`-style alias and increments the counter in place.
fn alloc_alias(counter: &mut usize) -> String {
    let num = *counter;
    *counter += 1;
    let mut buf = String::with_capacity(4);
    buf.push('t');
    let _ = write!(buf, "{num}");
    buf
}

/// Mutable scratch state threaded through subquery emitters.
struct SubqueryCtx<'s, 'p, V: SQLParam> {
    alias_counter: &'s mut usize,
    param_counter: &'s mut usize,
    sql: &'s mut String,
    params: &'s mut Vec<&'p V>,
}

/// Emits the additional WHERE predicates, trailing ORDER BY (when not already
/// inlined in `json_agg`), and LIMIT/OFFSET clauses for a relation subquery.
fn write_where_order_limit_offset<'p, V: SQLParam>(
    rel: &'p RenderedRelation<V>,
    target_table: &str,
    alias: &str,
    dialect: Dialect,
    pg_order_in_agg: bool,
    cardinality: RelCardinality,
    ctx: &mut SubqueryCtx<'_, 'p, V>,
) {
    let has_trailing_order = !rel.order_by_sql.is_empty() && !pg_order_in_agg;
    if !rel.where_sql.is_empty() || has_trailing_order {
        let table_prefix = format!("\"{target_table}\".");
        let alias_prefix = format!("\"{alias}\".");

        if !rel.where_sql.is_empty() {
            ctx.sql.push_str(" AND ");
            let rewritten = rel.where_sql.replace(&table_prefix, &alias_prefix);
            // For numbered placeholders ($1, $2, ...), offset to account for
            // params already collected before this clause.
            let rewritten = renumber_placeholders(dialect, &rewritten, *ctx.param_counter);
            ctx.sql.push_str(&rewritten);
            *ctx.param_counter += rel.where_params.len();
            for p in &rel.where_params {
                ctx.params.push(p);
            }
        }

        if has_trailing_order {
            ctx.sql.push_str(" ORDER BY ");
            ctx.sql
                .push_str(&rel.order_by_sql.replace(&table_prefix, &alias_prefix));
        }
    }

    // LIMIT
    match cardinality {
        RelCardinality::One | RelCardinality::OptionalOne => {
            ctx.sql.push_str(" LIMIT 1");
        }
        RelCardinality::Many => {
            if let Some(n) = rel.limit {
                ctx.sql.push_str(" LIMIT ");
                let _ = write!(ctx.sql, "{n}");
            }
        }
    }

    if let Some(n) = rel.offset {
        ctx.sql.push_str(" OFFSET ");
        let _ = write!(ctx.sql, "{n}");
    }
}

/// Writes the FK equality predicates that join a relation's rows against the
/// parent row. If a junction table is present, the predicates are emitted
/// between the junction alias and the parent alias; otherwise they join the
/// relation's own alias to the parent.
fn write_fk_join_conditions<V: SQLParam>(
    rel: &RenderedRelation<V>,
    alias: &str,
    parent_alias: &str,
    junction_alias: Option<&str>,
    fk_columns: &[(&str, &str)],
    sql: &mut String,
) {
    let push_pair = |a: &str, b: &str, ca: &str, cb: &str, sql: &mut String| {
        sql.push('"');
        sql.push_str(a);
        sql.push_str("\".\"");
        sql.push_str(ca);
        sql.push_str("\" = \"");
        sql.push_str(b);
        sql.push_str("\".\"");
        sql.push_str(cb);
        sql.push('"');
    };
    if let (Some(junction), Some(junc_alias)) = (&rel.junction, junction_alias) {
        for (i, (junc_col, src_col)) in junction.source_fk.iter().enumerate() {
            if i > 0 {
                sql.push_str(" AND ");
            }
            push_pair(junc_alias, parent_alias, junc_col, src_col, sql);
        }
    } else {
        for (i, (src_col, tgt_col)) in fk_columns.iter().enumerate() {
            if i > 0 {
                sql.push_str(" AND ");
            }
            push_pair(alias, parent_alias, src_col, tgt_col, sql);
        }
    }
}

/// Writes a correlated subquery for a single relation directly into `sql`.
fn write_relation_subquery<'p, V: SQLParam>(
    rel: &'p RenderedRelation<V>,
    parent_alias: &str,
    alias_counter: &mut usize,
    param_counter: &mut usize,
    sql: &mut String,
    params: &mut Vec<&'p V>,
) {
    let alias_buf = alloc_alias(alias_counter);
    let alias = &alias_buf;

    // Allocate junction alias if this is a many-to-many relation.
    let junction_alias = rel.junction.as_ref().map(|_| alloc_alias(alias_counter));

    let target_table = rel.table_name;
    let target_columns = &rel.column_names;
    let fk_columns = rel.fk_columns;
    let cardinality = rel.cardinality;
    let dialect = V::DIALECT;

    // PostgreSQL optimization: ORDER BY inside json_agg() avoids an inner subquery.
    // `json_agg(expr ORDER BY ...)` is more efficient than wrapping in a derived table.
    // SQLite's json_group_array doesn't reliably support this, so keep the subquery there.
    let pg_order_in_agg = cardinality == RelCardinality::Many
        && dialect == Dialect::PostgreSQL
        && !rel.order_by_sql.is_empty()
        && rel.limit.is_none()
        && rel.offset.is_none();

    // Many relations with LIMIT / OFFSET need a nested subquery so constraints
    // apply before aggregation. ORDER BY alone also needs one on SQLite (no
    // aggregate ORDER BY), but on PostgreSQL it goes inside json_agg instead.
    let needs_inner_subquery = cardinality == RelCardinality::Many
        && (rel.limit.is_some()
            || rel.offset.is_some()
            || (!pg_order_in_agg && !rel.order_by_sql.is_empty()));

    // (SELECT
    sql.push_str("(SELECT ");

    // json_group_array( / COALESCE(json_agg( wrapper for Many
    if cardinality == RelCardinality::Many {
        write_json_array_agg_open(dialect, sql);
    }

    write_json_object_body::<V>(
        rel,
        alias,
        target_columns,
        dialect,
        &mut SubqueryCtx {
            alias_counter,
            param_counter,
            sql,
            params,
        },
    );

    // PostgreSQL: ORDER BY inside json_agg — e.g. json_agg(expr ORDER BY "t1"."col" DESC)
    if pg_order_in_agg {
        let table_prefix = format!("\"{target_table}\".");
        let alias_prefix = format!("\"{alias}\".");
        sql.push_str(" ORDER BY ");
        sql.push_str(&rel.order_by_sql.replace(&table_prefix, &alias_prefix));
    }

    // close json_group_array / json_agg for Many
    if cardinality == RelCardinality::Many {
        write_json_array_agg_close(dialect, sql);
    }

    // FROM
    sql.push_str(" FROM ");

    if needs_inner_subquery {
        write_inner_subquery_prelude(rel, target_table, alias, target_columns, dialect, sql);
    } else {
        sql.push('"');
    }
    sql.push_str(target_table);
    sql.push_str("\" AS \"");
    sql.push_str(alias);
    sql.push('"');
    if let (Some(junction), Some(junc_alias)) = (&rel.junction, &junction_alias) {
        write_junction_join(junction, alias, junc_alias, sql);
    }
    sql.push_str(" WHERE ");

    // FK join conditions — junction replaces direct FK with INNER JOIN + WHERE
    write_fk_join_conditions(
        rel,
        alias,
        parent_alias,
        junction_alias.as_deref(),
        fk_columns,
        sql,
    );

    // Additional WHERE and ORDER BY, then LIMIT/OFFSET per cardinality.
    write_where_order_limit_offset(
        rel,
        target_table,
        alias,
        dialect,
        pg_order_in_agg,
        cardinality,
        &mut SubqueryCtx {
            alias_counter,
            param_counter,
            sql,
            params,
        },
    );

    if needs_inner_subquery {
        sql.push_str(") AS \"");
        sql.push_str(alias);
        sql.push('"');
    }

    sql.push(')'); // close outer (SELECT ...)
}

// =============================================================================
// Dialect-specific helpers
// =============================================================================

/// Writes `"alias"."column"` into the buffer.
fn write_qualified_column(alias: &str, column: &str, sql: &mut String) {
    sql.push('"');
    sql.push_str(alias);
    sql.push_str("\".\"");
    sql.push_str(column);
    sql.push('"');
}

/// Writes an `INNER JOIN` clause for a junction (many-to-many) table.
///
/// Generates: `INNER JOIN "junction" AS "junc_alias" ON "junc_alias"."col" = "target_alias"."col"`
fn write_junction_join(
    junction: &JunctionMeta,
    target_alias: &str,
    junc_alias: &str,
    sql: &mut String,
) {
    sql.push_str(" INNER JOIN \"");
    sql.push_str(junction.table_name);
    sql.push_str("\" AS \"");
    sql.push_str(junc_alias);
    sql.push_str("\" ON ");
    for (i, (junc_col, target_col)) in junction.target_fk.iter().enumerate() {
        if i > 0 {
            sql.push_str(" AND ");
        }
        sql.push('"');
        sql.push_str(junc_alias);
        sql.push_str("\".\"");
        sql.push_str(junc_col);
        sql.push_str("\" = \"");
        sql.push_str(target_alias);
        sql.push_str("\".\"");
        sql.push_str(target_col);
        sql.push('"');
    }
}

/// Writes a column reference for use inside `json_object()`.
///
/// For BLOB columns on `SQLite`, wraps with a NULL-safe `hex()` expression:
/// `CASE WHEN col IS NULL THEN NULL ELSE hex(col) END`.
///
/// Plain `hex(NULL)` returns an empty string `""` rather than SQL NULL,
/// which would cause `json_object()` to emit `"col":""` instead of
/// `"col":null`. The CASE expression preserves NULLs correctly.
///
/// `PostgreSQL` handles all types natively in `json_build_object()`, so no
/// wrapping is needed regardless of column type.
fn write_json_column(
    alias: &str,
    column: &str,
    blob_columns: &[&str],
    dialect: Dialect,
    sql: &mut String,
) {
    let is_blob = dialect == Dialect::SQLite && blob_columns.contains(&column);
    if is_blob {
        sql.push_str("CASE WHEN ");
        write_qualified_column(alias, column, sql);
        sql.push_str(" IS NULL THEN NULL ELSE hex(");
        write_qualified_column(alias, column, sql);
        sql.push_str(") END");
    } else {
        write_qualified_column(alias, column, sql);
    }
}

/// Opens a JSON object constructor.
/// `SQLite`: `json_object(`, `PostgreSQL`: `json_build_object(`
fn write_json_object_open(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.push_str("json_object("),
        Dialect::PostgreSQL => sql.push_str("json_build_object("),
    }
}

/// Opens a JSON array aggregation wrapper for Many relations.
/// `SQLite`: `json_group_array(`, `PostgreSQL`: `COALESCE(json_agg(`
fn write_json_array_agg_open(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.push_str("json_group_array("),
        Dialect::PostgreSQL => sql.push_str("COALESCE(json_agg("),
    }
}

/// Closes a JSON array aggregation wrapper for Many relations.
/// `SQLite`: `)`, `PostgreSQL`: `), '[]'::json)`
fn write_json_array_agg_close(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.push(')'),
        Dialect::PostgreSQL => sql.push_str("), '[]'::json)"),
    }
}

/// Renumbers `$N` placeholders in a pre-built SQL fragment to start at `offset`.
///
/// For `SQLite` (`?` placeholders), returns the input unchanged.
/// For `PostgreSQL`, rewrites `$1` → `$offset`, `$2` → `$offset+1`, etc.
fn renumber_placeholders(dialect: Dialect, sql: &str, offset: usize) -> String {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.to_string(),
        Dialect::PostgreSQL => renumber_dollar_placeholders(sql, offset),
    }
}

/// Rewrites `$1`, `$2`, ... in a SQL string so they start at `offset`.
/// `$1` becomes `$offset`, `$2` becomes `$offset+1`, etc.
fn renumber_dollar_placeholders(sql: &str, offset: usize) -> String {
    let mut result = String::with_capacity(sql.len() + 8);
    let mut chars = sql.char_indices().peekable();

    while let Some((_i, ch)) = chars.next() {
        if ch == '$'
            && let Some(&(start, next_ch)) = chars.peek()
            && next_ch.is_ascii_digit()
        {
            // Consume all digits
            let mut end = start;
            while let Some(&(j, d)) = chars.peek() {
                if d.is_ascii_digit() {
                    end = j + d.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let orig: usize = sql[start..end].parse().unwrap_or(0);
            let new_num = orig + offset - 1; // $1 -> $offset, $2 -> $offset+1
            result.push('$');
            let _ = write!(result, "{new_num}");
            continue;
        }
        result.push(ch);
    }

    result
}
