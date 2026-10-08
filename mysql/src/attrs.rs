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

/// Names the accessor that loads this table's rows from the referenced table
/// (the reverse relation).
///
/// By default it is the plural of this struct's name (`posts` for a `Post`
/// table), or the singular when this column alone is unique, which loads an
/// `Option` (`profile` for a `Profile` table). A column named for a role
/// rather than the table it references starts the name with the role:
/// `author_id` gives `users.author_posts()`, while `user_id` gives
/// `users.posts()`. The name depends on this column alone, so `RELATION` is needed
/// only to choose another one.
///
/// The forward relation (on this table) is unchanged; only the reverse
/// accessor on the referenced table is renamed.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// // Users get `.authored()` instead of `.author_posts()`
/// #[column(REFERENCES = User::id, RELATION = "authored")]
/// author_id: u64,
/// # "####;
/// ```
///
/// Requires a `REFERENCES` attribute on the same column.
pub const RELATION: AttributeMarker = AttributeMarker;

/// Names the many-to-many accessor the referenced table gets through this
/// link table.
///
/// A table is a link table when it has exactly two foreign keys and its rows
/// are that pair: the pair is the primary key or a `UNIQUE` constraint, or
/// the table has no other column besides a single-column primary key, and
/// neither key is unique on its own. Each referenced table then gets an
/// accessor to the other one, named after the other column: `post_id` and
/// `tag_id` in `PostTags` give `posts.tags()` and `tags.posts()`. A link whose
/// name adds to what it links appends it, so `PostLikes` gives
/// `users.posts_via_likes()` and never clashes with another link.
///
/// `MANY_TO_MANY` chooses another name, and makes any table with two foreign keys a
/// link table.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// // users.liked_posts() instead of users.posts_via_likes()
/// #[column(REFERENCES = User::id, MANY_TO_MANY = "liked_posts")]
/// user_id: u64,
/// # "####;
/// ```
///
/// Requires a `REFERENCES` attribute on the same column.
pub const MANY_TO_MANY: AttributeMarker = AttributeMarker;

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
