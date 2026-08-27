//! Infer a deterministic generator from column metadata.

use crate::generator::{Generator, GeneratorKind};
use drizzle_core::ColumnRef;

#[cfg(feature = "mysql")]
use crate::generator::{RngCore, SeedValue, numeric::IntGen, special::BlobGen, string::TextGen};
#[cfg(feature = "mysql")]
use drizzle_core::ColumnDialect;
#[cfg(feature = "mysql")]
use drizzle_types::mysql::MySQLTypeCategory;
#[cfg(feature = "mysql")]
use rand::Rng;

/// Build the most specific generator supported by the column domain.
pub(crate) fn infer_generator(column: &ColumnRef) -> Box<dyn Generator> {
    let sql_type = column.sql_type.to_uppercase();
    if column.primary_key() && is_integer_type(&sql_type) {
        return GeneratorKind::IntPrimaryKey.into_generator();
    }

    #[cfg(feature = "mysql")]
    if matches!(column.dialect, ColumnDialect::MySQL { .. })
        && let Some(generator) = infer_mysql_generator(column)
    {
        return generator;
    }

    let name = column.name.to_lowercase();
    infer_from_name(&name)
        .unwrap_or_else(|| infer_from_type(&sql_type))
        .into_generator()
}

#[cfg(feature = "mysql")]
fn infer_mysql_generator(column: &ColumnRef) -> Option<Box<dyn Generator>> {
    let category = MySQLTypeCategory::classify(column.sql_type);
    let integer_max = match category {
        MySQLTypeCategory::TinyInt => Some(i64::from(i8::MAX)),
        MySQLTypeCategory::TinyIntUnsigned => Some(i64::from(u8::MAX)),
        MySQLTypeCategory::SmallInt => Some(i64::from(i16::MAX)),
        MySQLTypeCategory::SmallIntUnsigned => Some(i64::from(u16::MAX)),
        MySQLTypeCategory::MediumInt => Some(8_388_607),
        MySQLTypeCategory::MediumIntUnsigned => Some(16_777_215),
        MySQLTypeCategory::Int => Some(i64::from(i32::MAX)),
        MySQLTypeCategory::IntUnsigned => Some(i64::from(u32::MAX)),
        MySQLTypeCategory::BigInt | MySQLTypeCategory::BigIntUnsigned => Some(i64::MAX),
        _ => None,
    };
    if let Some(max) = integer_max {
        return Some(Box::new(IntGen {
            min: 0,
            max: max.min(10_000),
        }));
    }

    match category {
        MySQLTypeCategory::Enum => mysql_inline_labels(column.sql_type, "ENUM")
            .map(|values| Box::new(ChoiceGen { values }) as Box<dyn Generator>),
        MySQLTypeCategory::Set => mysql_inline_labels(column.sql_type, "SET").map(|mut values| {
            // MySQL serializes SET values as a comma-separated string, so a
            // member containing a comma cannot round-trip unambiguously.
            values.retain(|value| !value.contains(','));
            Box::new(SetGen { values }) as Box<dyn Generator>
        }),
        MySQLTypeCategory::Year => Some(Box::new(YearGen)),
        MySQLTypeCategory::Boolean => Some(GeneratorKind::Bool.into_generator()),
        MySQLTypeCategory::Date => Some(GeneratorKind::Date.into_generator()),
        MySQLTypeCategory::Time => Some(GeneratorKind::Time.into_generator()),
        MySQLTypeCategory::DateTime | MySQLTypeCategory::Timestamp => {
            Some(GeneratorKind::Timestamp.into_generator())
        }
        MySQLTypeCategory::Decimal => {
            let (precision, scale) = mysql_numeric_args(column.sql_type).unwrap_or((10, 0));
            Some(Box::new(DecimalGen { precision, scale }))
        }
        MySQLTypeCategory::Float | MySQLTypeCategory::Double => {
            Some(GeneratorKind::Float.into_generator())
        }
        MySQLTypeCategory::Char => Some(mysql_text_generator(
            mysql_first_numeric_arg(column.sql_type).unwrap_or(1),
        )),
        MySQLTypeCategory::Varchar => mysql_first_numeric_arg(column.sql_type)
            .filter(|length| *length < 50)
            .map(mysql_text_generator),
        MySQLTypeCategory::Bit => Some(Box::new(BitGen {
            bits: mysql_first_numeric_arg(column.sql_type).unwrap_or(1),
        })),
        MySQLTypeCategory::Binary => Some(mysql_blob_generator(
            mysql_first_numeric_arg(column.sql_type).unwrap_or(1),
        )),
        MySQLTypeCategory::Varbinary => {
            mysql_first_numeric_arg(column.sql_type).map(mysql_blob_generator)
        }
        MySQLTypeCategory::TinyBlob
        | MySQLTypeCategory::Blob
        | MySQLTypeCategory::MediumBlob
        | MySQLTypeCategory::LongBlob => Some(GeneratorKind::Blob.into_generator()),
        MySQLTypeCategory::Json => Some(GeneratorKind::Json.into_generator()),
        _ => None,
    }
}

