//! Shared `ON CONFLICT` builder machinery.

use core::marker::PhantomData;

use crate::expr::Expr;
use crate::prelude::Box;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::BooleanLike;

/// Converts a dialect conflict target into an `ON CONFLICT ...` SQL fragment.
pub trait ConflictTargetSql<'a, V: SQLParam> {
    fn into_target_sql(self, target_where: Option<SQL<'a, V>>) -> SQL<'a, V>;
}

/// Shared column-list conflict target: `ON CONFLICT (col1, col2)`.
#[derive(Debug, Clone)]
pub struct ConflictColumnsTarget<'a, V: SQLParam> {
    columns: SQL<'a, V>,
}

impl<'a, V: SQLParam> ConflictColumnsTarget<'a, V> {
    #[inline]
    #[must_use]
    pub fn new(columns: SQL<'a, V>) -> Self {
        Self { columns }
    }
}

impl<'a, V: SQLParam> ConflictTargetSql<'a, V> for ConflictColumnsTarget<'a, V> {
    fn into_target_sql(self, target_where: Option<SQL<'a, V>>) -> SQL<'a, V> {
        let mut target = SQL::from_iter([Token::ON, Token::CONFLICT, Token::LPAREN])
            .append(self.columns)
            .push(Token::RPAREN);
        if let Some(target_where) = target_where {
            target = target.push(Token::WHERE).append(target_where);
        }
        target
    }
}

/// PostgreSQL conflict target, including `ON CONSTRAINT`.
#[derive(Debug, Clone)]
pub enum PostgresConflictTarget<'a, V: SQLParam> {
    Columns(Box<ConflictColumnsTarget<'a, V>>),
    Constraint(&'static str),
}

impl<'a, V: SQLParam> PostgresConflictTarget<'a, V> {
    #[inline]
    #[must_use]
    pub fn columns(columns: SQL<'a, V>) -> Self {
        Self::Columns(Box::new(ConflictColumnsTarget::new(columns)))
    }

    #[inline]
    #[must_use]
    pub const fn constraint(name: &'static str) -> Self {
        Self::Constraint(name)
    }
}

impl<'a, V: SQLParam> ConflictTargetSql<'a, V> for PostgresConflictTarget<'a, V> {
    fn into_target_sql(self, target_where: Option<SQL<'a, V>>) -> SQL<'a, V> {
        match self {
            Self::Columns(columns) => (*columns).into_target_sql(target_where),
            Self::Constraint(name) => {
                SQL::from_iter([Token::ON, Token::CONFLICT, Token::ON, Token::CONSTRAINT])
                    .append(SQL::ident(name))
            }
        }
    }
}

/// Dialect adapter for producing the concrete insert builder type.
pub trait OnConflictOutput<'a, V: SQLParam, Schema, Table> {
    type OnConflictSet;
    type DoUpdateSet;

    fn on_conflict(sql: SQL<'a, V>) -> Self::OnConflictSet;
    fn do_update(sql: SQL<'a, V>) -> Self::DoUpdateSet;
}

/// Intermediate builder for typed `ON CONFLICT` clause construction.
#[derive(Debug, Clone)]
pub struct OnConflictBuilder<'a, V, Schema, Table, Target, Output>
where
    V: SQLParam,
{
    sql: SQL<'a, V>,
    target: Target,
    target_where: Option<SQL<'a, V>>,
    schema: PhantomData<Schema>,
    table: PhantomData<Table>,
    output: PhantomData<Output>,
}

impl<'a, V, Schema, Table, Target, Output> OnConflictBuilder<'a, V, Schema, Table, Target, Output>
where
    V: SQLParam,
    Target: ConflictTargetSql<'a, V>,
    Output: OnConflictOutput<'a, V, Schema, Table>,
{
    #[inline]
    #[must_use]
    pub fn new(sql: SQL<'a, V>, target: Target) -> Self {
        Self {
            sql,
            target,
            target_where: None,
            schema: PhantomData,
            table: PhantomData,
            output: PhantomData,
        }
    }

    /// Adds a WHERE clause to the conflict target for partial index matching.
    #[must_use]
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: Expr<'a, V>,
        E::SQLType: BooleanLike,
    {
        self.target_where = Some(condition.into_expr_sql());
        self
    }

    fn into_parts(self) -> (SQL<'a, V>, SQL<'a, V>) {
        (self.sql, self.target.into_target_sql(self.target_where))
    }

    /// Resolves the conflict by doing nothing.
    #[must_use]
    pub fn do_nothing(self) -> Output::OnConflictSet {
        let (sql, target) = self.into_parts();
        Output::on_conflict(sql.append(target.push(Token::DO).push(Token::NOTHING)))
    }

    /// Resolves the conflict by updating the existing row.
    pub fn do_update(self, set: impl ToSQL<'a, V>) -> Output::DoUpdateSet {
        let (sql, target) = self.into_parts();
        let conflict = target
            .push(Token::DO)
            .push(Token::UPDATE)
            .push(Token::SET)
            .append(set.into_sql());
        Output::do_update(sql.append(conflict))
    }
}
