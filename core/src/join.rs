//! Join types and helper macros for SQL JOIN operations
//!
//! This module provides shared JOIN functionality that can be used by
//! dialect-specific implementations (SQLite, PostgreSQL, etc.)

use crate::{SQL, ToSQL, traits::SQLParam};

// =============================================================================
// Join Type Enum
// =============================================================================

/// The type of JOIN operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JoinType {
    #[default]
    Join,
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

// =============================================================================
// Join Builder Struct
// =============================================================================

/// Builder for constructing JOIN clauses
///
/// This struct uses a builder pattern with const fn methods to allow
/// compile-time construction of JOIN specifications.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Join {
    pub natural: bool,
    pub join_type: JoinType,
    pub outer: bool, // only meaningful for LEFT/RIGHT/FULL
}

impl Join {
    /// Creates a new Join with default settings (basic JOIN)
    pub const fn new() -> Self {
        Self {
            natural: false,
            join_type: JoinType::Join,
            outer: false,
        }
    }

    /// Makes this a NATURAL join
    pub const fn natural(mut self) -> Self {
        self.natural = true;
        self
    }

    /// Makes this an INNER join
    pub const fn inner(mut self) -> Self {
        self.join_type = JoinType::Inner;
        self
    }

    /// Makes this a LEFT join
    pub const fn left(mut self) -> Self {
        self.join_type = JoinType::Left;
        self
    }

    /// Makes this a RIGHT join
    pub const fn right(mut self) -> Self {
        self.join_type = JoinType::Right;
        self
    }

    /// Makes this a FULL join
    pub const fn full(mut self) -> Self {
        self.join_type = JoinType::Full;
        self
    }

    /// Makes this a CROSS join
    pub const fn cross(mut self) -> Self {
        self.join_type = JoinType::Cross;
        self
    }

    /// Makes this an OUTER join (LEFT OUTER, RIGHT OUTER, FULL OUTER)
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

// =============================================================================
// Join Helper Macro
// =============================================================================

/// Macro to generate join helper functions for a specific dialect.
///
/// This macro generates all the standard join helper functions (natural_join,
/// left_join, etc.) that create SQL JOIN clauses. Each dialect invokes this
/// macro with their specific table trait and SQL type.
///
/// # Usage
/// ```ignore
/// impl_join_helpers!(
///     /// Trait bound for table types
///     table_trait: SQLiteTable<'a>,
///     /// Trait bound for condition types
///     condition_trait: ToSQL<'a, SQLiteValue<'a>>,
///     /// Return type for SQL
///     sql_type: SQL<'a, SQLiteValue<'a>>,
/// );
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

        /// Helper function to create a NATURAL JOIN clause
        pub fn natural_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().natural(), condition)
        }

        /// Helper function to create a JOIN clause
        pub fn join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new(), condition)
        }

        /// Helper function to create a NATURAL LEFT JOIN clause
        pub fn natural_left_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().natural().left(), condition)
        }

        /// Helper function to create a LEFT JOIN clause
        pub fn left_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().left(), condition)
        }

        /// Helper function to create a LEFT OUTER JOIN clause
        pub fn left_outer_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().left().outer(), condition)
        }

        /// Helper function to create a NATURAL LEFT OUTER JOIN clause
        pub fn natural_left_outer_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(
                table,
                $crate::Join::new().natural().left().outer(),
                condition,
            )
        }

        /// Helper function to create a NATURAL RIGHT JOIN clause
        pub fn natural_right_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().natural().right(), condition)
        }

        /// Helper function to create a RIGHT JOIN clause
        pub fn right_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().right(), condition)
        }

        /// Helper function to create a RIGHT OUTER JOIN clause
        pub fn right_outer_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().right().outer(), condition)
        }

        /// Helper function to create a NATURAL RIGHT OUTER JOIN clause
        pub fn natural_right_outer_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(
                table,
                $crate::Join::new().natural().right().outer(),
                condition,
            )
        }

        /// Helper function to create a NATURAL FULL JOIN clause
        pub fn natural_full_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().natural().full(), condition)
        }

        /// Helper function to create a FULL JOIN clause
        pub fn full_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().full(), condition)
        }

        /// Helper function to create a FULL OUTER JOIN clause
        pub fn full_outer_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().full().outer(), condition)
        }

        /// Helper function to create a NATURAL FULL OUTER JOIN clause
        pub fn natural_full_outer_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(
                table,
                $crate::Join::new().natural().full().outer(),
                condition,
            )
        }

        /// Helper function to create a NATURAL INNER JOIN clause
        pub fn natural_inner_join<'a, Table>(
            table: Table,
            condition: impl $ConditionTrait,
        ) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().natural().inner(), condition)
        }

        /// Helper function to create an INNER JOIN clause
        pub fn inner_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().inner(), condition)
        }

        /// Helper function to create a CROSS JOIN clause
        pub fn cross_join<'a, Table>(table: Table, condition: impl $ConditionTrait) -> $SQLType
        where
            Table: $TableTrait,
        {
            join_internal(table, $crate::Join::new().cross(), condition)
        }
    };
}

/// Macro to generate dialect-specific `JoinArg` trait and impls.
///
/// This consolidates the shared logic for:
/// - explicit join tuples: `(table, condition)`
/// - auto-FK joins for bare tables
#[macro_export]
macro_rules! impl_join_arg_trait {
    (
        table_trait: $TableTrait:path,
        table_info_trait: $TableInfoTrait:path,
        condition_trait: $ConditionTrait:path,
        value_type: $ValueType:ty $(,)?
    ) => {
        /// Trait for arguments accepted by `.join()` and related join methods.
        pub trait JoinArg<'a, FromTable> {
            type JoinedTable;
            fn into_join_sql(self, join: $crate::Join) -> $crate::SQL<'a, $ValueType>;
        }

        /// Bare table: derives the ON condition from `Joinable::fk_columns()`.
        impl<'a, U, T> JoinArg<'a, T> for U
        where
            U: $TableTrait + $crate::Joinable<T>,
            T: $TableInfoTrait + ::core::default::Default,
        {
            type JoinedTable = U;

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
                        $crate::SQL::ident(join_name.to_string())
                            .push($crate::Token::DOT)
                            .append($crate::SQL::ident(self_col.to_string())),
                    );
                    condition.push_mut($crate::Token::EQ);
                    condition.append_mut(
                        $crate::SQL::ident(from_name.to_string())
                            .push($crate::Token::DOT)
                            .append($crate::SQL::ident(target_col.to_string())),
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
            U: $TableTrait,
            C: $ConditionTrait,
        {
            type JoinedTable = U;

            fn into_join_sql(self, join: $crate::Join) -> $crate::SQL<'a, $ValueType> {
                let (table, condition) = self;
                join_internal(table, join, condition)
            }
        }
    };
}
