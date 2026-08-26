//! SQL generation for the relational Query API.
//!
//! Renders typed relation structures into SQL with JSON subqueries.
//! Uses `V::DIALECT` to dispatch between SQLite, PostgreSQL, and MySQL syntax.

use core::fmt::Write;

use crate::SQL;
use crate::SQLParam;
use crate::dialect::Dialect;
use crate::prelude::*;
use crate::relation::{CardWrap, JunctionMeta, RelationDef};
use crate::sql::{SQLChunk, Token, write_dialect_quoted_ident};

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
    /// Column names that store or may store BLOB data and need tagged,
    /// storage-class-aware JSON projection.
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

    // PostgreSQL evaluates SELECT-list subqueries for every row the plan
    // produces before LIMIT/OFFSET discard it — an OFFSET of N runs each
    // relation subquery N extra times. Pushing the base scan into a derived
    // table applies pagination first, so relation subqueries only run for
    // rows that survive it. SQLite skips OFFSET rows before evaluating the
    // projection, so it keeps the flat shape.
    let paginate_first = dialect == Dialect::PostgreSQL
        && !relations.is_empty()
        && (limit.is_some() || offset.is_some());

    // Columns the derived table must expose beyond the selection: parent-side
    // FK columns for the relation joins and ORDER BY columns re-applied in
    // the outer query. Collected before `relations` is consumed below.
    let inner_extra_cols = if paginate_first {
        let mut extra = collect_nested_extra_cols(&relations, column_names);
        for chunk in &order_by_sql.chunks {
            if let SQLChunk::Column(column) = chunk
                && column.table == table_name
                && !column_names.contains(&column.name)
                && !extra.contains(&column.name)
            {
                extra.push(column.name);
            }
        }
        extra
    } else {
        Vec::new()
    };

    // SELECT base columns
    sql.push_str("SELECT ");

    if wrap_base_json {
        // Wrap base columns in json_object/json_build_object as "__base"
        write_json_object_open(dialect, sql.buf_mut());
        for (i, c) in column_names.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            write_json_key(dialect, c, sql.buf_mut());
            sql.push_str(", ");
            write_json_column(alias, c, blob_columns, dialect, sql.buf_mut());
        }
        sql.push(')');
        if dialect == Dialect::PostgreSQL {
            sql.push_str("::text");
        }
        sql.push_str(" AS ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), "__base");
    } else {
        for (i, c) in column_names.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            write_qualified_column(dialect, alias, c, sql.buf_mut());
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
        let mut relation_alias = String::from("__rel_");
        relation_alias.push_str(rel_name);
        sql.push_str(" AS ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), &relation_alias);
    }

    // FROM
    if paginate_first {
        // Derived table: base scan with WHERE/ORDER BY/LIMIT/OFFSET applied
        // inside, re-exposed under the same alias for the outer projection.
        sql.push_str(" FROM (SELECT ");
        for (i, c) in column_names
            .iter()
            .chain(inner_extra_cols.iter())
            .enumerate()
        {
            if i > 0 {
                sql.push_str(", ");
            }
            write_qualified_column(dialect, alias, c, sql.buf_mut());
        }
        sql.push_str(" FROM ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), table_name);
        sql.push_str(" AS ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), alias);

        if !where_sql.chunks.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_fragment(where_sql, table_name, alias);
        }

        // Row order out of a derived table is not guaranteed, so the ORDER BY
        // also gets re-applied in the outer query below.
        let outer_order_by = (!order_by_sql.chunks.is_empty()).then(|| order_by_sql.clone());
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

        sql.push_rparen();
        sql.push_str(" AS ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), alias);

        if let Some(order_by_sql) = outer_order_by {
            sql.push_str(" ORDER BY ");
            sql.push_fragment(order_by_sql, table_name, alias);
        }

        return sql.finish();
    }

    sql.push_str(" FROM ");
    write_dialect_quoted_ident(dialect, sql.buf_mut(), table_name);
    sql.push_str(" AS ");
    write_dialect_quoted_ident(dialect, sql.buf_mut(), alias);

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
    } else if dialect == Dialect::MySQL && offset.is_some() {
        // MySQL does not accept a bare OFFSET. Its documented unbounded-limit
        // sentinel preserves the caller's offset-only intent.
        sql.push_str(" LIMIT 18446744073709551615");
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
                    write_dialect_quoted_ident(V::DIALECT, &mut self.buf, alias);
                    self.buf.push('.');
                    write_dialect_quoted_ident(V::DIALECT, &mut self.buf, column.name);
                }
                SQLChunk::Table(table) if table.name == target_table => {
                    write_dialect_quoted_ident(V::DIALECT, &mut self.buf, alias);
                }
                other => {
                    self.flush();
                    self.sql.push_mut(other);
                }
            }
        }
    }

    /// Pushes a `)` as a token chunk. Raw `")"` text directly after a bound
    /// parameter renders with a stray space (`"$1 )"`); the token form
    /// follows the renderer's punctuation spacing rules instead.
    fn push_rparen(&mut self) {
        self.flush();
        self.sql.push_mut(SQLChunk::Token(Token::RPAREN));
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

/// Writes the inner-subquery select list (`[LATERAL ](SELECT cols`) used when
/// a Many relation needs a nested derived table (LIMIT/OFFSET/ORDER BY). The
/// table/alias/junction/WHERE suffix is emitted by the caller and shared with
/// the non-subquery path.
fn write_inner_subquery_select_list(
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
        write_qualified_column(dialect, alias, c, sql);
    }
    for c in extra_cols {
        sql.push_str(", ");
        write_qualified_column(dialect, alias, c, sql);
    }
}

