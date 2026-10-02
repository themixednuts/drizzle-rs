use crate::{Param, Placeholder, SQLParam};

/// A [`Param`] that owns its value, so it has no lifetime.
#[derive(Debug, Clone)]
pub struct OwnedParam<V: SQLParam> {
    /// The placeholder written into the SQL text.
    pub placeholder: Placeholder,
    /// The bound value, or `None` until one is bound.
    pub value: Option<V>,
}

impl<'a, V: SQLParam> From<Param<'a, V>> for OwnedParam<V> {
    fn from(value: Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.map(crate::prelude::Cow::into_owned),
        }
    }
}

impl<'a, V: SQLParam> From<&Param<'a, V>> for OwnedParam<V> {
    fn from(value: &Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.clone().map(crate::prelude::Cow::into_owned),
        }
    }
}
