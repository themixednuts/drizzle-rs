use crate::SQLParam;
use crate::prelude::*;

pub struct Query<V: SQLParam> {
    sql: String,
    params: Box<V>,
}

impl<V: SQLParam> Query<V> {
    pub fn sql(&self) -> String {
        self.sql.clone()
    }

    pub fn params(&self) -> Box<V> {
        self.params.clone()
    }
}
