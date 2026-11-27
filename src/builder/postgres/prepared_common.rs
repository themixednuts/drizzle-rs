#[cfg(feature = "postgres-sync")]
pub const fn postgres_sync_param_type(
    value: &drizzle_postgres::values::PostgresValue<'_>,
) -> Option<postgres::types::Type> {
    use drizzle_postgres::values::PostgresValue;
    use postgres::types::Type;

    match value {
        PostgresValue::Smallint(_) => Some(Type::INT2),
        PostgresValue::Integer(_) => Some(Type::INT4),
        PostgresValue::Bigint(_) => Some(Type::INT8),
        PostgresValue::Real(_) => Some(Type::FLOAT4),
        PostgresValue::DoublePrecision(_) => Some(Type::FLOAT8),
        #[cfg(feature = "rust-decimal")]
        PostgresValue::Numeric(_) => Some(Type::NUMERIC),
        PostgresValue::Text(_) => Some(Type::TEXT),
        PostgresValue::Bytea(_) => Some(Type::BYTEA),
        PostgresValue::Boolean(_) => Some(Type::BOOL),
        #[cfg(feature = "uuid")]
        PostgresValue::Uuid(_) => Some(Type::UUID),
        #[cfg(feature = "serde")]
        PostgresValue::Json(_) => Some(Type::JSON),
        #[cfg(feature = "serde")]
        PostgresValue::Jsonb(_) => Some(Type::JSONB),
        #[cfg(feature = "chrono")]
        PostgresValue::Date(_) => Some(Type::DATE),
        #[cfg(feature = "chrono")]
        PostgresValue::Time(_) => Some(Type::TIME),
        #[cfg(feature = "chrono")]
        PostgresValue::Timestamp(_) => Some(Type::TIMESTAMP),
        #[cfg(feature = "chrono")]
        PostgresValue::TimestampTz(_) => Some(Type::TIMESTAMPTZ),
        #[cfg(feature = "chrono")]
        PostgresValue::Interval(_) => Some(Type::INTERVAL),
        #[cfg(feature = "cidr")]
        PostgresValue::Inet(_) => Some(Type::INET),
        #[cfg(feature = "cidr")]
        PostgresValue::Cidr(_) => Some(Type::CIDR),
        #[cfg(feature = "cidr")]
        PostgresValue::MacAddr(_) => Some(Type::MACADDR),
        #[cfg(feature = "cidr")]
        PostgresValue::MacAddr8(_) => Some(Type::MACADDR8),
        #[cfg(feature = "geo-types")]
        PostgresValue::Point(_) => Some(Type::POINT),
        #[cfg(feature = "geo-types")]
        PostgresValue::LineString(_) => Some(Type::PATH),
        #[cfg(feature = "geo-types")]
        PostgresValue::Rect(_) => Some(Type::BOX),
        #[cfg(feature = "bit-vec")]
        PostgresValue::BitVec(_) => Some(Type::VARBIT),
        #[cfg(feature = "time")]
        PostgresValue::TimeDate(_) => Some(Type::DATE),
        #[cfg(feature = "time")]
        PostgresValue::TimeTime(_) => Some(Type::TIME),
        #[cfg(feature = "time")]
        PostgresValue::TimeTimestamp(_) => Some(Type::TIMESTAMP),
        #[cfg(feature = "time")]
        PostgresValue::TimeTimestampTz(_) => Some(Type::TIMESTAMPTZ),
        #[cfg(feature = "time")]
        PostgresValue::TimeInterval(_) => Some(Type::INTERVAL),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffDate(_) => Some(Type::DATE),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffTime(_) => Some(Type::TIME),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffDateTime(_) => Some(Type::TIMESTAMP),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffTimestamp(_) => Some(Type::TIMESTAMPTZ),
        PostgresValue::Null | PostgresValue::Enum(_) | PostgresValue::Array(_) => None,
    }
}

