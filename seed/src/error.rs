use core::fmt;

/// Failure to build a portable seed plan.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SeedError {
    /// The selected tables contain a foreign-key cycle.
    CyclicForeignKeys { tables: Vec<String> },
    /// A seeded table needs rows from a parent that is not being seeded.
    MissingParentRows { child: String, parent: String },
    /// Resetting a parent while leaving a referencing child untouched is unsafe.
    UnsafeResetSelection {
        parent: String,
        skipped_child: String,
    },
    /// One row alone exceeds the configured bind-parameter limit.
    ParameterLimitTooLow {
        table: String,
        required: usize,
        limit: usize,
    },
    /// A generated value cannot be represented by the target SQL type.
    InvalidValue {
        table: String,
        column: String,
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
