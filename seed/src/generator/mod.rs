//! Value generators for seeding database columns.

pub(crate) mod numeric;
pub(crate) mod special;
pub(crate) mod string;
pub(crate) mod temporal;

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
    /// PostgreSQL INET
    PgInet,
    /// PostgreSQL CIDR
    PgCidr,
    /// PostgreSQL MACADDR
    PgMacAddr,
    /// PostgreSQL MACADDR8
    PgMacAddr8,
    /// PostgreSQL POINT
    PgPoint,
    /// PostgreSQL LINE
    PgLine,
    /// PostgreSQL LSEG
    PgLseg,
    /// PostgreSQL BOX
    PgBox,
    /// PostgreSQL PATH
    PgPath,
    /// PostgreSQL POLYGON
    PgPolygon,
    /// PostgreSQL CIRCLE
    PgCircle,
    /// PostgreSQL BIT
    PgBit,
    /// PostgreSQL VARBIT
    PgVarBit,
    /// PostgreSQL arrays (generic empty array literal)
    PgArray,
}

impl GeneratorKind {
    /// Create a boxed `Generator` instance for this kind.
    pub fn into_generator(self) -> Box<dyn Generator> {
        match self {
            GeneratorKind::IntPrimaryKey => Box::new(numeric::IntPrimaryKeyGen),
            GeneratorKind::Int => Box::new(numeric::IntGen {
                min: 0,
                max: 10_000,
            }),
            GeneratorKind::Float => Box::new(numeric::FloatGen {
                min: 0.0,
                max: 10_000.0,
            }),
            GeneratorKind::Bool => Box::new(numeric::BoolGen),
            GeneratorKind::Text => Box::new(string::TextGen {
                min_len: 5,
                max_len: 50,
            }),
            GeneratorKind::FirstName => Box::new(string::FirstNameGen),
            GeneratorKind::LastName => Box::new(string::LastNameGen),
            GeneratorKind::FullName => Box::new(string::FullNameGen),
            GeneratorKind::Email => Box::new(string::EmailGen),
            GeneratorKind::Phone => Box::new(string::PhoneGen),
            GeneratorKind::City => Box::new(string::CityGen),
            GeneratorKind::Country => Box::new(string::CountryGen),
            GeneratorKind::Address => Box::new(string::AddressGen),
            GeneratorKind::JobTitle => Box::new(string::JobTitleGen),
            GeneratorKind::Company => Box::new(string::CompanyGen),
            GeneratorKind::LoremIpsum => Box::new(string::LoremGen { words: 10 }),
            GeneratorKind::Uuid => Box::new(special::UuidGen),
            GeneratorKind::Json => Box::new(special::JsonGen),
            GeneratorKind::Date => Box::new(temporal::DateGen),
            GeneratorKind::Timestamp => Box::new(temporal::TimestampGen),
            GeneratorKind::Time => Box::new(temporal::TimeGen),
            GeneratorKind::TimeTz => Box::new(temporal::TimeTzGen),
            GeneratorKind::Interval => Box::new(temporal::IntervalGen),
            GeneratorKind::Blob => Box::new(special::BlobGen { size: 32 }),
            GeneratorKind::PgInet => Box::new(special::InetGen),
            GeneratorKind::PgCidr => Box::new(special::CidrGen),
            GeneratorKind::PgMacAddr => Box::new(special::MacAddrGen),
            GeneratorKind::PgMacAddr8 => Box::new(special::MacAddr8Gen),
            GeneratorKind::PgPoint => Box::new(special::PointGen),
            GeneratorKind::PgLine => Box::new(special::LineGen),
            GeneratorKind::PgLseg => Box::new(special::LsegGen),
            GeneratorKind::PgBox => Box::new(special::BoxGen),
            GeneratorKind::PgPath => Box::new(special::PathGen),
            GeneratorKind::PgPolygon => Box::new(special::PolygonGen),
            GeneratorKind::PgCircle => Box::new(special::CircleGen),
            GeneratorKind::PgBit => Box::new(special::BitGen {
                min_len: 8,
                max_len: 8,
            }),
            GeneratorKind::PgVarBit => Box::new(special::BitGen {
                min_len: 1,
                max_len: 32,
            }),
            GeneratorKind::PgArray => Box::new(special::ArrayGen),
        }
    }
}
