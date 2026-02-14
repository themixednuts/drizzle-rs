//! Deterministic database seeding for drizzle-rs.
//!
//! Generates reproducible INSERT statements using type-aware generators
//! and column name heuristics. FK-aware topological ordering ensures
//! parent tables are seeded before children.
//!
//! # Example
//!
//! ```ignore
//! use drizzle_seed::SeedConfig;
//!
//! let schema = AppSchema::new();
//! let stmts = SeedConfig::sqlite(&schema)
//!     .seed(42)
//!     .count(&schema.users, 100)
//!     .count(&schema.posts, 500)
//!     .generate();
//! ```

pub(crate) mod batch;
pub(crate) mod config;
pub(crate) mod datasets;
pub(crate) mod generator;
pub(crate) mod inference;
pub(crate) mod rng;
pub(crate) mod topology;

pub use config::SeedConfig;
pub use generator::{Generator, GeneratorKind, RngCore, SeedValue};

use drizzle_core::{SQLColumnInfo, SQLTableInfo};
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
use drizzle_core::{OwnedSQL, SQL, SQLChunk, Token, param::Param, traits::ToSQL};

#[cfg(any(feature = "sqlite", feature = "postgres"))]
use std::borrow::Cow;

#[cfg(feature = "sqlite")]
pub use statement::SQLiteSeedStatement;

#[cfg(feature = "postgres")]
pub use statement::PostgresSeedStatement;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};

#[cfg(feature = "postgres")]
use drizzle_postgres::values::{OwnedPostgresValue, PostgresValue};

// ---------------------------------------------------------------------------
// Dialect marker types — encode the target database in the type system
// ---------------------------------------------------------------------------

/// SQLite dialect marker for type-safe seeder configuration.
#[cfg(feature = "sqlite")]
pub struct Sqlite;

/// PostgreSQL dialect marker for type-safe seeder configuration.
#[cfg(feature = "postgres")]
pub struct Postgres;

// ---------------------------------------------------------------------------
// Seed statement types
// ---------------------------------------------------------------------------

mod statement {
    #[cfg(any(feature = "sqlite", feature = "postgres"))]
    use super::*;

