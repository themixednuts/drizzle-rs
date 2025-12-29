use drizzle_postgres::values::PostgresValue;
use drizzle_core::{
    OwnedParam, Param,
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
};
use drizzle_postgres::values::OwnedPostgresValue;
use std::borrow::Cow;

/// A prepared statement for PostgreSQL queries using sqlx
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub inner: CorePreparedStatement<'a, PostgresValue<'a>>,
}

impl<'a> PreparedStatement<'a> {
    pub fn into_owned(self) -> OwnedPreparedStatement {
        let owned_params = self.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedPostgresValue::from(v.into_owned())),
        });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: owned_params.collect::<Box<[_]>>(),
        };

        OwnedPreparedStatement { inner }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub inner: CoreOwnedPreparedStatement<OwnedPostgresValue>,
}

impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        value.into_owned()
    }
}

impl From<OwnedPreparedStatement> for PreparedStatement<'_> {
    fn from(value: OwnedPreparedStatement) -> Self {
        let postgres_params = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(PostgresValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: postgres_params.collect::<Box<[_]>>(),
        };
        PreparedStatement { inner }
    }
}