fn collect_nested_extra_cols<V: SQLParam>(
    nested: &[RenderedRelation<'_, V>],
    target_columns: &[&str],
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
        write_json_key(dialect, c, ctx.sql.buf_mut());
        ctx.sql.push_str(", ");
        write_json_column(alias, c, blob_columns, dialect, ctx.sql.buf_mut());
    }

    // Nested relation subqueries as additional json_object args.
    for nested_rel in nested {
        if !first_arg {
            ctx.sql.push_str(", ");
        }
        first_arg = false;
        write_json_key(dialect, nested_rel.rel_name, ctx.sql.buf_mut());
        ctx.sql.push_str(", ");
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

fn alloc_internal_column_name(target_columns: &[&str], extra_cols: &[&str]) -> String {
    let mut name = String::from("__drizzle_order");
    let mut suffix = 0usize;
    while target_columns
        .iter()
        .chain(extra_cols)
        .any(|column| column.eq_ignore_ascii_case(&name))
    {
        suffix += 1;
        name.clear();
        name.push_str("__drizzle_order_");
        let _ = write!(name, "{suffix}");
    }
    name
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
    let has_order_by = order_by_sql
        .as_ref()
        .is_some_and(|order_by_sql| !order_by_sql.chunks.is_empty());

    if !where_sql.chunks.is_empty() {
        ctx.sql.push_str(" AND ");
        ctx.sql.push_fragment(where_sql, target_table, alias);
    }

    if !pg_order_in_agg && has_order_by {
        ctx.sql.push_str(" ORDER BY ");
        if let Some(order_by_sql) = order_by_sql {
            ctx.sql.push_fragment(order_by_sql, target_table, alias);
        }
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
            } else if V::DIALECT == Dialect::MySQL && offset.is_some() {
                ctx.sql.push_str(" LIMIT 18446744073709551615");
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
    dialect: Dialect,
    junction: Option<&JunctionMeta>,
    alias: &str,
    parent_alias: &str,
    junction_alias: Option<&str>,
    fk_columns: &[(&str, &str)],
    sql: &mut String,
) {
    let push_pair = |a: &str, b: &str, ca: &str, cb: &str, sql: &mut String| {
        write_qualified_column(dialect, a, ca, sql);
        sql.push_str(" = ");
        write_qualified_column(dialect, b, cb, sql);
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
    let mysql_ordered_many =
        cardinality == RelCardinality::Many && dialect == Dialect::MySQL && has_order_by;
    let materializer_order_by = mysql_ordered_many.then(|| order_by_sql.clone());
    let mysql_order_column = alloc_internal_column_name(&target_columns, &extra_cols);

    let mut order_by_sql = Some(order_by_sql);

    if mysql_ordered_many {
        sql.push_str("COALESCE((SELECT ");
    } else {
        sql.push_str("(SELECT ");
    }

    // json_group_array( / COALESCE(json_agg( wrapper for Many
    if cardinality == RelCardinality::Many {
        if mysql_ordered_many {
            sql.push_str("JSON_ARRAYAGG(");
        } else {
            write_json_array_agg_open(dialect, sql.buf_mut());
        }
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
        if mysql_ordered_many {
            sql.push_str(") OVER (ORDER BY ");
            write_qualified_column(dialect, alias, &mysql_order_column, sql.buf_mut());
            sql.push_str(" ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)");
        } else {
            write_json_array_agg_close(dialect, sql.buf_mut());
        }
    }

    // FROM
    sql.push_str(" FROM ");

    if needs_inner_subquery {
        write_inner_subquery_select_list(
            alias,
            &target_columns,
            &extra_cols,
            dialect,
            sql.buf_mut(),
        );
        if let Some(materializer_order_by) = materializer_order_by {
            // MySQL JSON_ARRAYAGG has no aggregate-local ORDER BY. Project a
            // stable ordinal for the explicit ordered window aggregate above.
            sql.push_str(", ROW_NUMBER() OVER (ORDER BY ");
            sql.push_fragment(materializer_order_by, target_table, alias);
            sql.push_str(") AS ");
            write_dialect_quoted_ident(dialect, sql.buf_mut(), &mysql_order_column);
        }
        sql.push_str(" FROM ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), target_table);
    } else {
        write_dialect_quoted_ident(dialect, sql.buf_mut(), target_table);
    }
    sql.push_str(" AS ");
    write_dialect_quoted_ident(dialect, sql.buf_mut(), alias);
    if let (Some(junction), Some(junc_alias)) = (&junction, &junction_alias) {
        write_junction_join(dialect, junction, alias, junc_alias, sql.buf_mut());
    }
    sql.push_str(" WHERE ");

    // FK join conditions — junction replaces direct FK with INNER JOIN + WHERE
    write_fk_join_conditions(
        dialect,
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
        sql.push_rparen();
        sql.push_str(" AS ");
        write_dialect_quoted_ident(dialect, sql.buf_mut(), alias);
    }

    if mysql_ordered_many {
        // Each input row carries the same full-frame window result. Select one
        // row, then supply [] when the scalar subquery has no input rows.
        sql.push_str(" LIMIT 1), JSON_ARRAY())");
    } else {
        sql.push(')'); // close outer (SELECT ...)
    }
}

// =============================================================================
// Dialect-specific helpers
// =============================================================================

/// Writes a dialect-quoted `alias.column` reference into the buffer.
fn write_qualified_column(dialect: Dialect, alias: &str, column: &str, sql: &mut String) {
    write_dialect_quoted_ident(dialect, sql, alias);
    sql.push('.');
    write_dialect_quoted_ident(dialect, sql, column);
}

/// Writes a JSON object key without depending on MySQL's backslash SQL mode.
fn write_json_key(dialect: Dialect, value: &str, sql: &mut String) {
    if dialect == Dialect::MySQL {
        sql.push_str("CONVERT(X'");
        for byte in value.as_bytes() {
            let _ = write!(sql, "{byte:02X}");
        }
        sql.push_str("' USING utf8mb4)");
        return;
    }

    sql.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            sql.push_str("''");
        } else {
            sql.push(ch);
        }
    }
    sql.push('\'');
}

/// Writes an `INNER JOIN` clause for a junction (many-to-many) table.
///
/// Generates: `INNER JOIN "junction" AS "junc_alias" ON "junc_alias"."col" = "target_alias"."col"`
fn write_junction_join(
    dialect: Dialect,
    junction: &JunctionMeta,
    target_alias: &str,
    junc_alias: &str,
    sql: &mut String,
) {
    sql.push_str(" INNER JOIN ");
    write_dialect_quoted_ident(dialect, sql, junction.table_name);
    sql.push_str(" AS ");
    write_dialect_quoted_ident(dialect, sql, junc_alias);
    sql.push_str(" ON ");
    for (i, (junc_col, target_col)) in junction.target_fk.iter().enumerate() {
        if i > 0 {
            sql.push_str(" AND ");
        }
        write_qualified_column(dialect, junc_alias, junc_col, sql);
        sql.push_str(" = ");
        write_qualified_column(dialect, target_alias, target_col, sql);
    }
}

/// Writes a column reference for use inside `json_object()`.
///
/// For columns that may use BLOB storage on `SQLite`, preserves the runtime
/// storage class and value in a tagged JSON object. BLOB values are hex-encoded
/// because SQLite's JSON functions cannot serialize them directly.
///
/// SQL NULL remains JSON null rather than a tagged object so nullable field
/// decoding retains its ordinary `None` representation.
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
    let is_blob = blob_columns.contains(&column);
    if dialect == Dialect::SQLite && is_blob {
        sql.push_str("json(CASE WHEN ");
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str(" IS NULL THEN NULL ELSE json_object('$drizzle_storage', typeof(");
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str("), '$drizzle_value', CASE WHEN typeof(");
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str(") = 'blob' THEN hex(");
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str(") ELSE ");
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str(" END) END)");
        return;
    }

    if dialect == Dialect::MySQL && is_blob {
        // MySQL JSON constructors reject binary-character-set strings. Keep
        // the same tagged-value contract used by SQLite while encoding the
        // payload as hexadecimal text for lossless driver-side decoding.
        sql.push_str("CASE WHEN ");
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str(
            " IS NULL THEN NULL ELSE JSON_OBJECT('$drizzle_storage', 'blob', '$drizzle_value', HEX(",
        );
        write_qualified_column(dialect, alias, column, sql);
        sql.push_str(")) END");
        return;
    }

    write_qualified_column(dialect, alias, column, sql);
}

/// Opens a JSON object constructor.
fn write_json_object_open(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite => sql.push_str("json_object("),
        Dialect::MySQL => sql.push_str("JSON_OBJECT("),
        Dialect::PostgreSQL => sql.push_str("json_build_object("),
    }
}

/// Opens a JSON array aggregation wrapper for Many relations.
fn write_json_array_agg_open(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite => sql.push_str("json_group_array("),
        Dialect::MySQL => sql.push_str("COALESCE(JSON_ARRAYAGG("),
        Dialect::PostgreSQL => sql.push_str("COALESCE(json_agg("),
    }
}