    // Generic OwnedSQL → SQL conversion (borrowing)
    #[cfg(any(feature = "sqlite", feature = "postgres"))]
    fn convert_to_sql<'a, Owned, Borrowed>(owned: &OwnedSQL<Owned>) -> SQL<'a, Borrowed>
    where
        Owned: drizzle_core::SQLParam,
        Borrowed: drizzle_core::SQLParam + From<Owned>,
    {
        let chunks = owned
            .chunks
            .iter()
            .map(|chunk| match chunk {
                drizzle_core::OwnedSQLChunk::Token(t) => SQLChunk::Token(*t),
                drizzle_core::OwnedSQLChunk::Ident(s) => SQLChunk::Ident(Cow::Owned(s.clone())),
                drizzle_core::OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(s.clone())),
                drizzle_core::OwnedSQLChunk::Number(v) => SQLChunk::Number(*v),
                drizzle_core::OwnedSQLChunk::Param(p) => SQLChunk::Param(Param {
                    placeholder: p.placeholder,
                    value: p
                        .value
                        .as_ref()
                        .map(|v| Cow::Owned(Borrowed::from(v.clone()))),
                }),
                drizzle_core::OwnedSQLChunk::Table(t) => SQLChunk::Table(*t),
                drizzle_core::OwnedSQLChunk::Column(c) => SQLChunk::Column(*c),
            })
            .collect();
        SQL { chunks }
    }

    // Generic OwnedSQL → SQL conversion (consuming — avoids cloning values)
    #[cfg(any(feature = "sqlite", feature = "postgres"))]
    fn convert_into_sql<'a, Owned, Borrowed>(owned: OwnedSQL<Owned>) -> SQL<'a, Borrowed>
    where
        Owned: drizzle_core::SQLParam,
        Borrowed: drizzle_core::SQLParam + From<Owned>,
    {
        let chunks = owned
            .chunks
            .into_iter()
            .map(|chunk| match chunk {
                drizzle_core::OwnedSQLChunk::Token(t) => SQLChunk::Token(t),
                drizzle_core::OwnedSQLChunk::Ident(s) => SQLChunk::Ident(Cow::Owned(s)),
                drizzle_core::OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(s)),
                drizzle_core::OwnedSQLChunk::Number(v) => SQLChunk::Number(v),
                drizzle_core::OwnedSQLChunk::Param(p) => SQLChunk::Param(Param {
                    placeholder: p.placeholder,
                    value: p.value.map(|v| Cow::Owned(Borrowed::from(v))),
                }),
                drizzle_core::OwnedSQLChunk::Table(t) => SQLChunk::Table(t),
                drizzle_core::OwnedSQLChunk::Column(c) => SQLChunk::Column(c),
            })
            .collect();
        SQL { chunks }
    }

    macro_rules! seed_statement {
        ($name:ident, $owned:ty, $feature:literal) => {
            #[cfg(feature = $feature)]
            #[derive(Debug, Clone)]
            pub struct $name {
                pub(crate) inner: OwnedSQL<$owned>,
            }

            #[cfg(feature = $feature)]
            impl $name {
                /// Render the INSERT statement as a SQL string.
                pub fn sql(&self) -> String {
                    self.inner.to_sql().build().0
                }

                /// Render the INSERT statement as a SQL string with bound parameters.
                pub fn build(&self) -> (String, Vec<$owned>) {
                    let sql = self.inner.to_sql();
                    let (text, params) = sql.build();
                    (text, params.into_iter().cloned().collect())
                }
            }

            #[cfg(feature = $feature)]
            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str(&self.sql())
                }
            }
        };
    }

    seed_statement!(SQLiteSeedStatement, OwnedSQLiteValue, "sqlite");
    seed_statement!(PostgresSeedStatement, OwnedPostgresValue, "postgres");

    #[cfg(feature = "sqlite")]
    impl<'a> ToSQL<'a, SQLiteValue<'a>> for SQLiteSeedStatement {
        fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
            convert_to_sql(&self.inner)
        }

        fn into_sql(self) -> SQL<'a, SQLiteValue<'a>> {
            convert_into_sql(self.inner)
        }
    }

    #[cfg(feature = "postgres")]
    impl<'a> ToSQL<'a, PostgresValue<'a>> for PostgresSeedStatement {
        fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
            convert_to_sql(&self.inner)
        }

        fn into_sql(self) -> SQL<'a, PostgresValue<'a>> {
            convert_into_sql(self.inner)
        }
    }
}

// ---------------------------------------------------------------------------
// Internal: generated data awaiting SQL rendering
// ---------------------------------------------------------------------------

struct GeneratedChunk<'a> {
    table: &'a dyn SQLTableInfo,
    rows: Vec<Vec<SeedValue>>,
}

#[derive(Clone)]
struct RelationSpec {
    target_table: String,
    fk_columns: Vec<String>,
    ref_columns: Vec<String>,
    children_per_parent: usize,
}

// ---------------------------------------------------------------------------
// Seeder (fully internal — public API is SeedConfig::generate)
// ---------------------------------------------------------------------------

struct Seeder<'a, D, S> {
    config: &'a SeedConfig<'a, D, S>,
}