#[cfg(any(feature = "tokio-postgres", feature = "hyperdrive"))]
pub const fn tokio_postgres_param_type(
    value: &drizzle_postgres::values::PostgresValue<'_>,
) -> Option<tokio_postgres::types::Type> {
    use drizzle_postgres::values::PostgresValue;
    use tokio_postgres::types::Type;

    match value {
        PostgresValue::Smallint(_) => Some(Type::INT2),
        PostgresValue::Integer(_) => Some(Type::INT4),
        PostgresValue::Bigint(_) => Some(Type::INT8),
        PostgresValue::Real(_) => Some(Type::FLOAT4),
        PostgresValue::DoublePrecision(_) => Some(Type::FLOAT8),
        #[cfg(feature = "rust-decimal")]
        PostgresValue::Numeric(_) => Some(Type::NUMERIC),
        PostgresValue::Text(_) => Some(Type::TEXT),
        PostgresValue::Bytea(_) => Some(Type::BYTEA),
        PostgresValue::Boolean(_) => Some(Type::BOOL),
        #[cfg(feature = "uuid")]
        PostgresValue::Uuid(_) => Some(Type::UUID),
        #[cfg(feature = "serde")]
        PostgresValue::Json(_) => Some(Type::JSON),
        #[cfg(feature = "serde")]
        PostgresValue::Jsonb(_) => Some(Type::JSONB),
        #[cfg(feature = "chrono")]
        PostgresValue::Date(_) => Some(Type::DATE),
        #[cfg(feature = "chrono")]
        PostgresValue::Time(_) => Some(Type::TIME),
        #[cfg(feature = "chrono")]
        PostgresValue::Timestamp(_) => Some(Type::TIMESTAMP),
        #[cfg(feature = "chrono")]
        PostgresValue::TimestampTz(_) => Some(Type::TIMESTAMPTZ),
        #[cfg(feature = "chrono")]
        PostgresValue::Interval(_) => Some(Type::INTERVAL),
        #[cfg(feature = "cidr")]
        PostgresValue::Inet(_) => Some(Type::INET),
        #[cfg(feature = "cidr")]
        PostgresValue::Cidr(_) => Some(Type::CIDR),
        #[cfg(feature = "cidr")]
        PostgresValue::MacAddr(_) => Some(Type::MACADDR),
        #[cfg(feature = "cidr")]
        PostgresValue::MacAddr8(_) => Some(Type::MACADDR8),
        #[cfg(feature = "geo-types")]
        PostgresValue::Point(_) => Some(Type::POINT),
        #[cfg(feature = "geo-types")]
        PostgresValue::LineString(_) => Some(Type::PATH),
        #[cfg(feature = "geo-types")]
        PostgresValue::Rect(_) => Some(Type::BOX),
        #[cfg(feature = "bit-vec")]
        PostgresValue::BitVec(_) => Some(Type::VARBIT),
        #[cfg(feature = "time")]
        PostgresValue::TimeDate(_) => Some(Type::DATE),
        #[cfg(feature = "time")]
        PostgresValue::TimeTime(_) => Some(Type::TIME),
        #[cfg(feature = "time")]
        PostgresValue::TimeTimestamp(_) => Some(Type::TIMESTAMP),
        #[cfg(feature = "time")]
        PostgresValue::TimeTimestampTz(_) => Some(Type::TIMESTAMPTZ),
        #[cfg(feature = "time")]
        PostgresValue::TimeInterval(_) => Some(Type::INTERVAL),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffDate(_) => Some(Type::DATE),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffTime(_) => Some(Type::TIME),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffDateTime(_) => Some(Type::TIMESTAMP),
        #[cfg(feature = "jiff")]
        PostgresValue::JiffTimestamp(_) => Some(Type::TIMESTAMPTZ),
        PostgresValue::Null | PostgresValue::Enum(_) | PostgresValue::Array(_) => None,
    }
}

/// Parameters of unnamed statements, which a handle in pooler mode runs.
///
/// An unnamed statement is parsed, bound and run in one round trip, so the
/// server never describes its parameters: each value carries its type up
/// front. That is the value's own type, an array type taken from the
/// elements, or the unspecified type (OID 0) for a `NULL`, an enum label, or
/// an array of text, enum labels or `NULL`s, which the server infers from the
/// statement as it would for a named one.
#[cfg(any(
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "hyperdrive"
))]
pub(crate) mod unnamed {
    use drizzle_postgres::values::PostgresValue;
    use smallvec::SmallVec;

