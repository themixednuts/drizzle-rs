//! JOIN keywords and the macros dialect crates use to build join helpers.
//!
//! Users join tables with the builder methods (`.join(...)`,
//! `.left_join(...)`, ...). This module holds the shared pieces those
//! methods render with.

use crate::{SQL, ToSQL, traits::SQLParam};

// =============================================================================
// Join Type Enum
// =============================================================================

/// The kind of a JOIN.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JoinType {
    /// Plain `JOIN` (an inner join).
    #[default]
    Join,
    /// `INNER JOIN`.
    Inner,
    /// `LEFT JOIN`.
    Left,
    /// `RIGHT JOIN`.
    Right,
    /// `FULL JOIN`.
    Full,
    /// `CROSS JOIN`.
    Cross,
}

// =============================================================================
// Join Builder Struct
// =============================================================================

/// The JOIN keyword of a join clause, such as `NATURAL LEFT OUTER JOIN`.
///
/// Built with `const` methods, so it can be a constant. Renders through
/// [`ToSQL`].
///
/// # Examples
///
/// ```
/// use drizzle_core::{Join, SQL, ToSQL};
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// let sql: SQL<'_, Value> = Join::new().left().outer().to_sql();
/// assert_eq!(sql.sql(), "LEFT OUTER JOIN");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Join {
    /// Adds `NATURAL`.
    pub natural: bool,
    /// The kind of join.
    pub join_type: JoinType,
    /// Adds `OUTER`. Only used for `LEFT`, `RIGHT` and `FULL`.
    pub outer: bool,
}

impl Join {
    /// Creates a plain `JOIN`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            natural: false,
            join_type: JoinType::Join,
            outer: false,
        }
    }

    /// Makes this a `NATURAL` join.
    #[must_use]
    pub const fn natural(mut self) -> Self {
        self.natural = true;
        self
    }

    /// Makes this an `INNER` join.
    #[must_use]
    pub const fn inner(mut self) -> Self {
        self.join_type = JoinType::Inner;
        self
    }

    /// Makes this a `LEFT` join.
    #[must_use]
    pub const fn left(mut self) -> Self {
        self.join_type = JoinType::Left;
        self
    }

    /// Makes this a `RIGHT` join.
    #[must_use]
    pub const fn right(mut self) -> Self {
        self.join_type = JoinType::Right;
        self
    }

    /// Makes this a `FULL` join.
    #[must_use]
    pub const fn full(mut self) -> Self {
        self.join_type = JoinType::Full;
        self
    }

    /// Makes this a `CROSS` join.
    #[must_use]
    pub const fn cross(mut self) -> Self {
        self.join_type = JoinType::Cross;
        self
    }

    /// Adds `OUTER` (`LEFT OUTER`, `RIGHT OUTER`, `FULL OUTER`). Ignored for
    /// other kinds.
    #[must_use]
    pub const fn outer(mut self) -> Self {
        self.outer = true;
        self
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Join {
    fn to_sql(&self) -> SQL<'a, V> {
        // Use pre-computed static strings to avoid Vec allocation
        let join_str = match (self.natural, self.join_type, self.outer) {
            // NATURAL variants
            (true, JoinType::Join, _) => "NATURAL JOIN",
            (true, JoinType::Inner, _) => "NATURAL INNER JOIN",
            (true, JoinType::Left, false) => "NATURAL LEFT JOIN",
            (true, JoinType::Left, true) => "NATURAL LEFT OUTER JOIN",
            (true, JoinType::Right, false) => "NATURAL RIGHT JOIN",
            (true, JoinType::Right, true) => "NATURAL RIGHT OUTER JOIN",
            (true, JoinType::Full, false) => "NATURAL FULL JOIN",
            (true, JoinType::Full, true) => "NATURAL FULL OUTER JOIN",
            (true, JoinType::Cross, _) => "NATURAL CROSS JOIN",
            // Non-NATURAL variants
            (false, JoinType::Join, _) => "JOIN",
            (false, JoinType::Inner, _) => "INNER JOIN",
            (false, JoinType::Left, false) => "LEFT JOIN",
            (false, JoinType::Left, true) => "LEFT OUTER JOIN",
            (false, JoinType::Right, false) => "RIGHT JOIN",
            (false, JoinType::Right, true) => "RIGHT OUTER JOIN",
            (false, JoinType::Full, false) => "FULL JOIN",
            (false, JoinType::Full, true) => "FULL OUTER JOIN",
            (false, JoinType::Cross, _) => "CROSS JOIN",
        };
        SQL::raw(join_str)
    }
}

/// A `(derived table, condition)` pair accepted by `JOIN LATERAL`.
#[doc(hidden)]
pub trait LateralArg<'a, V: SQLParam>: lateral_private::Arg {
    /// The source added to the query scope.
    type JoinedTable;
    /// Sources read by the `ON` condition (see [`crate::scope`]).
    type OnSources;

    /// Renders `<join> LATERAL <source> ON <condition>`.
    fn into_lateral_sql(self, join: Join) -> SQL<'a, V>;
}

