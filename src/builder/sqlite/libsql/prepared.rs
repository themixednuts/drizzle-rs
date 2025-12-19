use drizzle_core::error::{DrizzleError, Result};
use drizzle_core::{
    param::{OwnedParam, Param, ParamBind},
    prepared::{
        PreparedStatement as CorePreparedStatement,
        owned::OwnedPreparedStatement as CoreOwnedPreparedStatement,
    },
    traits::ToSQL,
};
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};
use std::borrow::Cow;

use libsql::{Connection, Row, params_from_iter};

#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
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

impl<'a> PreparedStatement<'a> {
    /// Runs the prepared statement and returns the number of affected rows
    pub async fn execute(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<u64> {
        let (sql_str, params) = self.inner.bind(params);

        // Execute with connection
        conn.execute(&sql_str, params_from_iter(params))
            .await
            .map_err(Into::into)
    }

    /// Runs the prepared statement and returns all matching rows
    pub async fn all<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.inner.bind(params);

        // Execute with connection
        let mut rows = conn.query(&sql_str, params_from_iter(params)).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub async fn get<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.inner.bind(params);
        // Execute with connection
        let mut rows = conn.query(&sql_str, params_from_iter(params)).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    pub fn into_owned(self) -> OwnedPreparedStatement {
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

#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub inner: CoreOwnedPreparedStatement<OwnedSQLiteValue>,
}

impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        value.into_owned()
    }
}

impl OwnedPreparedStatement {
    /// Runs the prepared statement and returns the number of affected rows
    pub async fn execute<'a>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<u64> {
        let (sql_str, params) = self.inner.bind(params);

        // Execute with connection
        conn.execute(&sql_str, params_from_iter(params))
            .await
            .map_err(Into::into)
    }

    /// Runs the prepared statement and returns all matching rows
    pub async fn all<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.inner.bind(params);

        // Execute with connection
        let mut rows = conn.query(&sql_str, params_from_iter(params)).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub async fn get<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.inner.bind(params);
        // Execute with connection
        let mut rows = conn.query(&sql_str, params_from_iter(params)).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}

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

impl<'a> ToSQL<'a, SQLiteValue<'a>> for PreparedStatement<'a> {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.inner.to_sql()
    }
}

impl<'a> ToSQL<'a, OwnedSQLiteValue> for OwnedPreparedStatement {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, OwnedSQLiteValue> {
        self.inner.to_sql()
    }
}