#[cfg(feature = "mysql")]
fn mysql_text_generator(length: usize) -> Box<dyn Generator> {
    Box::new(TextGen {
        min_len: length.min(5),
        max_len: length,
    })
}

#[cfg(feature = "mysql")]
fn mysql_blob_generator(length: usize) -> Box<dyn Generator> {
    Box::new(BlobGen {
        size: length.min(32),
    })
}

#[cfg(feature = "mysql")]
fn mysql_first_numeric_arg(sql_type: &str) -> Option<usize> {
    let open = sql_type.find('(')?;
    let close = sql_type[open + 1..].find(')')? + open + 1;
    sql_type[open + 1..close]
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

#[cfg(feature = "mysql")]
fn mysql_numeric_args(sql_type: &str) -> Option<(usize, usize)> {
    let open = sql_type.find('(')?;
    let close = sql_type[open + 1..].find(')')? + open + 1;
    let mut args = sql_type[open + 1..close].split(',');
    let precision = args.next()?.trim().parse().ok()?;
    let scale = args
        .next()
        .map_or(Some(0), |value| value.trim().parse().ok())?;
    (precision > 0 && scale <= precision).then_some((precision, scale))
}

#[cfg(feature = "mysql")]
pub(crate) fn mysql_inline_labels(sql_type: &str, keyword: &str) -> Option<Vec<String>> {
    let declaration = sql_type.trim();
    let open = declaration.find('(')?;
    if !declaration[..open].trim().eq_ignore_ascii_case(keyword) || !declaration.ends_with(')') {
        return None;
    }

    let mut values = Vec::new();
    let mut chars = declaration[open + 1..declaration.len() - 1]
        .chars()
        .peekable();
    loop {
        while chars
            .peek()
            .is_some_and(|character| character.is_whitespace() || *character == ',')
        {
            chars.next();
        }
        let quote = chars.next()?;
        if quote != '\'' {
            return None;
        }

        let mut value = String::new();
        let mut closed = false;
        while let Some(character) = chars.next() {
            match character {
                '\\' => value.push(chars.next()?),
                '\'' => {
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                        value.push('\'');
                    } else {
                        closed = true;
                        break;
                    }
                }
                _ => value.push(character),
            }
        }
        if !closed {
            return None;
        }
        values.push(value);

        while chars
            .peek()
            .is_some_and(|character| character.is_whitespace())
        {
            chars.next();
        }
        match chars.peek() {
            Some(',') => {
                chars.next();
            }
            None => break,
            Some(_) => return None,
        }
    }
    (!values.is_empty()).then_some(values)
}

#[cfg(feature = "mysql")]
struct ChoiceGen {
    values: Vec<String>,
}

#[cfg(feature = "mysql")]
impl Generator for ChoiceGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Text(self.values[rng.random_range(0..self.values.len())].clone())
    }

    fn name(&self) -> &'static str {
        "MySQLChoice"
    }
}

#[cfg(feature = "mysql")]
struct SetGen {
    values: Vec<String>,
}

#[cfg(feature = "mysql")]
impl Generator for SetGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        if self.values.is_empty() {
            return SeedValue::Text(String::new());
        }
        let count = rng.random_range(1..=self.values.len());
        SeedValue::Text(self.values[..count].join(","))
    }

    fn name(&self) -> &'static str {
        "MySQLSet"
    }
}

#[cfg(feature = "mysql")]
struct YearGen;

#[cfg(feature = "mysql")]
impl Generator for YearGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Integer(rng.random_range(1901..=2155))
    }

    fn name(&self) -> &'static str {
        "MySQLYear"
    }
}

#[cfg(feature = "mysql")]
struct DecimalGen {
    precision: usize,
    scale: usize,
}

#[cfg(feature = "mysql")]
impl Generator for DecimalGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let integer_digits = self.precision.saturating_sub(self.scale).min(6);
        let whole_limit = 10u64.pow(u32::try_from(integer_digits).unwrap_or(6));
        let whole = if integer_digits == 0 {
            0
        } else {
            rng.random_range(0..whole_limit)
        };
        if self.scale == 0 {
            return SeedValue::Text(whole.to_string());
        }

        let random_digits = self.scale.min(6);
        let fraction_limit = 10u64.pow(u32::try_from(random_digits).unwrap_or(6));
        let fraction = rng.random_range(0..fraction_limit);
        let mut value = format!("{whole}.{fraction:0random_digits$}");
        value.extend(std::iter::repeat_n('0', self.scale - random_digits));
        SeedValue::Text(value)
    }

    fn name(&self) -> &'static str {
        "MySQLDecimal"
    }
}