impl<'a, V, Name, Projection, Query, Condition> LateralArg<'a, V>
    for (crate::Derived<'a, V, Name, Projection, Query>, Condition)
where
    V: SQLParam + 'a,
    Name: crate::Tag,
    Projection: crate::DerivedProjection<Name>,
    Query: ToSQL<'a, V>,
    Condition: crate::expr::Expr<'a, V>,
    Condition::SQLType: crate::types::BooleanLike,
{
    type JoinedTable = crate::Derived<'a, V, Name, Projection, Query>;
    type OnSources = Condition::Sources;

    fn into_lateral_sql(self, join: Join) -> SQL<'a, V> {
        let (source, condition) = self;
        join.to_sql()
            .append(SQL::raw(" LATERAL "))
            .append(source.into_sql())
            .push(crate::Token::ON)
            .append(condition.into_sql())
    }
}

/// A derived table accepted by `CROSS JOIN LATERAL`.
#[doc(hidden)]
pub trait LateralSource<'a, V: SQLParam>: lateral_private::Source {
    /// The source added to the query scope.
    type JoinedTable;

    /// Renders `CROSS JOIN LATERAL <source>`.
    fn into_cross_lateral_sql(self) -> SQL<'a, V>;
}

impl<'a, V, Name, Projection, Query> LateralSource<'a, V>
    for crate::Derived<'a, V, Name, Projection, Query>
where
    V: SQLParam + 'a,
    Name: crate::Tag,
    Projection: crate::DerivedProjection<Name>,
    Query: ToSQL<'a, V>,
{
    type JoinedTable = Self;

    fn into_cross_lateral_sql(self) -> SQL<'a, V> {
        Join::new()
            .cross()
            .to_sql()
            .append(SQL::raw(" LATERAL "))
            .append(self.into_sql())
    }
}

mod lateral_private {
    pub trait Arg {}
    pub trait Source {}

    impl<V, Name, Projection, Query, Condition> Arg
        for (crate::Derived<'_, V, Name, Projection, Query>, Condition)
    where
        V: crate::SQLParam,
    {
    }

    impl<V, Name, Projection, Query> Source for crate::Derived<'_, V, Name, Projection, Query> where
        V: crate::SQLParam
    {
    }
}

// =============================================================================
// Join Helper Macro
// =============================================================================

/// Generates free functions that render join clauses (`join`, `left_join`,
/// `natural_full_outer_join`, ...) for one dialect.
///
/// Each function takes a table and, except for the `natural_*` ones, an ON
/// condition, and returns the rendered clause. Dialect crates invoke this
/// once with their table trait, condition trait and SQL type.
///
/// # Examples
///
/// ```
/// use drizzle_core::SQL;
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// mod joins {
///     use super::Value;
///     use drizzle_core::{SQL, ToSQL};
///
///     drizzle_core::impl_join_helpers!(
///         table_trait: ToSQL<'a, Value>,
///         condition_trait: ToSQL<'a, Value>,
///         sql_type: SQL<'a, Value>,
///     );
/// }
///
/// fn main() {
///     let clause = joins::left_join(SQL::<Value>::ident("posts"), SQL::<Value>::raw("TRUE"));
///     assert_eq!(clause.sql(), r#"LEFT JOIN "posts" ON TRUE"#);
/// }
/// ```
#[macro_export]
macro_rules! impl_join_helpers {
    (
        table_trait: $TableTrait:path,
        condition_trait: $ConditionTrait:path,
        sql_type: $SQLType:ty $(,)?
    ) => {
        fn join_internal<'a, Table>(
            table: Table,
            join: $crate::Join,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            join.to_sql()
                .append(&table)
                .push($crate::Token::ON)
                .append(&condition)
        }

        /// Renders `NATURAL JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new().natural().to_sql().append(&table)
        }

