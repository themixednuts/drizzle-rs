#[cfg(feature = "postgres-sync")]
pub(crate) fn postgres_sync_param_type(
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
        PostgresValue::Null | PostgresValue::Enum(_) | PostgresValue::Array(_) => None,
    }
}

macro_rules! postgres_prepared_sync_impl {
    ($client:ty, $row:ty, $to_sql:path) => {
        impl<'a> PreparedStatement<'a> {
            /// Runs the prepared statement and returns the number of affected rows
            pub fn execute(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.execute");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                #[cfg(feature = "postgres-sync")]
                {
                    let mut typed_params: smallvec::SmallVec<
                        [(&(dyn $to_sql + Sync), postgres::types::Type); 8],
                    > = smallvec::SmallVec::with_capacity(params_vec.len());
                    let mut all_typed = true;
                    for p in &params_vec {
                        if let Some(ty) =
                            crate::builder::postgres::prepared_common::postgres_sync_param_type(p)
                        {
                            typed_params.push((p as &(dyn $to_sql + Sync), ty));
                        } else {
                            all_typed = false;
                            break;
                        }
                    }

                    if all_typed {
                        #[cfg(feature = "profiling")]
                        drizzle_core::drizzle_profile_scope!(
                            "postgres.prepared",
                            "sync.execute.db_typed"
                        );
                        let mut rows = client.query_typed_raw(sql_str, typed_params)?;
                        while postgres::fallible_iterator::FallibleIterator::next(&mut rows)?
                            .is_some()
                        {}
                        return Ok(rows.rows_affected().unwrap_or(0));
                    }
                }

                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.execute.db");
                client.execute(sql_str, &params_refs).map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub fn all<T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.all");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let rows = client.query(sql_str, &params_refs)?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(T::try_from(&row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub fn get<T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.get");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let row = client.query_one(sql_str, &params_refs)?;
                T::try_from(&row).map_err(Into::into)
            }
        }

        impl OwnedPreparedStatement {
            /// Runs the prepared statement and returns the number of affected rows
            pub fn execute<'a>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_execute");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                #[cfg(feature = "postgres-sync")]
                {
                    let mut typed_params: smallvec::SmallVec<
                        [(&(dyn $to_sql + Sync), postgres::types::Type); 8],
                    > = smallvec::SmallVec::with_capacity(params_vec.len());
                    let mut all_typed = true;
                    for p in &params_vec {
                        if let Some(ty) =
                            crate::builder::postgres::prepared_common::postgres_sync_param_type(p)
                        {
                            typed_params.push((p as &(dyn $to_sql + Sync), ty));
                        } else {
                            all_typed = false;
                            break;
                        }
                    }

                    if all_typed {
                        #[cfg(feature = "profiling")]
                        drizzle_core::drizzle_profile_scope!(
                            "postgres.prepared",
                            "sync.owned_execute.db_typed"
                        );
                        let mut rows = client.query_typed_raw(sql_str, typed_params)?;
                        while postgres::fallible_iterator::FallibleIterator::next(&mut rows)?
                            .is_some()
                        {}
                        return Ok(rows.rows_affected().unwrap_or(0));
                    }
                }

                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_execute.db");
                client.execute(sql_str, &params_refs).map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub fn all<'a, T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_all");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let rows = client.query(sql_str, &params_refs)?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(T::try_from(&row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub fn get<'a, T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "sync.owned_get");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let row = client.query_one(sql_str, &params_refs)?;
                T::try_from(&row).map_err(Into::into)
            }
        }
    };
}

macro_rules! postgres_prepared_async_impl {
    ($client:ty, $row:ty, $to_sql:path) => {
        impl<'a> PreparedStatement<'a> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.execute");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                client
                    .execute(sql_str, &params_refs)
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.all");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let rows = client.query(sql_str, &params_refs).await?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(T::try_from(&row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.get");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let row = client.query_one(sql_str, &params_refs).await?;
                T::try_from(&row).map_err(Into::into)
            }
        }

        impl OwnedPreparedStatement {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<'a>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.owned_execute");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                client
                    .execute(sql_str, &params_refs)
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<'a, T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.owned_all");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let rows = client.query(sql_str, &params_refs).await?;

                let mut results = Vec::with_capacity(rows.len());
                // Consume rows by value so each decoded row can be dropped immediately.
                // Iterating by reference keeps the full row buffer live for the entire decode pass.
                for row in rows {
                    results.push(T::try_from(&row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<'a, T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                #[cfg(feature = "profiling")]
                drizzle_core::drizzle_profile_scope!("postgres.prepared", "async.owned_get");
                let (sql_str, bound_params) = self.inner.bind(params);
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

                let row = client.query_one(sql_str, &params_refs).await?;
                T::try_from(&row).map_err(Into::into)
            }
        }
    };
}

pub(crate) use postgres_prepared_async_impl;
pub(crate) use postgres_prepared_sync_impl;