impl<'a, D, S> Seeder<'a, D, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    fn new(config: &'a SeedConfig<'a, D, S>) -> Self {
        Self { config }
    }

    fn generate_chunks(&self, dialect_max_params: usize) -> Vec<GeneratedChunk<'a>> {
        let active_tables = self.config.active_tables();
        let order = topology::seeding_order(&active_tables);
        let table_map: HashMap<&str, &dyn SQLTableInfo> =
            active_tables.iter().map(|t| (t.name(), *t)).collect();

        let mut generated_values: HashMap<String, Vec<SeedValue>> = HashMap::new();
        let mut generated_counts: HashMap<String, usize> = HashMap::new();
        let mut chunks_out = Vec::new();

        for table_name in &order {
            let Some(&table) = table_map.get(table_name.as_str()) else {
                continue;
            };

            let columns = table.columns();
            if columns.is_empty() {
                continue;
            }

            let count = self.derived_count_for(table, &generated_counts);
            if count == 0 {
                generated_counts.insert(table_name.clone(), 0);
                continue;
            }

            let generators = self.build_generators(table);
            let col_index_map: HashMap<&str, usize> = columns
                .iter()
                .enumerate()
                .map(|(idx, col)| (col.name(), idx))
                .collect();
            let relation_specs = self.relation_specs_for(table);

            let mut all_rows: Vec<Vec<SeedValue>> = Vec::with_capacity(count);
            let mut col_rngs: Vec<StdRng> = columns
                .iter()
                .map(|c| rng::column_rng(table_name, c.name(), self.config.seed))
                .collect();

            for row_idx in 0..count {
                let mut row = Vec::with_capacity(columns.len());
                for (col_idx, generator) in generators.iter().enumerate() {
                    let val = generator.generate(
                        &mut col_rngs[col_idx],
                        row_idx,
                        columns[col_idx].r#type(),
                    );
                    row.push(val);
                }

                self.apply_many_to_one_relations(
                    &mut row,
                    &col_index_map,
                    &relation_specs,
                    row_idx,
                    &generated_values,
                );

                all_rows.push(row);
            }

            // Store generated values for all columns for FK/composite resolution
            for (col_idx, col) in columns.iter().enumerate() {
                let vals: Vec<SeedValue> =
                    all_rows.iter().map(|row| row[col_idx].clone()).collect();
                generated_values.insert(format!("{}.{}", table_name, col.name()), vals);
            }

            generated_counts.insert(table_name.clone(), count);

            let param_limit = self
                .config
                .max_params_per_batch
                .unwrap_or(dialect_max_params)
                .max(1);

            for (start, end) in batch_ranges_by_param_limit(&all_rows, param_limit) {
                chunks_out.push(GeneratedChunk {
                    table,
                    rows: all_rows[start..end].to_vec(),
                });
            }
        }

        chunks_out
    }

    fn derived_count_for(
        &self,
        table: &dyn SQLTableInfo,
        generated_counts: &HashMap<String, usize>,
    ) -> usize {
        if let Some(&count) = self.config.table_counts.get(table.name()) {
            return count;
        }

        let mut derived: Option<usize> = None;
        for parent_name in Self::parent_table_names(table) {
            if let Some(&parent_count) = generated_counts.get(parent_name) {
                let children_per_parent = self
                    .config
                    .relation_counts
                    .get(&(parent_name.to_string(), table.name().to_string()))
                    .copied()
                    .unwrap_or(1);
                let child_count = parent_count.saturating_mul(children_per_parent);
                derived = Some(derived.map_or(child_count, |current| current.max(child_count)));
            }
        }

        derived.unwrap_or_else(|| self.config.count_for(table.name()))
    }

    fn parent_table_names(table: &dyn SQLTableInfo) -> Vec<&str> {
        let mut seen = HashSet::new();
        let mut parent_names = Vec::new();

        for fk in table.foreign_keys() {
            let parent = fk.target_table().name();
            if parent != table.name() && seen.insert(parent) {
                parent_names.push(parent);
            }
        }

        parent_names
    }

    fn build_generators(&self, table: &dyn SQLTableInfo) -> Vec<Box<dyn Generator>> {
        let table_name = table.name();
        table
            .columns()
            .iter()
            .map(|col| {
                let col_name = col.name();
                let key = (table_name.to_string(), col_name.to_string());

                if let Some(custom) = self.config.column_generators.get(&key) {
                    return Box::new(Arc::clone(custom)) as Box<dyn Generator>;
                }

                if let Some(&kind) = self.config.column_kinds.get(&key) {
                    return kind.into_generator();
                }

                if col.has_default() && !col.is_primary_key() {
                    return Box::new(DefaultGen);
                }

                inference::infer_generator(*col).into_generator()
            })
            .collect()
    }

    fn relation_specs_for(&self, source_table: &dyn SQLTableInfo) -> Vec<RelationSpec> {
        source_table
            .foreign_keys()
            .iter()
            .map(|fk| {
                let target = fk.target_table().name().to_string();
                let source = source_table.name().to_string();
                let children_per_parent = self
                    .config
                    .relation_counts
                    .get(&(target.clone(), source))
                    .copied()
                    .unwrap_or(1);

                RelationSpec {
                    target_table: target,
                    fk_columns: fk
                        .source_columns()
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect(),
                    ref_columns: fk
                        .target_columns()
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect(),
                    children_per_parent,
                }
            })
            .collect()
    }

    fn apply_many_to_one_relations(
        &self,
        row: &mut [SeedValue],
        col_index_map: &HashMap<&str, usize>,
        relation_specs: &[RelationSpec],
        row_idx: usize,
        generated_values: &HashMap<String, Vec<SeedValue>>,
    ) {
        for rel in relation_specs {
            if rel.fk_columns.len() != rel.ref_columns.len() {
                continue;
            }

            let parent_count = rel
                .ref_columns
                .first()
                .and_then(|first_ref| {
                    generated_values
                        .get(&format!("{}.{}", rel.target_table, first_ref))
                        .map(|vals| vals.len())
                })
                .unwrap_or(0);

            if parent_count == 0 || rel.children_per_parent == 0 {
                for fk_col in &rel.fk_columns {
                    if let Some(&fk_idx) = col_index_map.get(fk_col.as_str()) {
                        row[fk_idx] = SeedValue::Null;
                    }
                }
                continue;
            }

            let parent_idx = (row_idx / rel.children_per_parent) % parent_count;
            for (fk_col, ref_col) in rel.fk_columns.iter().zip(rel.ref_columns.iter()) {
                let Some(&fk_idx) = col_index_map.get(fk_col.as_str()) else {
                    continue;
                };

                let key = format!("{}.{}", rel.target_table, ref_col);
                if let Some(parent_vals) = generated_values.get(&key) {
                    if let Some(parent_value) = parent_vals.get(parent_idx) {
                        row[fk_idx] = parent_value.clone();
                    } else {
                        row[fk_idx] = SeedValue::Null;
                    }
                } else {
                    row[fk_idx] = SeedValue::Null;
                }
            }
        }
    }
}

