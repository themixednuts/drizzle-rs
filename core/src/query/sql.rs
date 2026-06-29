//! SQL generation for the relational Query API.
//!
//! Renders typed relation structures into SQL with JSON subqueries.
//! Uses `V::DIALECT` to dispatch between `SQLite` and `PostgreSQL` syntax.

use core::fmt::Write;

use crate::SQL;
use crate::SQLParam;
use crate::dialect::Dialect;
use crate::prelude::*;
use crate::relation::{CardWrap, JunctionMeta, RelationDef};
use crate::sql::{SQLChunk, write_quoted_ident};

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
pub struct RenderedRelation<'a, V: SQLParam> {
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
    /// Pre-rendered WHERE SQL fragment.
    pub where_sql: SQL<'a, V>,
    /// Pre-rendered ORDER BY SQL fragment.
    pub order_by_sql: SQL<'a, V>,
    /// LIMIT fragment.
    pub limit: Option<SQL<'a, V>>,
    /// OFFSET fragment.
    pub offset: Option<SQL<'a, V>>,
    /// Nested rendered relations.
    pub nested: Vec<Self>,
    /// Junction table metadata for many-to-many relations.
    pub junction: Option<JunctionMeta>,
}

/// Converts a typed relation structure into `Vec<RenderedRelation<V>>`.
pub trait RenderRelations<'a, V: SQLParam> {
    /// Appends rendered relations to `out`, consuming self.
    fn render_into(self, out: &mut Vec<RenderedRelation<'a, V>>);
}

impl<'a, V: SQLParam> RenderRelations<'a, V> for () {
    #[inline]
    fn render_into(self, _out: &mut Vec<RenderedRelation<'a, V>>) {}
}

// AllColumns: use all columns from QueryTable
impl<'a, V, R, Nested, Rest, Cl> RenderRelations<'a, V>
    for (RelationHandle<'a, V, R, Nested, AllColumns, Cl>, Rest)
where
    V: SQLParam,
    R: RelationDef,
    Nested: RenderRelations<'a, V>,
    Rest: RenderRelations<'a, V>,
{
    fn render_into(self, out: &mut Vec<RenderedRelation<'a, V>>) {
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
            where_sql: handle.where_sql,
            order_by_sql: handle.order_by_sql,
            limit: handle.limit,
            offset: handle.offset,
            nested,
            junction: R::junction(),
        });
        rest.render_into(out);
    }
}

// PartialColumns: use filtered columns from the handle
impl<'a, V, R, Nested, Rest, Cl> RenderRelations<'a, V>
    for (RelationHandle<'a, V, R, Nested, PartialColumns, Cl>, Rest)
where
    V: SQLParam,
    R: RelationDef,
    Nested: RenderRelations<'a, V>,
    Rest: RenderRelations<'a, V>,
{
    fn render_into(self, out: &mut Vec<RenderedRelation<'a, V>>) {
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
            where_sql: handle.where_sql,
            order_by_sql: handle.order_by_sql,
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

/// Generates the full SQL for a query with relations.
///
/// When `wrap_base_json` is true, base columns are wrapped in a JSON object
/// (`json_object(...)` / `json_build_object(...)`) as a single `"__base"` column.
/// This is used for partial column selection.
///
/// Uses `V::DIALECT` to select the correct JSON functions and placeholder style.
#[allow(clippy::too_many_arguments)]
pub fn build_query_sql<'a, V: SQLParam>(
    table_name: &str,
    column_names: &[&str],
    blob_columns: &[&str],
    relations: Vec<RenderedRelation<'a, V>>,
    where_sql: SQL<'a, V>,
    order_by_sql: SQL<'a, V>,
    limit: Option<SQL<'a, V>>,
    offset: Option<SQL<'a, V>>,
    wrap_base_json: bool,
) -> SQL<'a, V> {
    let mut sql = QuerySql::new();
    let alias = "t0";
    let dialect = V::DIALECT;

    // SELECT base columns
    sql.push_str("SELECT ");

    if wrap_base_json {
        // Wrap base columns in json_object/json_build_object as "__base"
        write_json_object_open(dialect, sql.buf_mut());
        for (i, c) in column_names.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('\'');
            sql.push_str(c);
            sql.push_str("', ");
            write_json_column(alias, c, blob_columns, dialect, sql.buf_mut());
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
            write_qualified_column(alias, c, sql.buf_mut());
        }
    }

    // Add relation subqueries as additional SELECT columns.
    let mut alias_counter = 1usize;
    for rel in relations {
        let rel_name = rel.rel_name;
        sql.push_str(", ");
        write_relation_subquery::<V>(rel, alias, &mut alias_counter, &mut sql);
        // PostgreSQL returns json type — cast to text so the driver reads it as String
        if dialect == Dialect::PostgreSQL {
            sql.push_str("::text");
        }
        sql.push_str(" AS \"__rel_");
        sql.push_str(rel_name);
        sql.push('"');
    }

    // FROM
    sql.push_str(" FROM \"");
    sql.push_str(table_name);
    sql.push_str("\" AS \"");
    sql.push_str(alias);
    sql.push('"');

    // Rewrite table references to use the alias.
    if !where_sql.chunks.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_fragment(where_sql, table_name, alias);
    }

    if !order_by_sql.chunks.is_empty() {
        sql.push_str(" ORDER BY ");
        sql.push_fragment(order_by_sql, table_name, alias);
    }

    if let Some(limit_sql) = limit {
        sql.push_str(" LIMIT ");
        sql.push_fragment(limit_sql, table_name, alias);
    }

    if let Some(offset_sql) = offset {
        sql.push_str(" OFFSET ");
        sql.push_fragment(offset_sql, table_name, alias);
    }

    sql.finish()
}

