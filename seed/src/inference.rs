//! Infer the best generator for a column based on its SQL type and name.

use crate::generator::GeneratorKind;
use drizzle_core::SQLColumnInfo;

/// Select a generator kind for a column based on its type and name.
///
/// Priority: column name heuristics > SQL type mapping.
pub fn infer_generator(col: &dyn SQLColumnInfo) -> GeneratorKind {
    let name = col.name().to_lowercase();
    let sql_type = col.r#type().to_uppercase();

    // Primary key auto-increment
    if col.is_primary_key() && is_integer_type(&sql_type) {
        return GeneratorKind::IntPrimaryKey;
    }

    // Name-based heuristics (higher priority)
    if let Some(kind) = infer_from_name(&name) {
        return kind;
    }

    // SQL type mapping (fallback)
    infer_from_type(&sql_type)
}

fn infer_from_name(name: &str) -> Option<GeneratorKind> {
    // Order matters: more specific patterns first

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
        t if t.ends_with("[]") => GeneratorKind::PgArray,
        t if t.contains("SMALLINT") => GeneratorKind::Int,
        t if t.contains("INT") || t.contains("SERIAL") => GeneratorKind::Int,
        t if t.contains("REAL")
            || t.contains("FLOAT")
            || t.contains("DOUBLE")
            || t.contains("NUMERIC")
            || t.contains("DECIMAL") =>
        {
            GeneratorKind::Float
        }
        t if t.contains("UUID") => GeneratorKind::Uuid,
        t if t.contains("JSONB") || t.contains("JSON") => GeneratorKind::Json,
        t if t.contains("BYTEA") || t.contains("BLOB") => GeneratorKind::Blob,
        t if t.contains("BOOL") => GeneratorKind::Bool,
        t if t.contains("TIMESTAMPTZ") => GeneratorKind::Timestamp,
        t if t.contains("TIMESTAMP") || t.contains("DATETIME") => GeneratorKind::Timestamp,
        t if t.contains("TIMETZ") => GeneratorKind::TimeTz,
        t if t.contains("DATE") && !t.contains("TIME") => GeneratorKind::Date,
        t if t.contains("TIME") && !t.contains("STAMP") && !t.contains("DATE") => {
            GeneratorKind::Time
        }
        t if t.contains("INTERVAL") => GeneratorKind::Interval,
        t if t.contains("INET") => GeneratorKind::PgInet,
        t if t.contains("CIDR") => GeneratorKind::PgCidr,
        t if t.contains("MACADDR8") => GeneratorKind::PgMacAddr8,
        t if t.contains("MACADDR") => GeneratorKind::PgMacAddr,
        t if t.contains("POINT") => GeneratorKind::PgPoint,
        t if t.contains("LSEG") => GeneratorKind::PgLseg,
        t if t.contains("LINE") => GeneratorKind::PgLine,
        t if t.contains("BOX") => GeneratorKind::PgBox,
        t if t.contains("PATH") => GeneratorKind::PgPath,
        t if t.contains("POLYGON") => GeneratorKind::PgPolygon,
        t if t.contains("CIRCLE") => GeneratorKind::PgCircle,
        t if t.contains("VARBIT") => GeneratorKind::PgVarBit,
        t if t.contains("BIT") => GeneratorKind::PgBit,
        // TEXT, VARCHAR, CHAR, etc.
        _ => GeneratorKind::Text,
    }
}

fn is_integer_type(sql_type: &str) -> bool {
    sql_type.contains("INT") || sql_type.contains("SERIAL")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_heuristics() {
        assert_eq!(infer_from_name("email"), Some(GeneratorKind::Email));
        assert_eq!(infer_from_name("user_email"), Some(GeneratorKind::Email));
        assert_eq!(
            infer_from_name("first_name"),
            Some(GeneratorKind::FirstName)
        );
        assert_eq!(
            infer_from_name("created_at"),
            Some(GeneratorKind::Timestamp)
        );
        assert_eq!(infer_from_name("is_active"), Some(GeneratorKind::Bool));
        assert_eq!(infer_from_name("some_field"), None);
    }

    #[test]
    fn type_mapping() {
        assert_eq!(infer_from_type("INTEGER"), GeneratorKind::Int);
        assert_eq!(infer_from_type("TEXT"), GeneratorKind::Text);
        assert_eq!(infer_from_type("BOOLEAN"), GeneratorKind::Bool);
        assert_eq!(infer_from_type("REAL"), GeneratorKind::Float);
        assert_eq!(infer_from_type("BLOB"), GeneratorKind::Blob);
        assert_eq!(infer_from_type("UUID"), GeneratorKind::Uuid);
        assert_eq!(infer_from_type("TIMESTAMP"), GeneratorKind::Timestamp);
    }
}