    #[cfg(feature = "postgres-sync")]
    use super::postgres_sync_param_type as param_type;
    #[cfg(not(feature = "postgres-sync"))]
    use super::tokio_postgres_param_type as param_type;
    #[cfg(feature = "postgres-sync")]
    use postgres::types::{Kind, ToSql, Type};
    #[cfg(not(feature = "postgres-sync"))]
    use tokio_postgres::types::{Kind, ToSql, Type};

    /// Element types whose arrays `PostgresValue` encodes in binary.
    const ARRAY_TYPES: [(Type, Type); 18] = [
        (Type::INT2, Type::INT2_ARRAY),
        (Type::INT4, Type::INT4_ARRAY),
        (Type::INT8, Type::INT8_ARRAY),
        (Type::FLOAT4, Type::FLOAT4_ARRAY),
        (Type::FLOAT8, Type::FLOAT8_ARRAY),
        (Type::NUMERIC, Type::NUMERIC_ARRAY),
        (Type::BYTEA, Type::BYTEA_ARRAY),
        (Type::BOOL, Type::BOOL_ARRAY),
        (Type::UUID, Type::UUID_ARRAY),
        (Type::JSON, Type::JSON_ARRAY),
        (Type::JSONB, Type::JSONB_ARRAY),
        (Type::DATE, Type::DATE_ARRAY),
        (Type::TIME, Type::TIME_ARRAY),
        (Type::TIMESTAMP, Type::TIMESTAMP_ARRAY),
        (Type::TIMESTAMPTZ, Type::TIMESTAMPTZ_ARRAY),
        (Type::INET, Type::INET_ARRAY),
        (Type::CIDR, Type::CIDR_ARRAY),
        (Type::VARBIT, Type::VARBIT_ARRAY),
    ];

    /// Pairs each value with the type its unnamed statement declares.
    pub(crate) fn params<'p>(
        params: &[&'p PostgresValue<'_>],
    ) -> SmallVec<[(&'p (dyn ToSql + Sync), Type); 8]> {
        params
            .iter()
            .map(|&value| (value as &(dyn ToSql + Sync), declared_type(value)))
            .collect()
    }

    fn declared_type(value: &PostgresValue<'_>) -> Type {
        match value {
            PostgresValue::Array(elements) => array_type(elements),
            other => param_type(other),
        }
        .unwrap_or_else(|| Type::new("unspecified".into(), 0, Kind::Pseudo, "pg_catalog".into()))
    }

    /// The array type of the first non-`NULL` element, looking through nested
    /// arrays, which share their element's array type. Text and enum labels
    /// are left to the server, since either may fill an enum array.
    fn array_type(elements: &[PostgresValue<'_>]) -> Option<Type> {
        match elements.iter().find(|element| !element.is_null())? {
            PostgresValue::Array(inner) => array_type(inner),
            element => {
                let element = param_type(element)?;
                ARRAY_TYPES
                    .iter()
                    .find(|(scalar, _)| *scalar == element)
                    .map(|(_, array)| array.clone())
            }
        }
    }
}

macro_rules! postgres_prepared_sync_impl {
    ($client:ty, $row:ty) => {
        impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
            /// Binds `params` and runs the statement, returning the number of rows it
            /// changed.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra), or the
            /// database error when the statement fails.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub fn execute<const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.execute");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                {
                    #[cfg(feature = "profiling")]
                    drizzle_core::drizzle_profile_scope!(
                        "postgres.prepared",
                        "sync.execute.collect"
                    );
                    params_vec.extend(bound_params);
                }
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.execute.db");
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self.statement_cache.execute(client, sql_str, &params_refs);
                result.with_query(context)
            }

            /// Binds `params`, runs the query, and decodes every row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), the database

            /// error when the query fails, or a decode error when a row does not fit

            /// `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub fn all<T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.all");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.all.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params);
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self.statement_cache.query(client, sql_str, &params_refs);
                let rows = result.with_query(context)?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(<Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?);
                }

                Ok(results)
            }

            /// Binds `params`, runs the query, and decodes its single row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), an error

            /// when the query returns several rows or fails, or

            /// a decode error when the row does not fit `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub fn get<T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Option<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.get");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.get.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params);
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self.statement_cache.query_opt(client, sql_str, &params_refs);
                let row = result.with_query(context)?;
                row.map(|row| <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)).transpose()
            }
        }

        impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
            /// Binds `params` and runs the statement, returning the number of rows it
            /// changed.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra), or the
            /// database error when the statement fails.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub fn execute<'a, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_execute");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                {
                    #[cfg(feature = "profiling")]
                    drizzle_core::drizzle_profile_scope!(
                        "postgres.prepared",
                        "sync.owned_execute.collect"
                    );
                    params_vec
                        .extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                }
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_execute.db");
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self.statement_cache.execute(client, sql_str, &params_refs);
                result.with_query(context)
            }

            /// Binds `params`, runs the query, and decodes every row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), the database

            /// error when the query fails, or a decode error when a row does not fit

            /// `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub fn all<'a, T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_all");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_all.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self.statement_cache.query(client, sql_str, &params_refs);
                let rows = result.with_query(context)?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(<Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?);
                }

                Ok(results)
            }

            /// Binds `params`, runs the query, and decodes its single row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), an error

            /// when the query returns several rows or fails, or

            /// a decode error when the row does not fit `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub fn get<'a, T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Option<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_get");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_get.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self.statement_cache.query_opt(client, sql_str, &params_refs);
                let row = result.with_query(context)?;
                row.map(|row| <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)).transpose()
            }
        }
    };
}