/// Scratch SQL accumulator for the relational query renderer.
///
/// Most relation SQL is static scaffolding, so it is buffered as raw text and
/// flushed into the chunk list only when a typed user fragment is inserted.
struct QuerySql<'a, V: SQLParam> {
    sql: SQL<'a, V>,
    buf: String,
}

impl<'a, V: SQLParam> QuerySql<'a, V> {
    fn new() -> Self {
        Self {
            sql: SQL::empty(),
            buf: String::with_capacity(256),
        }
    }

    fn buf_mut(&mut self) -> &mut String {
        &mut self.buf
    }

    fn push(&mut self, ch: char) {
        self.buf.push(ch);
    }

    fn push_str(&mut self, text: &str) {
        self.buf.push_str(text);
    }

    fn push_fragment(&mut self, fragment: SQL<'a, V>, target_table: &str, alias: &str) {
        for chunk in fragment.chunks {
            match chunk {
                SQLChunk::Column(column) if column.table == target_table => {
                    write_quoted_ident(&mut self.buf, alias);
                    self.buf.push('.');
                    write_quoted_ident(&mut self.buf, column.name);
                }
                SQLChunk::Table(table) if table.name == target_table => {
                    write_quoted_ident(&mut self.buf, alias);
                }
                other => {
                    self.flush();
                    self.sql.push_mut(other);
                }
            }
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            self.sql
                .push_mut(SQLChunk::Raw(Cow::Owned(core::mem::take(&mut self.buf))));
        }
    }

    fn finish(mut self) -> SQL<'a, V> {
        self.flush();
        self.sql
    }
}

