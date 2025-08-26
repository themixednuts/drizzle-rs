use std::any::Any;
mod column;
mod param;
mod table;
mod tuple;

pub use column::*;
pub use param::*;
pub use table::*;

// Re-export enum traits from schema module
pub use crate::schema::{AsEnumInfo, SQLEnumInfo};

use crate::{SQL, ToSQL};

pub trait SQLSchema<'a, T, V: SQLParam + 'a>: ToSQL<'a, V> {
    const NAME: &'a str;
    const TYPE: T;
    const SQL: SQL<'a, V>;

    // Optional runtime SQL generation for tables with dynamic constraints
    fn sql(&self) -> SQL<'a, V> {
        Self::SQL
    }
}

pub trait SQLSchemaType: std::fmt::Debug + Any + Send + Sync {}

pub trait SQLSchemaImpl: Any + Send + Sync {
    fn create_statements(&self) -> Vec<String>;
}

/// Marker trait for types that can be compared in SQL expressions.
pub trait SQLComparable<'a, V: SQLParam, Rhs> {}

// ============================================================================
// SQLComparable Implementations
// ============================================================================
/// Column-to-Column comparison (most specific, type-safe)
/// Only allows comparisons between columns with the same underlying type
// impl<'a, V, L, R, T> SQLComparable<'a, V, R> for L
// where
//     V: SQLParam + 'a,
//     L: SQLColumn<'a, V, Type = T> + ToSQL<'a, V>,
//     R: SQLColumn<'a, V, Type = T> + ToSQL<'a, V>,
//     T: PartialEq, // Ensures the underlying types can be compared
// {
// }
/// Column-to-Value comparison (type-safe)
/// Only allows comparisons between a column and a value of the same type
// impl<'a, V, C, T> SQLComparable<'a, V, T> for C
// where
//     V: SQLParam + 'a,
//     C: SQLColumn<'a, V, Type = T> + ToSQL<'a, V>,
//     T: Into<V> + ToSQL<'a, V> + PartialEq,
// {
// }
/// Value-to-Value comparison (most general, permissive)
/// Allows any two values that can convert to SQL parameters
// impl<'a, V, L, R> SQLComparable<'a, V, R> for L
// where
//     V: SQLParam + 'a,
//     L: Into<V> + ToSQL<'a, V>,
//     R: Into<V> + ToSQL<'a, V>,
// {
// }
/// Blanket implementation for all compatible types
/// This covers all three cases:
/// 1. Column-to-Column (when both L and R are SQLColumn with same Type)
/// 2. Column-to-Value (when L is SQLColumn and R converts to same type)  
/// 3. Value-to-Value (when both convert to SQL values)
impl<'a, V, L, R> SQLComparable<'a, V, R> for L
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
}

/// Marker trait indicating that a table `T` is part of a schema represented by the marker type `S`.
pub trait IsInSchema<S> {}

pub trait SQLIndexInfo: Any + Send + Sync {
    fn table(&self) -> &dyn SQLTableInfo;
    /// The name of this index (for DROP INDEX statements)
    fn name(&self) -> &'static str;

    /// Whether this is a unique index
    fn is_unique(&self) -> bool {
        false
    }
}

pub trait AsIndexInfo: SQLIndexInfo {
    fn as_index(&self) -> &dyn SQLIndexInfo;
}

impl<T: SQLIndexInfo> AsIndexInfo for T {
    fn as_index(&self) -> &dyn SQLIndexInfo {
        self
    }
}

impl std::fmt::Debug for dyn SQLIndexInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLIndexInfo")
            .field("name", &self.name())
            .field("is_unique", &self.is_unique())
            .field("table", &self.table())
            .finish()
    }
}
/// Trait for types that represent database indexes.
/// Implemented by tuple structs like `struct UserEmailIdx(User::email);`
pub trait SQLIndex<'a, Type: SQLSchemaType, Value: SQLParam + 'a>:
    SQLIndexInfo + ToSQL<'a, Value>
{
    /// The table type this index is associated with
    type Table: SQLTable<'a, Type, Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_param_implementations() {
        // Test that common types implement SQLParam
        const fn assert_sql_param<T: SQLParam>(_: &T) {}

        assert_sql_param(&String::new());
        assert_sql_param(&"test");
        assert_sql_param(&42i32);
        assert_sql_param(&42i64);
        assert_sql_param(&true);
        assert_sql_param(&Some(42));
        assert_sql_param(&None::<i32>);
        assert_sql_param(&vec![1, 2, 3]);
    }

    #[test]
    fn test_option_sql_param() {
        fn accepts_sql_param<T: SQLParam>(_: T) {}

        accepts_sql_param(Some(42i32));
        accepts_sql_param(None::<String>);
        accepts_sql_param(Some("test".to_string()));
    }

    #[test]
    fn test_vec_sql_param() {
        fn accepts_sql_param<T: SQLParam>(_: T) {}

        accepts_sql_param(vec![1, 2, 3]);
        accepts_sql_param(vec!["a", "b"]);
        accepts_sql_param(Vec::<i32>::new());
    }
}
