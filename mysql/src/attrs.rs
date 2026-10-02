//! Names accepted inside `#[MySQLTable(...)]`, `#[column(...)]`,
//! `#[MySQLView(...)]` and `#[MySQLIndex(...)]`.
//!
//! The macros read these attributes by name before type checking. The
//! constants here exist so your editor can resolve each attribute and show
//! its documentation on hover; import them through the prelude. They carry
//! no runtime state.

/// The type of every attribute constant in this module.
#[derive(Debug, Clone, Copy)]
pub struct AttributeMarker;

macro_rules! markers {
    ($($name:ident),+ $(,)?) => {
        $(
            #[doc = concat!("The `", stringify!($name), "` schema attribute.")]
            pub const $name: AttributeMarker = AttributeMarker;
        )+
    };
}

/// Adds a `DEFAULT` clause to the column, so `MySQL` fills it when an insert
/// leaves it out.
///
/// String literals become quoted SQL values. SQL keywords and function calls
/// are written as SQL expressions.
pub const DEFAULT: AttributeMarker = AttributeMarker;

/// Generates a value in Rust for each insert that leaves the column unset.
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
