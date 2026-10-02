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
    /// A `*_by_name` setting names a table the schema does not have.
    UnknownTable {
        /// The name as given.
        table: String,
        /// The schema's tables.
        known: Vec<String>,
    },
    /// A `*_by_name` setting names a table that exists in several
    /// namespaces; qualify it as `schema.table`.
    AmbiguousTable {
        /// The name as given.
        table: String,
        /// The matching tables.
        candidates: Vec<String>,
    },
    /// A `*_by_name` setting names a column the table does not have.
    UnknownColumn {
        /// The table.
        table: String,
        /// The column name as given.
        column: String,
        /// The table's columns.
        known: Vec<String>,
    },
    /// `relation_by_name` was given a `child` with no foreign key to
    /// `parent`.
    NotRelated {
        /// The parent table.
        parent: String,
        /// The child table.
        child: String,
    },
    /// `Schema::from_snapshot` was given a snapshot for a dialect whose
    /// feature (`sqlite`, `postgres` or `mysql`) is off.
    DialectNotEnabled {
        /// The snapshot's dialect.
        dialect: String,
    },
    /// A value has no SQL literal form, so the statement cannot be written
    /// with its values inline (`inline_sql`, `try_generate_script`).
    NoLiteral {
        /// What is wrong with the value.
        reason: String,
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
            Self::UnknownTable { table, known } => write!(
                formatter,
                "no table `{table}` in the schema (tables: {})",
                known.join(", ")
            ),
            Self::AmbiguousTable { table, candidates } => write!(
                formatter,
                "table name `{table}` matches {}; qualify it as `schema.table`",
                candidates.join(", ")
            ),
            Self::UnknownColumn {
                table,
                column,
                known,
            } => write!(
                formatter,
                "table {table} has no column `{column}` (columns: {})",
                known.join(", ")
            ),
            Self::NotRelated { parent, child } => write!(
                formatter,
                "cannot set a relation count from {parent} to {child}: {child} has no foreign key to {parent}"
            ),
            Self::DialectNotEnabled { dialect } => write!(
                formatter,
                "a {dialect} snapshot needs drizzle-seed's feature for that dialect"
            ),
            Self::NoLiteral { reason } => {
                write!(formatter, "cannot write a value inline: {reason}")
            }
            Self::InvalidValue {
                table,
                column,
                reason,
            } => write!(formatter, "cannot seed {table}.{column}: {reason}"),
        }
    }
}

impl std::error::Error for SeedError {}
