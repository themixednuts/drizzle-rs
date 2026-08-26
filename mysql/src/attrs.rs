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
    DEFAULT,
    DEFAULT_FN,
    DEFAULT_SQL,
    GENERATED,
    VIRTUAL,
    STORED,
    ENUM,
    SET,
    JSON,
    CHECK,
    REFERENCES,
    RELATION,
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
    NUMERIC,
    FLOAT,
    DOUBLE,
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