/// Writes the inner-subquery prefix (`[LATERAL ](SELECT cols FROM "`) used when
/// a Many relation needs a nested derived table (LIMIT/OFFSET/ORDER BY). The
/// table/alias/junction/WHERE suffix is emitted by the caller and shared with
/// the non-subquery path.
fn write_inner_subquery_prelude(
    target_table: &str,
    alias: &str,
    target_columns: &[&'static str],
    extra_cols: &[&str],
    dialect: Dialect,
    sql: &mut String,
) {
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
    for c in extra_cols {
        sql.push_str(", ");
        write_qualified_column(alias, c, sql);
    }
    sql.push_str(" FROM \"");
    sql.push_str(target_table);
}

fn collect_nested_extra_cols<V: SQLParam>(
    nested: &[RenderedRelation<'_, V>],
    target_columns: &[&'static str],
) -> Vec<&'static str> {
    let mut extra_cols = Vec::new();
    for nested_rel in nested {
        if let Some(junction) = &nested_rel.junction {
            for (_, src_col) in junction.source_fk {
                if !target_columns.contains(src_col) && !extra_cols.contains(src_col) {
                    extra_cols.push(*src_col);
                }
            }
        } else {
            for (_, tgt_col) in nested_rel.fk_columns {
                if !target_columns.contains(tgt_col) && !extra_cols.contains(tgt_col) {
                    extra_cols.push(*tgt_col);
                }
            }
        }
    }
    extra_cols
}

/// Writes the `json_object(...)` / `json_build_object` body: first the
/// base columns with literal keys, then nested relations recursively rendered
/// as named subqueries. Emits the trailing `)` that closes the object.
fn write_json_object_body<'a, V: SQLParam>(
    blob_columns: &[&str],
    nested: Vec<RenderedRelation<'a, V>>,
    alias: &str,
    target_columns: &[&'static str],
    dialect: Dialect,
    ctx: &mut SubqueryCtx<'_, 'a, V>,
) {
    write_json_object_open(dialect, ctx.sql.buf_mut());
    let mut first_arg = true;
    for c in target_columns {
        if !first_arg {
            ctx.sql.push_str(", ");
        }
        first_arg = false;
        ctx.sql.push('\'');
        ctx.sql.push_str(c);
        ctx.sql.push_str("', ");
        write_json_column(alias, c, blob_columns, dialect, ctx.sql.buf_mut());
    }

    // Nested relation subqueries as additional json_object args.
    for nested_rel in nested {
        if !first_arg {
            ctx.sql.push_str(", ");
        }
        first_arg = false;
        ctx.sql.push('\'');
        ctx.sql.push_str(nested_rel.rel_name);
        ctx.sql.push_str("', ");
        write_relation_subquery::<V>(nested_rel, alias, ctx.alias_counter, ctx.sql);
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
struct SubqueryCtx<'s, 'a, V: SQLParam> {
    alias_counter: &'s mut usize,
    sql: &'s mut QuerySql<'a, V>,
}

struct RelationClauseSql<'a, V: SQLParam> {
    where_sql: SQL<'a, V>,
    order_by_sql: Option<SQL<'a, V>>,
    limit: Option<SQL<'a, V>>,
    offset: Option<SQL<'a, V>>,
}

/// Emits the additional WHERE predicates, trailing ORDER BY (when not already
/// inlined in `json_agg`), and LIMIT/OFFSET clauses for a relation subquery.
fn write_where_order_limit_offset<'a, V: SQLParam>(
    target_table: &str,
    alias: &str,
    pg_order_in_agg: bool,
    cardinality: RelCardinality,
    clauses: RelationClauseSql<'a, V>,
    ctx: &mut SubqueryCtx<'_, 'a, V>,
) {
    let RelationClauseSql {
        where_sql,
        order_by_sql,
        limit,
        offset,
    } = clauses;

    if !where_sql.chunks.is_empty() {
        ctx.sql.push_str(" AND ");
        ctx.sql.push_fragment(where_sql, target_table, alias);
    }

    if !pg_order_in_agg
        && let Some(order_by_sql) = order_by_sql
        && !order_by_sql.chunks.is_empty()
    {
        ctx.sql.push_str(" ORDER BY ");
        ctx.sql.push_fragment(order_by_sql, target_table, alias);
    }

    // LIMIT
    match cardinality {
        RelCardinality::One | RelCardinality::OptionalOne => {
            ctx.sql.push_str(" LIMIT 1");
        }
        RelCardinality::Many => {
            if let Some(limit_sql) = limit {
                ctx.sql.push_str(" LIMIT ");
                ctx.sql.push_fragment(limit_sql, target_table, alias);
            }
        }
    }

    if let Some(offset_sql) = offset {
        ctx.sql.push_str(" OFFSET ");
        ctx.sql.push_fragment(offset_sql, target_table, alias);
    }
}

/// Writes the FK equality predicates that join a relation's rows against the
/// parent row. If a junction table is present, the predicates are emitted
/// between the junction alias and the parent alias; otherwise they join the
/// relation's own alias to the parent.
fn write_fk_join_conditions(
    junction: Option<&JunctionMeta>,
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
    if let (Some(junction), Some(junc_alias)) = (junction, junction_alias) {
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
fn write_relation_subquery<'a, V: SQLParam>(
    rel: RenderedRelation<'a, V>,
    parent_alias: &str,
    alias_counter: &mut usize,
    sql: &mut QuerySql<'a, V>,
) {
    let RenderedRelation {
        table_name: target_table,
        column_names: target_columns,
        blob_columns,
        fk_columns,
        cardinality,
        where_sql,
        order_by_sql,
        nested,
        junction,
        limit,
        offset,
        ..
    } = rel;

    let alias_buf = alloc_alias(alias_counter);
    let alias = &alias_buf;

    // Allocate junction alias if this is a many-to-many relation.
    let junction_alias = junction.as_ref().map(|_| alloc_alias(alias_counter));

    let dialect = V::DIALECT;
    let has_order_by = !order_by_sql.chunks.is_empty();
    let extra_cols = collect_nested_extra_cols(&nested, &target_columns);

    // PostgreSQL optimization: ORDER BY inside json_agg() avoids an inner subquery.
    // `json_agg(expr ORDER BY ...)` is more efficient than wrapping in a derived table.
    // SQLite's json_group_array doesn't reliably support this, so keep the subquery there.
    let pg_order_in_agg = cardinality == RelCardinality::Many
        && dialect == Dialect::PostgreSQL
        && has_order_by
        && limit.is_none()
        && offset.is_none();

    // Many relations with LIMIT / OFFSET need a nested subquery so constraints
    // apply before aggregation. ORDER BY alone also needs one on SQLite (no
    // aggregate ORDER BY), but on PostgreSQL it goes inside json_agg instead.
    let needs_inner_subquery = cardinality == RelCardinality::Many
        && (limit.is_some() || offset.is_some() || (!pg_order_in_agg && has_order_by));

    let mut order_by_sql = Some(order_by_sql);

    // (SELECT
    sql.push_str("(SELECT ");

    // json_group_array( / COALESCE(json_agg( wrapper for Many
    if cardinality == RelCardinality::Many {
        write_json_array_agg_open(dialect, sql.buf_mut());
    }

    write_json_object_body::<V>(
        blob_columns,
        nested,
        alias,
        &target_columns,
        dialect,
        &mut SubqueryCtx { alias_counter, sql },
    );

    // PostgreSQL: ORDER BY inside json_agg — e.g. json_agg(expr ORDER BY "t1"."col" DESC)
    if pg_order_in_agg {
        sql.push_str(" ORDER BY ");
        if let Some(order_by_sql) = order_by_sql.take() {
            sql.push_fragment(order_by_sql, target_table, alias);
        }
    }

    // close json_group_array / json_agg for Many
    if cardinality == RelCardinality::Many {
        write_json_array_agg_close(dialect, sql.buf_mut());
    }

    // FROM
    sql.push_str(" FROM ");

    if needs_inner_subquery {
        write_inner_subquery_prelude(
            target_table,
            alias,
            &target_columns,
            &extra_cols,
            dialect,
            sql.buf_mut(),
        );
    } else {
        sql.push('"');
        sql.push_str(target_table);
    }
    sql.push_str("\" AS \"");
    sql.push_str(alias);
    sql.push('"');
    if let (Some(junction), Some(junc_alias)) = (&junction, &junction_alias) {
        write_junction_join(junction, alias, junc_alias, sql.buf_mut());
    }
    sql.push_str(" WHERE ");

    // FK join conditions — junction replaces direct FK with INNER JOIN + WHERE
    write_fk_join_conditions(
        junction.as_ref(),
        alias,
        parent_alias,
        junction_alias.as_deref(),
        fk_columns,
        sql.buf_mut(),
    );

    // Additional WHERE and ORDER BY, then LIMIT/OFFSET per cardinality.
    write_where_order_limit_offset(
        target_table,
        alias,
        pg_order_in_agg,
        cardinality,
        RelationClauseSql {
            where_sql,
            order_by_sql,
            limit,
            offset,
        },
        &mut SubqueryCtx { alias_counter, sql },
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
