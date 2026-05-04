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
        PostgresValue::Null | PostgresValue::Enum(_) | PostgresValue::Array(_) => None,
    }
}

#[cfg(feature = "postgres-sync")]
pub fn postgres_sync_param_types(
    params: &[drizzle_postgres::values::PostgresValue<'_>],
) -> smallvec::SmallVec<[postgres::types::Type; 8]> {
    let mut types = smallvec::SmallVec::with_capacity(params.len());
    for param in params {
        let Some(ty) = postgres_sync_param_type(param) else {
            types.clear();
            break;
        };
        types.push(ty);
    }
    types
}

#[cfg(feature = "tokio-postgres")]
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
        PostgresValue::Null | PostgresValue::Enum(_) | PostgresValue::Array(_) => None,
    }
}

#[cfg(feature = "tokio-postgres")]
pub fn tokio_postgres_param_types(
    params: &[drizzle_postgres::values::PostgresValue<'_>],
) -> smallvec::SmallVec<[tokio_postgres::types::Type; 8]> {
    let mut types = smallvec::SmallVec::with_capacity(params.len());
    for param in params {
        let Some(ty) = tokio_postgres_param_type(param) else {
            types.clear();
            break;
        };
        types.push(ty);
    }
    types
}

macro_rules! postgres_prepared_sync_impl {
    ($client:ty, $row:ty, $to_sql:path) => {
        impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
            /// Runs the prepared statement and returns the number of affected rows
            pub fn execute<const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.execute");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'a>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::new();
                {
                    #[cfg(feature = "profiling")]
                    drizzle_core::drizzle_profile_scope!(
                        "postgres.prepared",
                        "sync.execute.collect"
                    );
                    params_vec.extend(bound_params);
                    params_refs.reserve(params_vec.len());
                    for p in &params_vec {
                        params_refs.push(p as &(dyn $to_sql + Sync));
                    }
                }

                let param_types =
                    crate::builder::postgres::prepared_common::postgres_sync_param_types(
                        &params_vec,
                    );

                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.execute.db");
                let statement = self.driver_statement(client, sql_str, &param_types)?;
                client.execute(&statement, &params_refs).map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub fn all<T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::postgres_sync_param_types(
                        &params_vec,
                    );

                let statement = self.driver_statement(client, sql_str, &param_types)?;
                let rows = client.query(&statement, &params_refs)?;

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

            /// Runs the prepared statement and returns a single row
            pub fn get<T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<T>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::postgres_sync_param_types(
                        &params_vec,
                    );

                let statement = self.driver_statement(client, sql_str, &param_types)?;
                let row = client.query_one(&statement, &params_refs)?;
                <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)
            }
        }

        impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
            /// Runs the prepared statement and returns the number of affected rows
            pub fn execute<'a, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_execute");
                let (sql_str, bound_params) = self.inner.bind(params)?;
                let (lower, upper) = bound_params.size_hint();
                let mut params_vec: smallvec::SmallVec<
                    [drizzle_postgres::values::PostgresValue<'_>; 8],
                > = smallvec::SmallVec::with_capacity(upper.unwrap_or(lower));
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::new();
                {
                    #[cfg(feature = "profiling")]
                    drizzle_core::drizzle_profile_scope!(
                        "postgres.prepared",
                        "sync.owned_execute.collect"
                    );
                    params_vec
                        .extend(bound_params.map(drizzle_postgres::values::PostgresValue::from));
                    params_refs.reserve(params_vec.len());
                    for p in &params_vec {
                        params_refs.push(p as &(dyn $to_sql + Sync));
                    }
                }

                let param_types =
                    crate::builder::postgres::prepared_common::postgres_sync_param_types(
                        &params_vec,
                    );

                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_execute.db");
                let statement = self.driver_statement(client, sql_str, &param_types)?;
                client.execute(&statement, &params_refs).map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub fn all<'a, T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::postgres_sync_param_types(
                        &params_vec,
                    );

                let statement = self.driver_statement(client, sql_str, &param_types)?;
                let rows = client.query(&statement, &params_refs)?;

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

            /// Runs the prepared statement and returns a single row
            pub fn get<'a, T, const N: usize>(
                &self,
                client: &mut $client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<T>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::postgres_sync_param_types(
                        &params_vec,
                    );

                let statement = self.driver_statement(client, sql_str, &param_types)?;
                let row = client.query_one(&statement, &params_refs)?;
                <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)
            }
        }
    };
}

macro_rules! postgres_prepared_async_impl {
    ($client:ty, $row:ty, $to_sql:path) => {
        impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::tokio_postgres_param_types(
                        &params_vec,
                    );

                let statement = self
                    .driver_statement(client, sql_str, &param_types)
                    .await?;
                client
                    .execute(&statement, &params_refs)
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::tokio_postgres_param_types(
                        &params_vec,
                    );

                let statement = self
                    .driver_statement(client, sql_str, &param_types)
                    .await?;
                let rows = client.query(&statement, &params_refs).await?;

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

            /// Runs the prepared statement and returns a single row
            pub async fn get<T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<T>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::tokio_postgres_param_types(
                        &params_vec,
                    );

                let statement = self
                    .driver_statement(client, sql_str, &param_types)
                    .await?;
                let row = client.query_one(&statement, &params_refs).await?;
                <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)
            }
        }

        impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<'a, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<u64> {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::tokio_postgres_param_types(
                        &params_vec,
                    );

                let statement = self
                    .driver_statement(client, sql_str, &param_types)
                    .await?;
                client
                    .execute(&statement, &params_refs)
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<'a, T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::tokio_postgres_param_types(
                        &params_vec,
                    );

                let statement = self
                    .driver_statement(client, sql_str, &param_types)
                    .await?;
                let rows = client.query(&statement, &params_refs).await?;

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

            /// Runs the prepared statement and returns a single row
            pub async fn get<'a, T, const N: usize>(
                &self,
                client: &$client,
                params: [drizzle_core::param::ParamBind<'a, drizzle_postgres::values::PostgresValue<'a>>; N],
            ) -> drizzle_core::error::Result<T>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
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
                let mut params_refs: smallvec::SmallVec<[&(dyn $to_sql + Sync); 8]> =
                    smallvec::SmallVec::with_capacity(params_vec.len());
                for p in &params_vec {
                    params_refs.push(p as &(dyn $to_sql + Sync));
                }

                let param_types =
                    crate::builder::postgres::prepared_common::tokio_postgres_param_types(
                        &params_vec,
                    );

                let statement = self
                    .driver_statement(client, sql_str, &param_types)
                    .await?;
                let row = client.query_one(&statement, &params_refs).await?;
                <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)
            }
        }
    };
}

pub(crate) use postgres_prepared_async_impl;
pub(crate) use postgres_prepared_sync_impl;