/// Closes a JSON array aggregation wrapper for Many relations.
fn write_json_array_agg_close(dialect: Dialect, sql: &mut String) {
    match dialect {
        Dialect::SQLite => sql.push(')'),
        Dialect::MySQL => sql.push_str("), JSON_ARRAY())"),
        Dialect::PostgreSQL => sql.push_str("), '[]'::json)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MySQLDialect, SQLParam};

    #[derive(Clone, Debug)]
    struct MySQLTestValue;

    impl SQLParam for MySQLTestValue {
        const DIALECT: Dialect = Dialect::MySQL;
        type DialectMarker = MySQLDialect;
    }

    impl From<MySQLTestValue> for Cow<'_, MySQLTestValue> {
        fn from(value: MySQLTestValue) -> Self {
            Cow::Owned(value)
        }
    }

    #[test]
    fn mysql_qualified_column_escapes_backticks() {
        let mut sql = String::new();
        write_qualified_column(Dialect::MySQL, "account`owner", "display`name", &mut sql);
        assert_eq!(sql, "`account``owner`.`display``name`");
    }

    #[test]
    fn json_key_literal_escapes_single_quotes() {
        let mut sql = String::new();
        write_json_key(Dialect::SQLite, "owner's posts", &mut sql);
        assert_eq!(sql, "'owner''s posts'");
    }

    #[test]
    fn mysql_json_key_is_safe_in_every_backslash_mode() {
        let mut sql = String::new();
        write_json_key(Dialect::MySQL, "x\\'; DROP TABLE audit; --", &mut sql);
        assert_eq!(
            sql,
            "CONVERT(X'785C273B2044524F50205441424C452061756469743B202D2D' USING utf8mb4)"
        );
    }

    #[test]
    fn mysql_many_relation_uses_mysql_json_aggregation() {
        let mut sql = String::new();
        write_json_array_agg_open(Dialect::MySQL, &mut sql);
        sql.push_str("JSON_OBJECT()");
        write_json_array_agg_close(Dialect::MySQL, &mut sql);
        assert_eq!(sql, "COALESCE(JSON_ARRAYAGG(JSON_OBJECT()), JSON_ARRAY())");
    }

    #[test]
    fn mysql_binary_json_values_use_the_tagged_hex_contract() {
        let mut sql = String::new();
        write_json_column("account", "avatar", &["avatar"], Dialect::MySQL, &mut sql);

        assert_eq!(
            sql,
            "CASE WHEN `account`.`avatar` IS NULL THEN NULL ELSE JSON_OBJECT('$drizzle_storage', 'blob', '$drizzle_value', HEX(`account`.`avatar`)) END"
        );
    }

    #[test]
    fn mysql_offset_only_uses_the_unbounded_limit_sentinel() {
        let sql = build_query_sql::<MySQLTestValue>(
            "account",
            &["id"],
            &[],
            vec![],
            SQL::empty(),
            SQL::empty(),
            None,
            Some(SQL::param(MySQLTestValue)),
            false,
        )
        .sql();

        assert_eq!(
            sql,
            "SELECT `t0`.`id` FROM `account` AS `t0` LIMIT 18446744073709551615 OFFSET ?"
        );
    }

    #[test]
    fn mysql_relation_offset_only_uses_the_unbounded_limit_sentinel() {
        let relation = RenderedRelation::<MySQLTestValue> {
            table_name: "post",
            column_names: vec!["id"],
            blob_columns: &[],
            fk_columns: &[("author_id", "id")],
            cardinality: RelCardinality::Many,
            rel_name: "posts",
            where_sql: SQL::empty(),
            order_by_sql: SQL::empty(),
            limit: None,
            offset: Some(SQL::param(MySQLTestValue)),
            nested: vec![],
            junction: None,
        };

        let sql = build_query_sql::<MySQLTestValue>(
            "user",
            &["id"],
            &[],
            vec![relation],
            SQL::empty(),
            SQL::empty(),
            None,
            None,
            false,
        )
        .sql();

        assert_eq!(
            sql,
            "SELECT `t0`.`id`, (SELECT COALESCE(JSON_ARRAYAGG(JSON_OBJECT(CONVERT(X'6964' USING utf8mb4), `t1`.`id`)), JSON_ARRAY()) FROM (SELECT `t1`.`id` FROM `post` AS `t1` WHERE `t1`.`author_id` = `t0`.`id` LIMIT 18446744073709551615 OFFSET ?) AS `t1`) AS `__rel_posts` FROM `user` AS `t0`"
        );
    }

    #[test]
    fn mysql_ordered_relation_uses_an_explicit_ordered_window_aggregate() {
        let relation = RenderedRelation::<MySQLTestValue> {
            table_name: "post",
            column_names: vec!["id"],
            blob_columns: &[],
            fk_columns: &[("author_id", "id")],
            cardinality: RelCardinality::Many,
            rel_name: "posts",
            where_sql: SQL::empty(),
            order_by_sql: SQL::raw("`t1`.`id` DESC"),
            limit: None,
            offset: None,
            nested: vec![],
            junction: None,
        };

        let sql = build_query_sql::<MySQLTestValue>(
            "user",
            &["id"],
            &[],
            vec![relation],
            SQL::empty(),
            SQL::empty(),
            None,
            None,
            false,
        )
        .sql();

        assert!(
            sql.contains("ROW_NUMBER() OVER (ORDER BY `t1`.`id` DESC ) AS `__drizzle_order`"),
            "{sql}"
        );
        assert!(
            sql.contains(
                "JSON_ARRAYAGG(JSON_OBJECT(CONVERT(X'6964' USING utf8mb4), `t1`.`id`)) OVER (ORDER BY `t1`.`__drizzle_order` ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)"
            ),
            "{sql}"
        );
        assert!(
            sql.ends_with("LIMIT 1), JSON_ARRAY()) AS `__rel_posts` FROM `user` AS `t0`"),
            "{sql}"
        );
    }

    #[test]
    fn mysql_ordered_relation_avoids_internal_column_name_collisions() {
        let relation = RenderedRelation::<MySQLTestValue> {
            table_name: "post",
            column_names: vec!["id", "__DRIZZLE_ORDER", "__Drizzle_Order_1"],
            blob_columns: &[],
            fk_columns: &[("author_id", "id")],
            cardinality: RelCardinality::Many,
            rel_name: "posts",
            where_sql: SQL::empty(),
            order_by_sql: SQL::raw("`t1`.`id` DESC"),
            limit: None,
            offset: None,
            nested: vec![],
            junction: None,
        };

        let sql = build_query_sql::<MySQLTestValue>(
            "user",
            &["id"],
            &[],
            vec![relation],
            SQL::empty(),
            SQL::empty(),
            None,
            None,
            false,
        )
        .sql();

        assert!(
            sql.contains("ROW_NUMBER() OVER (ORDER BY `t1`.`id` DESC ) AS `__drizzle_order_2`"),
            "{sql}"
        );
        assert!(
            sql.contains("OVER (ORDER BY `t1`.`__drizzle_order_2` ROWS BETWEEN"),
            "{sql}"
        );
    }

    #[test]
    fn mysql_relational_query_quotes_every_identifier_and_json_key() {
        let relation = RenderedRelation::<MySQLTestValue> {
            table_name: "role`table",
            column_names: vec!["role`id", "label"],
            blob_columns: &[],
            fk_columns: &[("role`id", "account`id")],
            cardinality: RelCardinality::Many,
            rel_name: "roles'\\`",
            where_sql: SQL::empty(),
            order_by_sql: SQL::empty(),
            limit: None,
            offset: None,
            nested: vec![],
            junction: Some(JunctionMeta {
                table_name: "account`roles",
                source_fk: &[("account`fk", "account`id")],
                target_fk: &[("role`fk", "role`id")],
            }),
        };

        let sql = build_query_sql::<MySQLTestValue>(
            "account`table",
            &["account`id", "display`name"],
            &[],
            vec![relation],
            SQL::empty(),
            SQL::empty(),
            None,
            None,
            false,
        )
        .sql();

        assert_eq!(
            sql,
            r#"SELECT `t0`.`account``id`, `t0`.`display``name`, (SELECT COALESCE(JSON_ARRAYAGG(JSON_OBJECT(CONVERT(X'726F6C65606964' USING utf8mb4), `t1`.`role``id`, CONVERT(X'6C6162656C' USING utf8mb4), `t1`.`label`)), JSON_ARRAY()) FROM `role``table` AS `t1` INNER JOIN `account``roles` AS `t2` ON `t2`.`role``fk` = `t1`.`role``id` WHERE `t2`.`account``fk` = `t0`.`account``id`) AS `__rel_roles'\``` FROM `account``table` AS `t0`"#
        );
    }
}
