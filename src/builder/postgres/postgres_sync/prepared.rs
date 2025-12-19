use std::borrow::Cow;

use drizzle_core::error::{DrizzleError, Result};
use drizzle_core::{
    param::{OwnedParam, Param, ParamBind},
    prepared::{
        PreparedStatement as CorePreparedStatement,
        owned::OwnedPreparedStatement as CoreOwnedPreparedStatement,
    },
    traits::ToSQL,
};
use drizzle_postgres::{PostgresValue, values::OwnedPostgresValue};
use postgres::{Client, Row, types::ToSql};

/// A prepared statement that can be executed multiple times with different parameters.
///
/// This is a wrapper around an owned prepared statement that can be used with the postgres driver.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub inner: CorePreparedStatement<'a, PostgresValue<'a>>,
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

impl<'a> PreparedStatement<'a> {
    /// Gets the SQL query string by reconstructing it from text segments
    pub fn sql(&self) -> String {
        self.inner.to_sql().sql()
    }

    /// Gets the number of parameters in the query
    pub fn param_count(&self) -> usize {
        self.inner.params.len()
    }

    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute(
        &self,
        client: &mut Client,
        params: impl IntoIterator<Item = ParamBind<'a, PostgresValue<'a>>>,
    ) -> Result<u64> {
        let (sql_str, bound_params) = self.inner.bind(params);
        let params_vec: Vec<PostgresValue<'a>> = bound_params.collect();
        let params_refs: Vec<&(dyn ToSql + Sync)> = params_vec
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect();

        client.execute(&sql_str, &params_refs).map_err(Into::into)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<T>(
        &self,
        client: &mut Client,
        params: impl IntoIterator<Item = ParamBind<'a, PostgresValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, bound_params) = self.inner.bind(params);
        let params_vec: Vec<PostgresValue<'a>> = bound_params.collect();
        let params_refs: Vec<&(dyn ToSql + Sync)> = params_vec
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect();

        let rows = client.query(&sql_str, &params_refs)?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            results.push(T::try_from(row).map_err(Into::into)?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<T>(
        &self,
        client: &mut Client,
        params: impl IntoIterator<Item = ParamBind<'a, PostgresValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, bound_params) = self.inner.bind(params);
        let params_vec: Vec<PostgresValue<'a>> = bound_params.collect();
        let params_refs: Vec<&(dyn ToSql + Sync)> = params_vec
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect();

        let row = client.query_one(&sql_str, &params_refs)?;
        T::try_from(&row).map_err(Into::into)
    }

    /// Converts this borrowed prepared statement into an owned one.
    pub fn into_owned(self) -> OwnedPreparedStatement {
        let owned_params = self
            .inner
            .params
            .into_vec()
            .into_iter()
            .map(|p| OwnedParam {
                placeholder: p.placeholder,
                value: p.value.map(|v| OwnedPostgresValue::from(v.into_owned())),
            });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments,
            params: owned_params.collect::<Box<[_]>>(),
        };

        OwnedPreparedStatement { inner }
    }
}

/// Owned PostgreSQL prepared statement wrapper.
///
/// This is the owned counterpart to [`PreparedStatement`] that doesn't have any lifetime
/// constraints. All data is owned by this struct, making it suitable for long-term storage,
/// caching, or passing across thread boundaries.
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub inner: CoreOwnedPreparedStatement<OwnedPostgresValue>,
}

impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        value.into_owned()
    }
}

impl OwnedPreparedStatement {
    /// Gets the SQL query string by reconstructing it from text segments
    pub fn sql(&self) -> String {
        use drizzle_core::traits::ToSQL;
        self.inner.to_sql().sql()
    }

    /// Gets the number of parameters in the query
    pub fn param_count(&self) -> usize {
        self.inner.params.len()
    }

    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute<'a>(
        &self,
        client: &mut Client,
        params: impl IntoIterator<Item = ParamBind<'a, PostgresValue<'a>>>,
    ) -> Result<u64> {
        let (sql_str, bound_params) = self.inner.bind(params);
        let params_vec: Vec<PostgresValue<'_>> = bound_params.map(PostgresValue::from).collect();
        let params_refs: Vec<&(dyn ToSql + Sync)> = params_vec
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect();

        client.execute(&sql_str, &params_refs).map_err(Into::into)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<'a, T>(
        &self,
        client: &mut Client,
        params: impl IntoIterator<Item = ParamBind<'a, PostgresValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, bound_params) = self.inner.bind(params);
        let params_vec: Vec<PostgresValue<'_>> = bound_params.map(PostgresValue::from).collect();
        let params_refs: Vec<&(dyn ToSql + Sync)> = params_vec
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect();

        let rows = client.query(&sql_str, &params_refs)?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            results.push(T::try_from(row).map_err(Into::into)?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<'a, T>(
        &self,
        client: &mut Client,
        params: impl IntoIterator<Item = ParamBind<'a, PostgresValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, bound_params) = self.inner.bind(params);
        let params_vec: Vec<PostgresValue<'_>> = bound_params.map(PostgresValue::from).collect();
        let params_refs: Vec<&(dyn ToSql + Sync)> = params_vec
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect();

        let row = client.query_one(&sql_str, &params_refs)?;
        T::try_from(&row).map_err(Into::into)
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

impl<'a> ToSQL<'a, PostgresValue<'a>> for PreparedStatement<'a> {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.inner.to_sql()
    }
}

impl<'a> ToSQL<'a, OwnedPostgresValue> for OwnedPreparedStatement {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, OwnedPostgresValue> {
        self.inner.to_sql()
    }
}