#[cfg(feature = "sqlite")]
impl<S> Seeder<'_, Sqlite, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    fn generate_sqlite(&self) -> Vec<SQLiteSeedStatement> {
        self.generate_chunks(batch::SQLITE_MAX_PARAMS)
            .iter()
            .map(|chunk| build_sqlite_statement(chunk))
            .collect()
    }
}

#[cfg(feature = "postgres")]
impl<S> Seeder<'_, Postgres, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    fn generate_postgres(&self) -> Vec<PostgresSeedStatement> {
        self.generate_chunks(batch::POSTGRES_MAX_PARAMS)
            .iter()
            .map(|chunk| build_postgres_statement(chunk))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Batching helpers
// ---------------------------------------------------------------------------

fn row_param_count(row: &[SeedValue]) -> usize {
    row.iter()
        .filter(|v| !matches!(v, SeedValue::Default | SeedValue::CurrentTime))
        .count()
}

fn batch_ranges_by_param_limit(rows: &[Vec<SeedValue>], param_limit: usize) -> Vec<(usize, usize)> {
    if rows.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut current_params = 0usize;

    for (idx, row) in rows.iter().enumerate() {
        let row_params = row_param_count(row);
        if idx > start && current_params.saturating_add(row_params) > param_limit {
            ranges.push((start, idx));
            start = idx;
            current_params = 0;
        }

        current_params = current_params.saturating_add(row_params);

        if start == idx && row_params > param_limit {
            ranges.push((idx, idx + 1));
            start = idx + 1;
            current_params = 0;
        }
    }

    if start < rows.len() {
        ranges.push((start, rows.len()));
    }

    ranges
}

// ---------------------------------------------------------------------------
// Per-dialect rendering: SeedValue → SQL fragments, assembled via core's SQL
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgres"))]
fn build_insert_sql<V>(table: &dyn SQLTableInfo, rows: &[Vec<SQL<'static, V>>]) -> OwnedSQL<V>
where
    V: drizzle_core::SQLParam + Clone + ToOwned<Owned = V> + 'static,
{
    let columns = table.columns();

    let column_idents = SQL::join(
        columns
            .iter()
            .map(|c| SQL::<'static, V>::ident(c.name().to_string())),
        Token::COMMA,
    );

    let sql = SQL::<'static, V>::token(Token::INSERT)
        .push(Token::INTO)
        .append(SQL::<'static, V>::ident(table.name().to_string()))
        .append(column_idents.parens())
        .push(Token::VALUES);

    let mut values_sql = SQL::<'static, V>::empty();
    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx > 0 {
            values_sql = values_sql.push(Token::COMMA);
        }
        let row_sql = SQL::join(row.iter().cloned(), Token::COMMA);
        values_sql = values_sql.append(row_sql.parens());
    }

    sql.append(values_sql).into_owned()
}

#[cfg(feature = "sqlite")]
fn seed_value_to_sqlite_sql(value: &SeedValue) -> SQL<'static, OwnedSQLiteValue> {
    match value {
        SeedValue::Default => SQL::token(Token::DEFAULT),
        SeedValue::Null => SQL::param(Cow::Owned(OwnedSQLiteValue::Null)),
        SeedValue::Integer(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Integer(*v))),
        SeedValue::Float(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Real(*v))),
        SeedValue::Text(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Text(v.clone()))),
        SeedValue::Bool(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Integer(if *v {
            1
        } else {
            0
        }))),
        SeedValue::Blob(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Blob(
            v.clone().into_boxed_slice(),
        ))),
        SeedValue::CurrentTime => SQL::raw("CURRENT_TIMESTAMP"),
    }
}

