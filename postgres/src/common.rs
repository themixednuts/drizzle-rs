use drizzle_core::{SQL, SQLEnumInfo, SQLIndexInfo, SQLSchemaType, ToSQL, traits::SQLParam};

use crate::traits::PostgresTableInfo;

/// The type of database object
#[derive(Debug, Clone)]
pub enum PostgresSchemaType {
    /// A regular table
    Table(&'static dyn PostgresTableInfo),
    /// A view
    View,
    /// An index
    Index(&'static dyn SQLIndexInfo),
    /// A trigger
    Trigger,
    /// A database enum type (PostgreSQL)
    Enum(&'static dyn SQLEnumInfo),
}

impl SQLSchemaType for PostgresSchemaType {}

//------------------------------------------------------------------------------
// Number Type
//------------------------------------------------------------------------------

/// Numeric type that can be either an integer or a floating point value
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Number {
    /// Integer value
    Integer(i64),
    /// Floating point value
    Real(f64),
}

impl Default for Number {
    fn default() -> Self {
        Self::Integer(Default::default())
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

// Note: Generic From implementation is removed to avoid conflicts.
// The table macro will generate specific implementations using PostgresEnumVisitor.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Join {
    pub natural: bool,
    pub join_type: JoinType,
    pub outer: bool, // only meaningful for LEFT/RIGHT/FULL
}

impl Join {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn natural(mut self) -> Self {
        self.natural = true;
        self
    }

    pub fn inner(mut self) -> Self {
        self.join_type = JoinType::Inner;
        self
    }

    pub fn left(mut self) -> Self {
        self.join_type = JoinType::Left;
        self
    }

    pub fn right(mut self) -> Self {
        self.join_type = JoinType::Right;
        self
    }

    pub fn full(mut self) -> Self {
        self.join_type = JoinType::Full;
        self
    }

    pub fn cross(mut self) -> Self {
        self.join_type = JoinType::Cross;
        self
    }

    pub fn outer(mut self) -> Self {
        self.outer = true;
        self
    }
}
impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Join {
    fn to_sql(&self) -> SQL<'a, V> {
        let mut parts = Vec::new();

        if self.natural {
            parts.push("NATURAL");
        }

        match self.join_type {
            JoinType::Inner => parts.push("INNER"),
            JoinType::Left => {
                parts.push("LEFT");
                if self.outer {
                    parts.push("OUTER");
                }
            }
            JoinType::Right => {
                parts.push("RIGHT");
                if self.outer {
                    parts.push("OUTER");
                }
            }
            JoinType::Full => {
                parts.push("FULL");
                if self.outer {
                    parts.push("OUTER");
                }
            }
            JoinType::Cross => parts.push("CROSS"),
            JoinType::Join => {}
        }

        parts.push("JOIN");
        SQL::raw(parts.join(" "))
    }
}
