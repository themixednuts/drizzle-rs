use crate::{sql::SQL, traits::SQLParam, ToSQL};

/// The type of SQLite database object
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum SQLSchemaType {
    /// A regular table
    Table,
    /// A view
    View,
    /// An index
    Index,
    /// A trigger
    Trigger,
}

/// Sort direction for ORDER BY clauses
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    Asc,
    Desc,
}

impl OrderBy {
    /// Creates an ascending ORDER BY clause: "column ASC"
    pub fn asc<'a, V, T>(column: T) -> SQL<'a, V>
    where
        V: SQLParam + 'a,
        T: ToSQL<'a, V>,
    {
        column.to_sql().append(Self::Asc.to_sql())
    }

    /// Creates a descending ORDER BY clause: "column DESC"
    pub fn desc<'a, V, T>(column: T) -> SQL<'a, V>
    where
        V: SQLParam + 'a,
        T: ToSQL<'a, V>,
    {
        column.to_sql().append(Self::Desc.to_sql())
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for OrderBy {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            OrderBy::Asc => "ASC",
            OrderBy::Desc => "DESC",
        };
        SQL::raw(sql_str)
    }
}