#[cfg(feature = "sqlite")]
fn build_sqlite_statement(chunk: &GeneratedChunk<'_>) -> SQLiteSeedStatement {
    let rows: Vec<Vec<SQL<'static, OwnedSQLiteValue>>> = chunk
        .rows
        .iter()
        .map(|row| row.iter().map(seed_value_to_sqlite_sql).collect())
        .collect();

    SQLiteSeedStatement {
        inner: build_insert_sql(chunk.table, &rows),
    }
}

#[cfg(feature = "postgres")]
fn seed_value_to_postgres_sql(
    value: &SeedValue,
    col: &dyn drizzle_core::SQLColumnInfo,
) -> SQL<'static, OwnedPostgresValue> {
    match value {
        SeedValue::Default => SQL::token(Token::DEFAULT),
        SeedValue::Null => SQL::param(Cow::Owned(OwnedPostgresValue::Null)),
        SeedValue::Integer(v) => {
            let ty = normalize_pg_type(col.r#type());
            let owned = if ty.contains("SMALLINT") {
                OwnedPostgresValue::Smallint((*v).clamp(i16::MIN as i64, i16::MAX as i64) as i16)
            } else if ty.contains("INT") || ty.contains("SERIAL") {
                OwnedPostgresValue::Integer((*v).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
            } else {
                OwnedPostgresValue::Bigint(*v)
            };
            SQL::param(Cow::Owned(owned))
        }
        SeedValue::Float(v) => SQL::param(Cow::Owned(OwnedPostgresValue::DoublePrecision(*v))),
        SeedValue::Text(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Text(v.clone()))),
        SeedValue::Bool(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Boolean(*v))),
        SeedValue::Blob(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Bytea(v.clone()))),
        SeedValue::CurrentTime => SQL::raw("now()"),
    }
}

#[cfg(feature = "postgres")]
fn normalize_pg_type(sql_type: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = false;
    for ch in sql_type.trim().chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(ch.to_ascii_uppercase());
            last_was_space = false;
        }
    }
    out
}

#[cfg(feature = "postgres")]
fn build_postgres_statement(chunk: &GeneratedChunk<'_>) -> PostgresSeedStatement {
    let columns = chunk.table.columns();
    let rows: Vec<Vec<SQL<'static, OwnedPostgresValue>>> = chunk
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(idx, value)| seed_value_to_postgres_sql(value, columns[idx]))
                .collect()
        })
        .collect();

    PostgresSeedStatement {
        inner: build_insert_sql(chunk.table, &rows),
    }
}

// ---------------------------------------------------------------------------
// Internal generator types
// ---------------------------------------------------------------------------

#[cfg(test)]
struct FkGen {
    parent_values: Vec<SeedValue>,
    children_per_parent: usize,
}

#[cfg(test)]
impl Generator for FkGen {
    fn generate(
        &self,
        _rng: &mut dyn generator::RngCore,
        index: usize,
        _sql_type: &str,
    ) -> SeedValue {
        if self.parent_values.is_empty() || self.children_per_parent == 0 {
            return SeedValue::Null;
        }
        let idx = (index / self.children_per_parent) % self.parent_values.len();
        self.parent_values[idx].clone()
    }
    fn name(&self) -> &'static str {
        "ForeignKey"
    }
}

struct DefaultGen;

impl Generator for DefaultGen {
    fn generate(
        &self,
        _rng: &mut dyn generator::RngCore,
        _index: usize,
        _sql_type: &str,
    ) -> SeedValue {
        SeedValue::Default
    }
    fn name(&self) -> &'static str {
        "Default"
    }
}

impl<C> Generator for &'static C
where
    C: SQLColumnInfo,
{
    fn generate(
        &self,
        rng: &mut dyn generator::RngCore,
        index: usize,
        sql_type: &str,
    ) -> SeedValue {
        inference::infer_generator(*self)
            .into_generator()
            .generate(rng, index, sql_type)
    }

    fn name(&self) -> &'static str {
        "Column"
    }
}

