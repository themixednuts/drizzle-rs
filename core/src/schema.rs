//! Schema metadata for database enums, and `ORDER BY` terms.
//!
//! [`asc`] and [`desc`] (re-exported at the crate root) build the [`Ordered`]
//! terms that `order_by` and window [`order_by`](crate::expr::WindowSpec::order_by)
//! take. [`SQLEnumInfo`] is implemented by `#[PostgresEnum]`.

use crate::prelude::*;
use crate::{ToSQL, sql::SQL, traits::SQLParam};
use core::any::Any;

#[cfg(feature = "std")]
use std::collections::BTreeSet;

/// Schema metadata for a database enum type, such as a PostgreSQL
/// `CREATE TYPE ... AS ENUM`.
///
/// Implemented by `#[PostgresEnum]`.
pub trait SQLEnumInfo: Any + Send + Sync {
    /// The type name in the database.
    fn name(&self) -> &'static str;

    /// The `CREATE TYPE` statement for this enum.
    fn create_type_sql(&self) -> String;

    /// Every value of the enum, in declaration order.
    fn variants(&self) -> &'static [&'static str];
}

impl core::fmt::Debug for dyn SQLEnumInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLEnumInfo")
            .field("name", &self.name())
            .field("variants", &self.variants())
            .finish()
    }
}

/// Sort direction of an `ORDER BY` term.
///
/// Usually written through [`asc`] and [`desc`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    /// Ascending order (`ASC`): smallest first.
    Asc,
    /// Descending order (`DESC`): largest first.
    Desc,
}

/// One `ORDER BY` term, such as `"users"."age" DESC`; created by [`asc`] or
/// [`desc`].
///
/// `S` records the tables the term reads, for the query's scope check. To
/// order by several columns, pass a tuple of terms:
/// `order_by((asc(a), desc(b)))`. An array or `Vec` also works when every
/// term has the same type.
#[derive(Debug, Clone)]
pub struct Ordered<'a, V: SQLParam, S> {
    sql: SQL<'a, V>,
    sources: core::marker::PhantomData<fn() -> S>,
}

impl<'a, V: SQLParam, S> Ordered<'a, V, S> {
    /// Forgets which tables this term reads (see
    /// [`SQLExpr::unscoped`](crate::expr::SQLExpr::unscoped)).
    #[must_use]
    pub fn unscoped(self) -> Ordered<'a, V, ()> {
        Ordered {
            sql: self.sql,
            sources: core::marker::PhantomData,
        }
    }
}

impl<'a, V: SQLParam, S> ToSQL<'a, V> for Ordered<'a, V, S> {
    fn to_sql(&self) -> SQL<'a, V> {
        self.sql.clone()
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.sql
    }
}

impl<V: SQLParam, S> crate::expr::ExprSources for Ordered<'_, V, S> {
    type Sources = S;
}

/// A value that can be one `ORDER BY` term: an [`Ordered`] or raw [`SQL`].
///
/// Arrays and `Vec`s of one term type read that type's sources.
pub trait OrderTerm: crate::expr::ExprSources {}

impl<V: SQLParam, S> OrderTerm for Ordered<'_, V, S> {}

impl<V: SQLParam> OrderTerm for SQL<'_, V> {}

impl<T: OrderTerm, const N: usize> crate::expr::ExprSources for [T; N] {
    type Sources = T::Sources;
}

impl<T: OrderTerm> crate::expr::ExprSources for Vec<T> {
    type Sources = T::Sources;
}

/// Sorts by an expression in ascending order (`expr ASC`).
///
/// Accepts a column or any other expression and returns an [`Ordered`] term
/// for `order_by`.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// use drizzle_core::{asc, desc};
/// # use drizzle_core::ToSQL;
///
/// assert_eq!(asc(users.age).to_sql().sql(), r#""users"."age" ASC"#);
///
/// // Several terms go in a tuple.
/// let terms = (desc(users.score), asc(users.name));
/// assert_eq!(terms.to_sql().sql(), r#""users"."score" DESC, "users"."name" ASC"#);
/// ```
pub fn asc<'a, V, T>(column: T) -> Ordered<'a, V, T::Sources>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V> + crate::expr::ExprSources,
{
    Ordered {
        sql: column.to_sql().append(&OrderBy::Asc),
        sources: core::marker::PhantomData,
    }
}

/// Sorts by an expression in descending order (`expr DESC`).
///
/// Accepts a column or any other expression and returns an [`Ordered`] term
/// for `order_by`.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// use drizzle_core::{asc, desc};
/// # use drizzle_core::ToSQL;
///
/// assert_eq!(desc(users.age).to_sql().sql(), r#""users"."age" DESC"#);
///
/// // Several terms go in a tuple.
/// let terms = (desc(users.score), asc(users.name));
/// assert_eq!(terms.to_sql().sql(), r#""users"."score" DESC, "users"."name" ASC"#);
/// ```
pub fn desc<'a, V, T>(column: T) -> Ordered<'a, V, T::Sources>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V> + crate::expr::ExprSources,
{
    Ordered {
        sql: column.to_sql().append(&OrderBy::Desc),
        sources: core::marker::PhantomData,
    }
}

/// Topological sort of `(name, dependency_names)` pairs using Kahn's algorithm.
///
/// Returns names in dependency order (dependencies before dependents).
/// Uses `BTreeSet` for deterministic tie-breaking (lexicographic).
///
/// # Errors
///
/// Returns an error if a cycle is detected.
#[cfg(feature = "std")]
#[allow(dead_code)]
pub(crate) fn topological_order<'a>(
    items: impl IntoIterator<Item = (&'a str, &'a [&'a str])>,
) -> crate::error::Result<Vec<&'a str>> {
    let items: Vec<_> = items.into_iter().collect();
    let name_set: HashMap<&str, usize> = items
        .iter()
        .enumerate()
        .map(|(i, (name, _))| (*name, i))
        .collect();

    let n = items.len();
    let mut indegree = vec![0usize; n];
    let mut reverse_edges: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, (_name, deps)) in items.iter().enumerate() {
        for dep in *deps {
            if let Some(&j) = name_set.get(dep) {
                indegree[i] += 1;
                reverse_edges[j].push(i);
            }
        }
    }

    let mut queue: BTreeSet<(&str, usize)> = BTreeSet::new();
    for (i, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            queue.insert((items[i].0, i));
        }
    }

    let mut result = Vec::with_capacity(n);
    while let Some(&entry) = queue.first() {
        queue.remove(&entry);
        let (_name, idx) = entry;
        result.push(items[idx].0);

        for &neighbor in &reverse_edges[idx] {
            indegree[neighbor] -= 1;
            if indegree[neighbor] == 0 {
                queue.insert((items[neighbor].0, neighbor));
            }
        }
    }

    if result.len() != n {
        return Err(crate::error::DrizzleError::Schema(
            "cycle detected in table dependencies".into(),
        ));
    }

    Ok(result)
}

/// Renders `ASC` or `DESC`.
impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for OrderBy {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        };
        SQL::raw(sql_str)
    }
}
