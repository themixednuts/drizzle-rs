//! IDE-visible markers accepted by the MySQL schema macros.
//!
//! Attribute macros parse these tokens before Rust type checking. Re-exporting
//! marker constants from the prelude lets rust-analyzer resolve and document
//! them without turning the attribute vocabulary into runtime state.

/// Zero-sized marker referenced by generated hover bindings.
#[derive(Debug, Clone, Copy)]
pub struct AttributeMarker;

macro_rules! markers {
    ($($name:ident),+ $(,)?) => {
        $(
            #[doc = concat!("MySQL schema attribute marker `", stringify!($name), "`.")]
            pub const $name: AttributeMarker = AttributeMarker;
        )+
    };
}

/// Adds a database `DEFAULT` clause and leaves omitted insert values to MySQL.
///
/// String literals become quoted SQL values. SQL keywords and function calls
/// are emitted as database expressions.
pub const DEFAULT: AttributeMarker = AttributeMarker;

/// Generates an omitted insert value in the Rust application.
///
/// This does not add a database `DEFAULT` clause.
pub const DEFAULT_FN: AttributeMarker = AttributeMarker;

/// Declares a foreign key to another table's column.
///
/// ```rust
/// # let _ = r####"
/// #[column(REFERENCES = User::id)]
/// user_id: u64,
/// # "####;
/// ```
///
/// With the `query` feature this also generates relation accessors: a
/// forward one on this table, named after the column without its `_id` suffix
/// (`user_id` gives `.user()`), and a reverse one on the referenced table
/// (see [`RELATION`]).
pub const REFERENCES: AttributeMarker = AttributeMarker;

/// Sets the reverse relation accessor name on the referenced table.
///
/// By default, reverse relations are named from the referencing struct
/// (`posts` for a `Post` table). When several foreign keys target the same
/// table, or the foreign key is self-referential, the name is disambiguated as
/// `{forward}_{plural}` (e.g. `author_posts`). Use `RELATION` to pick an
/// explicit reverse name instead; it is required only when two reverse names
/// would still collide.
///
/// The forward relation (on this table) is unchanged; only the reverse
/// accessor on the referenced table is renamed.
///
/// ```rust
/// # let _ = r####"
/// // Users get `.authored()` instead of `.author_posts()`
/// #[column(REFERENCES = User::id, RELATION = "authored")]
/// author_id: u64,
///
/// // Still auto-disambiguated: Users get `.editor_posts()`
/// #[column(REFERENCES = User::id)]
/// editor_id: Option<u64>,
/// # "####;
/// ```
///
/// Requires a `REFERENCES` attribute on the same column.
pub const RELATION: AttributeMarker = AttributeMarker;

markers!(
    NAME,
    DATABASE,
    SCHEMA,
    PRIMARY,
    PRIMARY_KEY,
    UNIQUE,
    NOT_NULL,
    AUTO_INCREMENT,
    AUTOINCREMENT,
    GENERATED,
    VIRTUAL,
    STORED,
    ENUM,
    SET,
    JSON,
    CHECK,
    ON_DELETE,
    ON_UPDATE,
    CASCADE,
    SET_NULL,
    RESTRICT,
    NO_ACTION,
    COLLATE,
    CHARACTER_SET,
    CHARSET,
    COMMENT,
    TEMPORARY,
    ENGINE,
    DEFAULT_CHARSET,
    DEFINITION,
    EXISTING,
    ALGORITHM,
    SQL_SECURITY,
    CHECK_OPTION,
    FOREIGN_KEY,
    TINYINT,
    TINYINT_UNSIGNED,
    SMALLINT,
    SMALLINT_UNSIGNED,
    MEDIUMINT,
    MEDIUMINT_UNSIGNED,
    INT,
    INTEGER,
    INT_UNSIGNED,
    INTEGER_UNSIGNED,
    BIGINT,
    BIGINT_UNSIGNED,
    DECIMAL,
    DECIMAL_UNSIGNED,
    NUMERIC,
    NUMERIC_UNSIGNED,
    FLOAT,
    FLOAT_UNSIGNED,
    DOUBLE,
    DOUBLE_UNSIGNED,
    REAL,
    REAL_UNSIGNED,
    BOOLEAN,
    BOOL,
    BIT,
    CHAR,
    VARCHAR,
    TINYTEXT,
    TEXT,
    MEDIUMTEXT,
    LONGTEXT,
    BINARY,
    VARBINARY,
    TINYBLOB,
    BLOB,
    MEDIUMBLOB,
    LONGBLOB,
    DATE,
    TIME,
    DATETIME,
    TIMESTAMP,
    YEAR,
);