impl Generator for Arc<dyn Generator> {
    fn generate(
        &self,
        rng: &mut dyn generator::RngCore,
        index: usize,
        sql_type: &str,
    ) -> SeedValue {
        (**self).generate(rng, index, sql_type)
    }
    fn name(&self) -> &'static str {
        (**self).name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_generator_delegation() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g: Arc<dyn Generator> = Arc::new(generator::numeric::IntPrimaryKeyGen);
        let mut rng = StdRng::seed_from_u64(42);

        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Integer(1));
        assert_eq!(g.generate(&mut rng, 4, "INTEGER"), SeedValue::Integer(5));
        assert_eq!(g.name(), "IntPrimaryKey");
    }

    #[test]
    fn fk_gen_picks_from_parent_values() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let parent_vals = vec![
            SeedValue::Integer(10),
            SeedValue::Integer(20),
            SeedValue::Integer(30),
        ];
        let g = FkGen {
            parent_values: parent_vals.clone(),
            children_per_parent: 1,
        };
        let mut rng = StdRng::seed_from_u64(42);

        for i in 0..6 {
            let val = g.generate(&mut rng, i, "INTEGER");
            assert!(
                parent_vals.contains(&val),
                "FK value {:?} not in parent set",
                val
            );
        }
    }

    #[test]
    fn fk_gen_empty_parent_returns_null() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![],
            children_per_parent: 1,
        };
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Null);
    }

    #[test]
    fn default_gen_returns_default_keyword() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = DefaultGen;
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0, "TEXT"), SeedValue::Default);
    }

    #[test]
    fn fk_gen_with_relation_count_is_deterministic() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![SeedValue::Integer(1), SeedValue::Integer(2)],
            children_per_parent: 3,
        };
        let mut rng = StdRng::seed_from_u64(42);

        let generated: Vec<SeedValue> =
            (0..6).map(|i| g.generate(&mut rng, i, "INTEGER")).collect();
        assert_eq!(
            generated,
            vec![
                SeedValue::Integer(1),
                SeedValue::Integer(1),
                SeedValue::Integer(1),
                SeedValue::Integer(2),
                SeedValue::Integer(2),
                SeedValue::Integer(2),
            ]
        );
    }

    #[test]
    fn batch_ranges_split_on_param_limit() {
        let rows = vec![
            vec![SeedValue::Integer(1), SeedValue::Text("a".to_string())],
            vec![SeedValue::Integer(2), SeedValue::Text("b".to_string())],
            vec![SeedValue::Integer(3), SeedValue::Text("c".to_string())],
            vec![SeedValue::Integer(4), SeedValue::Text("d".to_string())],
            vec![SeedValue::Integer(5), SeedValue::Text("e".to_string())],
        ];

        let ranges = batch_ranges_by_param_limit(&rows, 4);
        assert_eq!(ranges, vec![(0, 2), (2, 4), (4, 5)]);
    }

    #[test]
    fn batch_ranges_counts_default_as_zero_params() {
        let rows = vec![
            vec![SeedValue::Default, SeedValue::Integer(1)],
            vec![SeedValue::Default, SeedValue::Integer(2)],
            vec![SeedValue::Default, SeedValue::Integer(3)],
        ];

        let ranges = batch_ranges_by_param_limit(&rows, 2);
        assert_eq!(ranges, vec![(0, 2), (2, 3)]);
    }

    #[test]
    fn batch_ranges_current_time_counts_as_zero_params() {
        let rows = vec![
            vec![SeedValue::Integer(1), SeedValue::CurrentTime],
            vec![SeedValue::Integer(2), SeedValue::CurrentTime],
            vec![SeedValue::Integer(3), SeedValue::CurrentTime],
        ];

        // Each row has 1 param (Integer). CurrentTime is raw SQL, not a param.
        // With limit 2, we should fit 2 rows per batch.
        let ranges = batch_ranges_by_param_limit(&rows, 2);
        assert_eq!(ranges, vec![(0, 2), (2, 3)]);
    }

    #[test]
    fn fk_gen_zero_children_per_parent_returns_null() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![SeedValue::Integer(1)],
            children_per_parent: 0,
        };
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Null);
    }
}