#[cfg(feature = "mysql")]
struct BitGen {
    bits: usize,
}

#[cfg(feature = "mysql")]
impl Generator for BitGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let byte_count = self.bits.div_ceil(8);
        let mut bytes = vec![0; byte_count];
        rng.fill_bytes(&mut bytes);
        if let Some(first) = bytes.first_mut() {
            let unused_bits = byte_count * 8 - self.bits;
            *first &= u8::MAX >> unused_bits;
        }
        SeedValue::Blob(bytes)
    }

    fn name(&self) -> &'static str {
        "MySQLBit"
    }
}

fn infer_from_name(name: &str) -> Option<GeneratorKind> {
    if name.contains("email") || name.contains("e_mail") {
        return Some(GeneratorKind::Email);
    }
    if name.contains("phone") || name.contains("tel") || name.contains("mobile") {
        return Some(GeneratorKind::Phone);
    }
    if name.contains("first_name") || name.contains("fname") || name.contains("given_name") {
        return Some(GeneratorKind::FirstName);
    }
    if name.contains("last_name")
        || name.contains("lname")
        || name.contains("surname")
        || name.contains("family_name")
    {
        return Some(GeneratorKind::LastName);
    }
    if name == "name"
        || name.contains("full_name")
        || name.contains("display_name")
        || name.contains("username")
    {
        return Some(GeneratorKind::FullName);
    }
    if name.contains("city") || name.contains("town") {
        return Some(GeneratorKind::City);
    }
    if name.contains("country") || name.contains("nation") {
        return Some(GeneratorKind::Country);
    }
    if name.contains("address") || name.contains("street") {
        return Some(GeneratorKind::Address);
    }
    if name.contains("job")
        || name.contains("title")
        || name.contains("position")
        || name.contains("role")
    {
        return Some(GeneratorKind::JobTitle);
    }
    if name.contains("company") || name.contains("org") || name.contains("employer") {
        return Some(GeneratorKind::Company);
    }
    if name.contains("description")
        || name.contains("bio")
        || name.contains("about")
        || name.contains("summary")
        || name.contains("content")
        || name.contains("body")
    {
        return Some(GeneratorKind::LoremIpsum);
    }
    if name.contains("uuid") || name.contains("guid") {
        return Some(GeneratorKind::Uuid);
    }
    if name.contains("json")
        || name.contains("data")
        || name.contains("metadata")
        || name.contains("payload")
    {
        return Some(GeneratorKind::Json);
    }
    if name.contains("date") || name.contains("birthday") || name.contains("dob") {
        return Some(GeneratorKind::Date);
    }
    if name.contains("timestamp")
        || name.contains("created_at")
        || name.contains("updated_at")
        || name.contains("deleted_at")
    {
        return Some(GeneratorKind::Timestamp);
    }
    if name.contains("time") && !name.contains("timestamp") {
        return Some(GeneratorKind::Time);
    }
    if name.contains("active")
        || name.contains("enabled")
        || name.contains("is_")
        || name.contains("has_")
        || name.contains("verified")
        || name.contains("approved")
    {
        return Some(GeneratorKind::Bool);
    }
    None
}

fn infer_from_type(sql_type: &str) -> GeneratorKind {
    match sql_type {
        value if value.ends_with("[]") => GeneratorKind::PgArray,
        value if value.contains("INT") || value.contains("SERIAL") => GeneratorKind::Int,
        value
            if value.contains("REAL")
                || value.contains("FLOAT")
                || value.contains("DOUBLE")
                || value.contains("NUMERIC")
                || value.contains("DECIMAL") =>
        {
            GeneratorKind::Float
        }
        value if value.contains("UUID") => GeneratorKind::Uuid,
        value if value.contains("JSONB") || value.contains("JSON") => GeneratorKind::Json,
        value if value.contains("BYTEA") || value.contains("BLOB") => GeneratorKind::Blob,
        value if value.contains("BOOL") => GeneratorKind::Bool,
        value if value.contains("TIMESTAMPTZ") => GeneratorKind::Timestamp,
        value if value.contains("TIMESTAMP") || value.contains("DATETIME") => {
            GeneratorKind::Timestamp
        }
        value if value.contains("TIMETZ") => GeneratorKind::TimeTz,
        value if value.contains("DATE") && !value.contains("TIME") => GeneratorKind::Date,
        value if value.contains("TIME") && !value.contains("STAMP") && !value.contains("DATE") => {
            GeneratorKind::Time
        }
        value if value.contains("INTERVAL") => GeneratorKind::Interval,
        value if value.contains("INET") => GeneratorKind::PgInet,
        value if value.contains("CIDR") => GeneratorKind::PgCidr,
        value if value.contains("MACADDR8") => GeneratorKind::PgMacAddr8,
        value if value.contains("MACADDR") => GeneratorKind::PgMacAddr,
        value if value.contains("POINT") => GeneratorKind::PgPoint,
        value if value.contains("LSEG") => GeneratorKind::PgLseg,
        value if value.contains("LINE") => GeneratorKind::PgLine,
        value if value.contains("BOX") => GeneratorKind::PgBox,
        value if value.contains("PATH") => GeneratorKind::PgPath,
        value if value.contains("POLYGON") => GeneratorKind::PgPolygon,
        value if value.contains("CIRCLE") => GeneratorKind::PgCircle,
        value if value.contains("VARBIT") => GeneratorKind::PgVarBit,
        value if value.contains("BIT") => GeneratorKind::PgBit,
        _ => GeneratorKind::Text,
    }
}

