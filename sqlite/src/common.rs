pub type Integer = i64;
pub type Real = f64;
pub type Text = String;
pub type Blob = Vec<u8>;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Any {
    Integer(Integer),
    Real(Real),
    Text(Text),
    Blob(Blob),
}

impl Default for Any {
    fn default() -> Self {
        Self::Integer(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Number {
    Int(Integer),
    Real(Real),
}

impl Default for Number {
    fn default() -> Self {
        Self::Int(0)
    }
}

pub enum SQLiteTableType {
    Table,
    View,
    Index,
    Trigger,
}

pub struct SQLiteTableSchema {
    pub name: &'static str,
    pub _type: SQLiteTableType,
}