macro_rules! postgres_prepared_async_impl {
    ($client:ty, $row:ty) => {
        impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
            /// Binds `params` and runs the statement, returning the number of rows it
            /// changed.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra), or the
            /// database error when the statement fails.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn execute<const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.execute");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.execute.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params);
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self
                    .statement_cache
                    .execute(client, sql_str, &params_refs)
                    .await;
                result.with_query(context)
            }

            /// Binds `params`, runs the query, and decodes every row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), the database

            /// error when the query fails, or a decode error when a row does not fit

            /// `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn all<T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.all");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.all.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params);
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self
                    .statement_cache
                    .query(client, sql_str, &params_refs)
                    .await;
                let rows = result.with_query(context)?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(<Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?);
                }

                Ok(results)
            }

            /// Binds `params`, runs the query, and decodes its single row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), an error

            /// when the query returns several rows or fails, or

            /// a decode error when the row does not fit `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn get<T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Option<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.get");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.get.collect");
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params);
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self
                    .statement_cache
                    .query_opt(client, sql_str, &params_refs)
                    .await;
                let row = result.with_query(context)?;
                row.map(|row| <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)).transpose()
            }
        }

        impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
            /// Binds `params` and runs the statement, returning the number of rows it
            /// changed.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra), or the
            /// database error when the statement fails.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn execute<'a, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.owned_execute");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!(
                    "postgres.prepared",
                    "async.owned_execute.collect"
                );
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self
                    .statement_cache
                    .execute(client, sql_str, &params_refs)
                    .await;
                result.with_query(context)
            }

            /// Binds `params`, runs the query, and decodes every row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), the database

            /// error when the query fails, or a decode error when a row does not fit

            /// `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn all<'a, T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.owned_all");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!(
                    "postgres.prepared",
                    "async.owned_all.collect"
                );
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self
                    .statement_cache
                    .query(client, sql_str, &params_refs)
                    .await;
                let rows = result.with_query(context)?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(<Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?);
                }

                Ok(results)
            }

            /// Binds `params`, runs the query, and decodes its single row into `T`.

            ///

            /// # Errors

            ///

            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the

            /// statement's placeholders (missing, duplicated, or extra), an error

            /// when the query returns several rows or fails, or

            /// a decode error when the row does not fit `T`.

            ///

            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError

            ///

            /// # Panics

            ///

            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn get<'a, T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Option<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.owned_get");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!(
                    "postgres.prepared",
                    "async.owned_get.collect"
                );
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                params_vec.extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                let params_refs: smallvec::SmallVec<[&drizzle_postgres::values::PostgresValue<'_>; 8]> =
                    params_vec.iter().collect();

                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params_vec.iter().collect::<Vec<_>>(),
                    )
                };
                let result = self
                    .statement_cache
                    .query_opt(client, sql_str, &params_refs)
                    .await;
                let row = result.with_query(context)?;
                row.map(|row| <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)).transpose()
            }
        }
    };
}

pub(crate) use postgres_prepared_async_impl;
pub(crate) use postgres_prepared_sync_impl;