        /// Renders `JOIN table ON condition`.
        pub fn join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new(), condition)
        }

        /// Renders `NATURAL LEFT JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_left_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new().natural().left().to_sql().append(&table)
        }

        /// Renders `LEFT JOIN table ON condition`.
        pub fn left_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().left(), condition)
        }

        /// Renders `LEFT OUTER JOIN table ON condition`.
        pub fn left_outer_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().left().outer(), condition)
        }

        /// Renders `NATURAL LEFT OUTER JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_left_outer_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new()
                .natural()
                .left()
                .outer()
                .to_sql()
                .append(&table)
        }

        /// Renders `NATURAL RIGHT JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_right_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new()
                .natural()
                .right()
                .to_sql()
                .append(&table)
        }

        /// Renders `RIGHT JOIN table ON condition`.
        pub fn right_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().right(), condition)
        }

        /// Renders `RIGHT OUTER JOIN table ON condition`.
        pub fn right_outer_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().right().outer(), condition)
        }

        /// Renders `NATURAL RIGHT OUTER JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_right_outer_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new()
                .natural()
                .right()
                .outer()
                .to_sql()
                .append(&table)
        }

        /// Renders `NATURAL FULL JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_full_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new().natural().full().to_sql().append(&table)
        }

        /// Renders `FULL JOIN table ON condition`.
        pub fn full_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().full(), condition)
        }

        /// Renders `FULL OUTER JOIN table ON condition`.
        pub fn full_outer_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().full().outer(), condition)
        }

        /// Renders `NATURAL FULL OUTER JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_full_outer_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new()
                .natural()
                .full()
                .outer()
                .to_sql()
                .append(&table)
        }

        /// Renders `NATURAL INNER JOIN table`.
        ///
        /// A natural join matches the columns both sides share by name, so it
        /// takes no ON condition.
        pub fn natural_inner_join<'a, Table>(table: Table) -> $SQLType
        where
            Table: $TableTrait,
        {
            use $crate::ToSQL;
            $crate::Join::new()
                .natural()
                .inner()
                .to_sql()
                .append(&table)
        }

        /// Renders `INNER JOIN table ON condition`.
        pub fn inner_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().inner(), condition)
        }

        /// Compatibility helper for a conditional cross join.
        ///
        /// This renders the portable equivalent `INNER JOIN ... ON ...`.
        /// Use the dialect builder's bare `.cross_join(source)` for an
        /// unconditional `CROSS JOIN`.
        pub fn cross_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().inner(), condition)
        }
    };
}

/// Generates a dialect's `JoinArg` trait: what `.join(...)` and its variants
/// accept.
///
/// Two forms are accepted:
/// - `(source, condition)`: an explicit ON condition;
/// - a bare table: the ON condition comes from the foreign key between the
///   two tables ([`Joinable`](crate::Joinable)).
#[macro_export]
macro_rules! impl_join_arg_trait {
    (
        table_trait: $TableTrait:path,
        table_info_trait: $TableInfoTrait:path,
        condition_trait: $ConditionTrait:path,
        join_source_trait: $JoinSourceTrait:path,
        value_type: $ValueType:ty $(,)?
    ) => {
        /// Trait for arguments accepted by `.join()` and related join methods.
        pub trait JoinArg<'a, FromTable> {
            /// Table added to the query scope by this join.
            type JoinedTable;

            /// Sources read by the `ON` condition (see `drizzle_core::scope`).
            type OnSources;

            /// Renders the join source and its `ON` condition.
            fn into_join_sql(self, join: $crate::Join) -> $crate::SQL<'a, $ValueType>;
        }

        /// Bare table: the ON condition matches the foreign-key columns from
        /// `Joinable::fk_columns()`.
        impl<'a, U, T> JoinArg<'a, T> for U
        where
            U: $TableTrait + $crate::Joinable<T>,
            T: $TableInfoTrait + ::core::default::Default,
        {
            type JoinedTable = U;
            // The derived condition reads only the joined and current tables.
            type OnSources = ();

            fn into_join_sql(self, join: $crate::Join) -> $crate::SQL<'a, $ValueType> {
                use $crate::ToSQL;

                let from = T::default();
                let cols = <U as $crate::Joinable<T>>::fk_columns();
                let join_name = self.name();
                let from_name = from.name();

                let mut condition = $crate::SQL::with_capacity_chunks(cols.len() * 7);
                for (idx, (self_col, target_col)) in cols.iter().enumerate() {
                    if idx > 0 {
                        condition.push_mut($crate::Token::AND);
                    }
                    condition.append_mut(
                        $crate::SQL::ident(join_name)
                            .push($crate::Token::DOT)
                            .append($crate::SQL::ident(*self_col)),
                    );
                    condition.push_mut($crate::Token::EQ);
                    condition.append_mut(
                        $crate::SQL::ident(from_name)
                            .push($crate::Token::DOT)
                            .append($crate::SQL::ident(*target_col)),
                    );
                }

                join.to_sql()
                    .append(&self)
                    .push($crate::Token::ON)
                    .append(&condition)
            }
        }

        /// Tuple `(table, condition)`: explicit ON condition.
        impl<'a, U, C, T> JoinArg<'a, T> for (U, C)
        where
            U: $JoinSourceTrait,
            C: $ConditionTrait + $crate::expr::ExprSources,
        {
            type JoinedTable = U::JoinedTable;
            type OnSources = C::Sources;

            fn into_join_sql(self, join: $crate::Join) -> $crate::SQL<'a, $ValueType> {
                let (source, condition) = self;
                join.to_sql()
                    .append($crate::SQL::raw(" "))
                    .append(source.into_join_source_sql())
                    .push($crate::Token::ON)
                    .append(condition.into_sql())
            }
        }
    };
}
