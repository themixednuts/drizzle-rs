//! Value generators for seeding database columns.

pub mod numeric;
pub mod special;
pub mod string;
pub mod temporal;

/// Dialect-agnostic seed value (IR only — rendering to SQL is done by core).
#[derive(Debug, Clone, PartialEq)]
pub enum SeedValue {
    Default,
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Blob(Vec<u8>),
    /// Semantic keyword that maps to a dialect-specific current timestamp expression/value.
    CurrentTime,
}

/// Trait for deterministic value generators.
///
/// Each generator produces a single column value given an RNG and a row index.
pub trait Generator: Send + Sync {
    /// Generate a value for row `index`.
    fn generate(&self, rng: &mut dyn RngCore, index: usize, sql_type: &str) -> SeedValue;

    /// Human-readable name of this generator for debugging.
    fn name(&self) -> &'static str;
}

/// Object-safe wrapper for `rand::RngCore`.
pub use rand::RngCore;

/// Which generator to use for a column, determined by type and name heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorKind {
    /// Primary key auto-increment
    IntPrimaryKey,
    /// Regular integer
    Int,
    /// Floating point
    Float,
    /// Boolean
    Bool,
    /// Generic text string
    Text,
    /// First name
    FirstName,
    /// Last name
    LastName,
    /// Full name (first + last)
    FullName,
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// City name
    City,
    /// Country name
    Country,
    /// Street address
    Address,
    /// Job title
    JobTitle,
    /// Company name
    Company,
    /// Lorem ipsum text
    LoremIpsum,
    /// UUID v4
    Uuid,
    /// JSON object
    Json,
    /// Date (YYYY-MM-DD)
    Date,
    /// Timestamp (YYYY-MM-DD HH:MM:SS)
    Timestamp,
    /// Time (HH:MM:SS)
    Time,
    /// Time with timezone (HH:MM:SS+00)
    TimeTz,
    /// Interval (e.g. "12 hours")
    Interval,
    /// Binary blob
    Blob,
    /// `PostgreSQL` INET
    PgInet,
    /// `PostgreSQL` CIDR
    PgCidr,
    /// `PostgreSQL` MACADDR
    PgMacAddr,
    /// `PostgreSQL` MACADDR8
    PgMacAddr8,
    /// `PostgreSQL` POINT
    PgPoint,
    /// `PostgreSQL` LINE
    PgLine,
    /// `PostgreSQL` LSEG
    PgLseg,
    /// `PostgreSQL` BOX
    PgBox,
    /// `PostgreSQL` PATH
    PgPath,
    /// `PostgreSQL` POLYGON
    PgPolygon,
    /// `PostgreSQL` CIRCLE
    PgCircle,
    /// `PostgreSQL` BIT
    PgBit,
    /// `PostgreSQL` VARBIT
    PgVarBit,
    /// `PostgreSQL` arrays (generic empty array literal)
    PgArray,
}

impl GeneratorKind {
    /// Create a boxed `Generator` instance for this kind.
    #[must_use]
    pub fn into_generator(self) -> Box<dyn Generator> {
        match self {
            Self::IntPrimaryKey => Box::new(numeric::IntPrimaryKeyGen),
            Self::Int => Box::new(numeric::IntGen {
                min: 0,
                max: 10_000,
            }),
            Self::Float => Box::new(numeric::FloatGen {
                min: 0.0,
                max: 10_000.0,
            }),
            Self::Bool => Box::new(numeric::BoolGen),
            Self::Text => Box::new(string::TextGen {
                min_len: 5,
                max_len: 50,
            }),
            Self::FirstName => Box::new(string::FirstNameGen),
            Self::LastName => Box::new(string::LastNameGen),
            Self::FullName => Box::new(string::FullNameGen),
            Self::Email => Box::new(string::EmailGen),
            Self::Phone => Box::new(string::PhoneGen),
            Self::City => Box::new(string::CityGen),
            Self::Country => Box::new(string::CountryGen),
            Self::Address => Box::new(string::AddressGen),
            Self::JobTitle => Box::new(string::JobTitleGen),
            Self::Company => Box::new(string::CompanyGen),
            Self::LoremIpsum => Box::new(string::LoremGen { words: 10 }),
            Self::Uuid => Box::new(special::UuidGen),
            Self::Json => Box::new(special::JsonGen),
            Self::Date => Box::new(temporal::DateGen),
            Self::Timestamp => Box::new(temporal::TimestampGen),
            Self::Time => Box::new(temporal::TimeGen),
            Self::TimeTz => Box::new(temporal::TimeTzGen),
            Self::Interval => Box::new(temporal::IntervalGen),
            Self::Blob => Box::new(special::BlobGen { size: 32 }),
            Self::PgInet => Box::new(special::InetGen),
            Self::PgCidr => Box::new(special::CidrGen),
            Self::PgMacAddr => Box::new(special::MacAddrGen),
            Self::PgMacAddr8 => Box::new(special::MacAddr8Gen),
            Self::PgPoint => Box::new(special::PointGen),
            Self::PgLine => Box::new(special::LineGen),
            Self::PgLseg => Box::new(special::LsegGen),
            Self::PgBox => Box::new(special::BoxGen),
            Self::PgPath => Box::new(special::PathGen),
            Self::PgPolygon => Box::new(special::PolygonGen),
            Self::PgCircle => Box::new(special::CircleGen),
            Self::PgBit => Box::new(special::BitGen {
                min_len: 8,
                max_len: 8,
            }),
            Self::PgVarBit => Box::new(special::BitGen {
                min_len: 1,
                max_len: 32,
            }),
            Self::PgArray => Box::new(special::ArrayGen),
        }
    }
}
