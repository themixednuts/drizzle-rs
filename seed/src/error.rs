use core::fmt;

/// Why [`SeedConfig`](crate::SeedConfig) could not build the statements
/// (returned by `try_generate` and `reset_plan`).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SeedError {
    /// The non-skipped `tables` have a foreign-key cycle, so there is no
    /// parent-first order. A self-reference counts only when all of its
    /// columns are `NOT NULL`.
    CyclicForeignKeys { tables: Vec<String> },
    /// A `NOT NULL` foreign key in `child` has no `parent` row to point at:
    /// the seeded parent got zero rows, or the relation count is zero. (A
    /// nullable foreign key is set to `NULL` instead.)
    MissingParentRows { child: String, parent: String },
    /// `reset_plan` would delete a parent's rows while a skipped table still
    /// references it.
    UnsafeResetSelection {
        /// The table that would be emptied.
        parent: String,
        /// The skipped table that references it.
        skipped_child: String,
    },
    /// A single row needs more bind parameters than the limit (`max_params`,
    /// or the dialect's maximum).
    ParameterLimitTooLow {
        /// The table being seeded.
        table: String,
        /// Parameters one row needs.
        required: usize,
        /// The configured limit.
        limit: usize,
    },
    /// A generated value cannot be stored in the column's SQL type.
    InvalidValue {
        /// The table being seeded.
        table: String,
        /// The column.
        column: String,
        /// What is wrong with the value.
        reason: String,
    },
}

impl fmt::Display for SeedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CyclicForeignKeys { tables } => write!(
                formatter,
                "cannot seed a foreign-key cycle in one portable pass: {}",
                tables.join(", ")
            ),
            Self::MissingParentRows { child, parent } => write!(
                formatter,
                "cannot seed {child}: referenced parent {parent} has no generated rows"
            ),
            Self::UnsafeResetSelection {
                parent,
                skipped_child,
            } => write!(
                formatter,
                "cannot reset {parent} while referenced table {skipped_child} is skipped"
            ),
            Self::ParameterLimitTooLow {
                table,
                required,
                limit,
            } => write!(
                formatter,
                "cannot seed {table}: one row needs {required} bind parameters, above limit {limit}"
            ),
            Self::InvalidValue {
                table,
                column,
                reason,
            } => write!(formatter, "cannot seed {table}.{column}: {reason}"),
        }
    }
}

impl std::error::Error for SeedError {}
