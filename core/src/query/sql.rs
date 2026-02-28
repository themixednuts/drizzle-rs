//! SQL generation for the relational Query API.
//!
//! Renders typed relation structures into SQL with JSON subqueries.
//! Uses `V::DIALECT` to dispatch between SQLite and PostgreSQL syntax.

use core::fmt::Write;

use crate::SQLParam;
use crate::dialect::Dialect;
use crate::prelude::*;
use crate::relation::{CardWrap, RelationDef};

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
    /// Target table columns for SELECT (e.g., ["id", "content", "author_id"]).
    pub column_names: Vec<&'static str>,
    /// FK column pairs for the join condition.
    /// Each pair `(a, b)` generates `target_alias."a" = parent_alias."b"`.
    pub fk_columns: &'static [(&'static str, &'static str)],
    /// Cardinality (Many, One, OptionalOne).
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
    pub nested: Vec<RenderedRelation<V>>,
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
            fk_columns: R::fk_columns(),
            cardinality: <R::Card as CardWrap>::CARDINALITY,
            rel_name: R::NAME,
            where_sql: handle.where_clause,
            where_params: handle.where_params,
            order_by_sql: handle.order_by_clause,
            limit: handle.limit,
            offset: handle.offset,
            nested,
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
            fk_columns: R::fk_columns(),
            cardinality: <R::Card as CardWrap>::CARDINALITY,
            rel_name: R::NAME,
            where_sql: handle.where_clause,
            where_params: handle.where_params,
            order_by_sql: handle.order_by_clause,
            limit: handle.limit,
            offset: handle.offset,
            nested,
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
            write_qualified_column(alias, c, &mut sql);
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
            &mut params,
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

        // WHERE
        if !where_clause.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause.replace(&table_prefix, &alias_prefix));
            for p in where_params {
                params.push(p);
            }
        }

        // ORDER BY
        if !order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_by.replace(&table_prefix, &alias_prefix));
        }
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

/// Writes a correlated subquery for a single relation directly into `sql`.
fn write_relation_subquery<'p, V: SQLParam>(
    rel: &'p RenderedRelation<V>,
    parent_alias: &str,
    alias_counter: &mut usize,
    param_counter: &mut usize,
    sql: &mut String,
    params: &mut Vec<&'p V>,
) {
    let alias_num = *alias_counter;
    *alias_counter += 1;
    let mut alias_buf = String::with_capacity(4);
    alias_buf.push('t');
    let _ = write!(alias_buf, "{alias_num}");
    let alias = &alias_buf;

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

    // json_object(...) / json_build_object(...)
    write_json_object_open(dialect, sql);
    let mut first_arg = true;
    for c in target_columns {
        if !first_arg {
            sql.push_str(", ");
        }
        first_arg = false;
        sql.push('\'');
        sql.push_str(c);
        sql.push_str("', ");
        write_qualified_column(alias, c, sql);
    }

    // Nested relation subqueries as additional json_object args
    for nested_rel in &rel.nested {
        if !first_arg {
            sql.push_str(", ");
        }
        first_arg = false;
        sql.push('\'');
        sql.push_str(nested_rel.rel_name);
        sql.push_str("', ");
        write_relation_subquery::<V>(nested_rel, alias, alias_counter, param_counter, sql, params);
    }

    sql.push(')'); // close json_object / json_build_object

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
        // Inner subquery: (SELECT cols FROM table AS alias WHERE ... ORDER BY ... LIMIT N)
        sql.push_str("(SELECT ");
        for (i, c) in target_columns.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            write_qualified_column(alias, c, sql);
        }
        sql.push_str(" FROM \"");
        sql.push_str(target_table);
        sql.push_str("\" AS \"");
        sql.push_str(alias);
        sql.push_str("\" WHERE ");
    } else {
        sql.push('"');
        sql.push_str(target_table);
        sql.push_str("\" AS \"");
        sql.push_str(alias);
        sql.push_str("\" WHERE ");
    }

    // FK join conditions
    for (i, (src_col, tgt_col)) in fk_columns.iter().enumerate() {
        if i > 0 {
            sql.push_str(" AND ");
        }
        sql.push('"');
        sql.push_str(alias);
        sql.push_str("\".\"");
        sql.push_str(src_col);
        sql.push_str("\" = \"");
        sql.push_str(parent_alias);
        sql.push_str("\".\"");
        sql.push_str(tgt_col);
        sql.push('"');
    }

    // Additional WHERE and ORDER BY (rewrite table references to alias).
    // Skip ORDER BY here when it was already placed inside json_agg (pg_order_in_agg).
    let has_trailing_order = !rel.order_by_sql.is_empty() && !pg_order_in_agg;
    if !rel.where_sql.is_empty() || has_trailing_order {
        let table_prefix = format!("\"{target_table}\".");
        let alias_prefix = format!("\"{alias}\".");

        if !rel.where_sql.is_empty() {
            sql.push_str(" AND ");
            let rewritten = rel.where_sql.replace(&table_prefix, &alias_prefix);
            // For numbered placeholders ($1, $2, ...), offset to account for
            // params already collected before this clause.
            let rewritten = renumber_placeholders(dialect, &rewritten, *param_counter);
            sql.push_str(&rewritten);
            *param_counter += rel.where_params.len();
            for p in &rel.where_params {
                params.push(p);
            }
        }

        if has_trailing_order {
            sql.push_str(" ORDER BY ");
            sql.push_str(&rel.order_by_sql.replace(&table_prefix, &alias_prefix));
        }
    }

    // LIMIT / OFFSET
    match cardinality {
        RelCardinality::One | RelCardinality::OptionalOne => {
            sql.push_str(" LIMIT 1");
        }
        RelCardinality::Many => {
            if let Some(n) = rel.limit {
                sql.push_str(" LIMIT ");
                let _ = write!(sql, "{n}");
            }
        }
    }

    if let Some(n) = rel.offset {
        sql.push_str(" OFFSET ");
        let _ = write!(sql, "{n}");
    }

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

/// Opens a JSON object constructor.
/// SQLite: `json_object(`, PostgreSQL: `json_build_object(`
fn write_json_object_open(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.push_str("json_object("),
        Dialect::PostgreSQL => sql.push_str("json_build_object("),
    }
}

/// Opens a JSON array aggregation wrapper for Many relations.
/// SQLite: `json_group_array(`, PostgreSQL: `COALESCE(json_agg(`
fn write_json_array_agg_open(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.push_str("json_group_array("),
        Dialect::PostgreSQL => sql.push_str("COALESCE(json_agg("),
    }
}

/// Closes a JSON array aggregation wrapper for Many relations.
/// SQLite: `)`, PostgreSQL: `), '[]'::json)`
fn write_json_array_agg_close(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite | Dialect::MySQL => sql.push(')'),
        Dialect::PostgreSQL => sql.push_str("), '[]'::json)"),
    }
}

/// Renumbers `$N` placeholders in a pre-built SQL fragment to start at `offset`.
///
/// For SQLite (`?` placeholders), returns the input unchanged.
/// For PostgreSQL, rewrites `$1` → `$offset`, `$2` → `$offset+1`, etc.
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
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
            // Parse the original number
            let start = i + 1;
            let mut end = start;
            while end < len && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let orig: usize = sql[start..end].parse().unwrap_or(0);
            let new_num = orig + offset - 1; // $1 -> $offset, $2 -> $offset+1
            result.push('$');
            let _ = write!(result, "{new_num}");
            i = end;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}
