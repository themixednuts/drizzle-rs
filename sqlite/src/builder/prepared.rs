use std::borrow::Cow;

use drizzle_core::{
    OwnedParam, Param,
    prepared::{
        PreparedStatement as CorePreparedStatement,
        owned::OwnedPreparedStatement as CoreOwnedPreparedStatement,
    },
};

use crate::{SQLiteValue, values::OwnedSQLiteValue};

/// Generic prepared statement wrapper for SQLite
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub inner: CorePreparedStatement<'a, crate::SQLiteValue<'a>>,
}

impl<'a> PreparedStatement<'a> {
    pub fn into_owned(&self) -> OwnedPreparedStatement {
        let owned_params = self.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: owned_params.collect::<Box<[_]>>(),
        };

        OwnedPreparedStatement { inner }
    }
}

/// Generic owned prepared statement wrapper for SQLite
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub inner: CoreOwnedPreparedStatement<crate::values::OwnedSQLiteValue>,
}
impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        let owned_params = value.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });
        let inner = CoreOwnedPreparedStatement {
            text_segments: value.inner.text_segments,
            params: owned_params.collect::<Box<[_]>>(),
        };
        Self { inner }
    }
}

impl From<OwnedPreparedStatement> for PreparedStatement<'_> {
    fn from(value: OwnedPreparedStatement) -> Self {
        let sqlitevalue = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(SQLiteValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: sqlitevalue.collect::<Box<[_]>>(),
        };
        PreparedStatement { inner }
    }
}

impl OwnedPreparedStatement {}

impl<'a> std::fmt::Display for PreparedStatement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::fmt::Display for OwnedPreparedStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}
