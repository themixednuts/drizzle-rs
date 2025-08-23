use drizzle_core::error::DrizzleError;

/// Trait for connections that can execute queries
pub trait Execute<Q> {
    type ExecuteOutput;
    type AllOutput<T>;
    type GetOutput<T>;
    type Row<'r>;

    fn execute(query: Q, conn: &Self) -> Self::ExecuteOutput;

    fn all<T>(query: Q, conn: &Self) -> Self::AllOutput<T>
    where
        T: for<'r> TryFrom<&'r Self::Row<'r>>,
        for<'r> <T as TryFrom<&'r Self::Row<'r>>>::Error: Into<DrizzleError>;

    fn get<T>(query: Q, conn: &Self) -> Self::GetOutput<T>
    where
        T: for<'r> TryFrom<&'r Self::Row<'r>>,
        for<'r> <T as TryFrom<&'r Self::Row<'r>>>::Error: Into<DrizzleError>;
}