fn is_integer_type(sql_type: &str) -> bool {
    sql_type.contains("INT") || sql_type.contains("SERIAL")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "mysql")]
    use drizzle_core::{ColumnDialect, ColumnFlags};
    #[cfg(feature = "mysql")]
    use rand::SeedableRng;

    #[cfg(feature = "mysql")]
    fn mysql_column(sql_type: &'static str) -> ColumnRef {
        ColumnRef {
            table: "items",
            name: "value",
            sql_type,
            flags: ColumnFlags::empty(),
            dialect: ColumnDialect::MySQL {
                auto_increment: false,
                default: None,
                generated_expression: None,
                generated_stored: false,
                charset: None,
                collate: None,
                on_update: None,
            },
        }
    }

    #[test]
    fn name_heuristics() {
        assert_eq!(infer_from_name("email"), Some(GeneratorKind::Email));
        assert_eq!(
            infer_from_name("created_at"),
            Some(GeneratorKind::Timestamp)
        );
        assert_eq!(infer_from_name("some_field"), None);
    }

    #[test]
    fn type_mapping() {
        assert_eq!(infer_from_type("INTEGER"), GeneratorKind::Int);
        assert_eq!(infer_from_type("TEXT"), GeneratorKind::Text);
        assert_eq!(infer_from_type("BOOLEAN"), GeneratorKind::Bool);
        assert_eq!(infer_from_type("TIMESTAMP"), GeneratorKind::Timestamp);
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn parses_mysql_inline_labels() {
        assert_eq!(
            mysql_inline_labels("ENUM('Draft', 'Published')", "ENUM").unwrap(),
            ["Draft", "Published"]
        );
        assert_eq!(
            mysql_inline_labels("SET('read,only', 'writer')", "SET").unwrap(),
            ["read,only", "writer"]
        );
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_set_generator_avoids_ambiguous_comma_bearing_members() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let generator = infer_generator(&mysql_column("SET('read,only')"));
        assert_eq!(
            generator.generate(&mut rng, 0, "SET('read,only')"),
            SeedValue::Text(String::new())
        );
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn parses_mysql_numeric_declaration_arguments() {
        assert_eq!(mysql_numeric_args("DECIMAL(8, 3)"), Some((8, 3)));
        assert_eq!(mysql_first_numeric_arg("VARCHAR(32)"), Some(32));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_char_and_binary_without_lengths_default_to_one_byte() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let char_value = infer_generator(&mysql_column("CHAR")).generate(&mut rng, 0, "CHAR");
        let binary_value = infer_generator(&mysql_column("BINARY")).generate(&mut rng, 0, "BINARY");

        assert!(matches!(char_value, SeedValue::Text(value) if value.len() == 1));
        assert!(matches!(binary_value, SeedValue::Blob(value) if value.len() == 1));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_decimal_generator_respects_precision_and_scale() {
        let generator = DecimalGen {
            precision: 8,
            scale: 3,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        for index in 0..100 {
            let SeedValue::Text(value) = generator.generate(&mut rng, index, "DECIMAL(8,3)") else {
                panic!("decimal generator returned a non-text value");
            };
            let (whole, fraction) = value.split_once('.').expect("decimal point");
            assert!(whole.len() <= 5);
            assert_eq!(fraction.len(), 3);
            assert!(value.chars().filter(|char| char.is_ascii_digit()).count() <= 8);
        }
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_bit_generator_masks_unused_high_bits() {
        let generator = BitGen { bits: 9 };
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        for index in 0..100 {
            let SeedValue::Blob(bytes) = generator.generate(&mut rng, index, "BIT(9)") else {
                panic!("bit generator returned a non-blob value");
            };
            assert_eq!(bytes.len(), 2);
            assert!(bytes[0] <= 1);
        }
    }
}
