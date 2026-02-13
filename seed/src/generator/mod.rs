//! Value generators for seeding database columns.

pub mod numeric;
pub mod special;
pub mod string;
pub mod temporal;

/// A SQL-compatible value produced by a generator.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Blob(Vec<u8>),
}

impl SqlValue {
    /// Render as a SQL literal for use in INSERT statements.
    pub fn to_sql_literal(&self) -> String {
        match self {
            SqlValue::Null => "NULL".to_string(),
            SqlValue::Integer(v) => v.to_string(),
            SqlValue::Float(v) => format!("{v}"),
            SqlValue::Text(v) => format!("'{}'", v.replace('\'', "''")),
            SqlValue::Bool(true) => "1".to_string(),
            SqlValue::Bool(false) => "0".to_string(),
            SqlValue::Blob(b) => format!("X'{}'", hex(b)),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Trait for deterministic value generators.
///
/// Each generator produces a single column value given an RNG and a row index.
pub trait Generator: Send + Sync {
    /// Generate a value for row `index`.
    fn generate(&self, rng: &mut dyn RngCore, index: usize) -> SqlValue;

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
    /// Binary blob
    Blob,
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
            GeneratorKind::Blob => Box::new(special::BlobGen { size: 32 }),
        }
    }
}
