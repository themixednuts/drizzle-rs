//! Shared macro for dialect CTE view types.

/// Generate dialect-specific `CTEDefinition` and `CTEView` types.
#[macro_export]
macro_rules! impl_cte_types {
    (value_type: $ValueType:ty $(,)?) => {
        /// Trait for types that can provide a CTE definition for WITH clauses.
        pub trait CTEDefinition<'a> {
            /// Returns the SQL for the CTE definition (e.g., `cte_name AS (SELECT ...)`).
            fn cte_definition(&self) -> $crate::SQL<'a, $ValueType>;
        }

        /// A CTE (Common Table Expression) view with typed table projection.
        #[derive(Clone, Debug)]
        pub struct CTEView<'a, Table, Query> {
            /// The aliased table for typed field access.
            pub table: Table,
            /// The CTE name.
            name: &'static str,
            /// The defining query.
            query: Query,
            _phantom: ::core::marker::PhantomData<$ValueType>,
        }

        impl<'a, Table, Query> CTEView<'a, Table, Query>
        where
            Query: $crate::ToSQL<'a, $ValueType>,
        {
            /// Creates a new `CTEView`.
            pub fn new(table: Table, name: &'static str, query: Query) -> Self {
                Self {
                    table,
                    name,
                    query,
                    _phantom: ::core::marker::PhantomData,
                }
            }

            /// Returns the CTE name.
            pub fn cte_name(&self) -> &'static str {
                self.name
            }

            /// Returns a reference to the underlying query.
            pub fn query(&self) -> &Query {
                &self.query
            }
        }

        impl<'a, Table, Query> CTEDefinition<'a> for CTEView<'a, Table, Query>
        where
            Query: $crate::ToSQL<'a, $ValueType>,
        {
            fn cte_definition(&self) -> $crate::SQL<'a, $ValueType> {
                $crate::SQL::raw(self.name)
                    .push($crate::Token::AS)
                    .append(self.query.to_sql().parens())
            }
        }

        impl<'a, Table, Query> CTEDefinition<'a> for &CTEView<'a, Table, Query>
        where
            Query: $crate::ToSQL<'a, $ValueType>,
        {
            fn cte_definition(&self) -> $crate::SQL<'a, $ValueType> {
                $crate::SQL::raw(self.name)
                    .push($crate::Token::AS)
                    .append(self.query.to_sql().parens())
            }
        }

        impl<'a, Table, Query> ::core::ops::Deref for CTEView<'a, Table, Query> {
            type Target = Table;

            fn deref(&self) -> &Self::Target {
                &self.table
            }
        }

        impl<'a, Table, Query> $crate::ToSQL<'a, $ValueType> for CTEView<'a, Table, Query>
        where
            Query: $crate::ToSQL<'a, $ValueType>,
        {
            fn to_sql(&self) -> $crate::SQL<'a, $ValueType> {
                $crate::SQL::ident(self.name)
            }
        }
    };
}
