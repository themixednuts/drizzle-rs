pub mod builders;
pub mod expressions;
pub mod traits;

pub trait ToSQL {
    fn to_sql(self) -> String;
}

pub enum SQLChunk {
    Str(&'static str),
    Table,
    View,
    AnyColumn,
    Name,
    Param,
    Placeholder,
    SQL,
}

// pub struct SQL<T> {
//     decoder: dyn Any,
// }

pub fn sql<T>() {}
