use crate::{Param, Placeholder, SQLParam};

#[derive(Debug, Clone)]
pub struct OwnedParam<V: SQLParam> {
    /// The placeholder to use in the SQL
    pub placeholder: Placeholder,
    /// The value to bind
    pub value: Option<V>,
}

impl<'a, V: SQLParam> From<Param<'a, V>> for OwnedParam<V> {
    fn from(value: Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.map(|v| v.into_owned()),
        }
    }
}

impl<'a, V: SQLParam> From<&Param<'a, V>> for OwnedParam<V> {
    fn from(value: &Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.clone().map(|v| v.into_owned()),
        }
    }
}
