//! Parser combinators using nom
//!
//! Shared parsing logic for all dialects.

use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::{opt, recognize},
    multi::many0,
    sequence::{delimited, pair, preceded},
};

use super::types::{ParsedField, ParsedIndex, ParsedSchema, ParsedTable};
use std::collections::HashMap;

// =============================================================================
// Basic Combinators
// =============================================================================

/// Parse a Rust identifier (alphanumeric + underscore, starting with letter or _)
fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while1(|c: char| c.is_alphabetic() || c == '_'),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    ))
    .parse(input)
}

/// Parse content inside balanced delimiters, handling nesting
fn balanced_content(open: char, close: char) -> impl Fn(&str) -> IResult<&str, &str> {
    move |input: &str| {
        let mut depth = 0;
        let mut end_pos = 0;

        for (i, c) in input.char_indices() {
            if c == open {
                depth += 1;
            } else if c == close {
                if depth == 0 {
                    end_pos = i;
                    break;
                }
                depth -= 1;
            }
            end_pos = i + c.len_utf8();
        }

        if end_pos == 0 && !input.is_empty() {
            end_pos = input.len();
        }

        Ok((&input[end_pos..], &input[..end_pos]))
    }
}

/// Parse an attribute like #[column(primary, default = 42)]
fn parse_attribute(input: &str) -> IResult<&str, &str> {
    recognize((
        tag("#["),
        take_while1(|c: char| c.is_alphanumeric() || c == '_'),
        opt(delimited(char('('), balanced_content('(', ')'), char(')'))),
        char(']'),
    ))
    .parse(input)
}

/// Parse a Rust type (handles generics like Option<String>, Vec<u8>)
fn parse_type(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        identifier,
        opt(delimited(char('<'), balanced_content('<', '>'), char('>'))),
    ))
    .parse(input)
}

// =============================================================================
// Field Parsing
// =============================================================================

/// Parse a struct field with optional attributes
fn parse_field(input: &str) -> IResult<&str, ParsedField> {
    let (input, _) = multispace0.parse(input)?;

    // Collect all #[...] attributes before the field
    let (input, attrs) = many0(preceded(multispace0, parse_attribute)).parse(input)?;

    let (input, _) = multispace0.parse(input)?;

    // Optional `pub`
    let (input, _) = opt(pair(tag("pub"), multispace1)).parse(input)?;

    // Field name
    let (input, name) = identifier(input)?;

    let (input, _) = multispace0.parse(input)?;
    let (input, _) = char(':').parse(input)?;
    let (input, _) = multispace0.parse(input)?;

    // Field type
    let (input, ty) = parse_type(input)?;

    // Optional trailing comma
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = opt(char(',')).parse(input)?;

    Ok((
        input,
        ParsedField {
            name: name.to_string(),
            ty: ty.to_string(),
            attrs: attrs.iter().map(|s| s.to_string()).collect(),
        },
    ))
}

// =============================================================================
// Table Parsing
// =============================================================================

/// Parse a table struct: #[SQLiteTable(...)] struct Name { fields... }
pub fn parse_table_struct(input: &str) -> IResult<&str, ParsedTable> {
    // Parse the table attribute
    let (input, attr) = parse_attribute(input)?;

    let (input, _) = multispace0.parse(input)?;

    // Optional `pub`
    let (input, _) = opt(pair(tag("pub"), multispace1)).parse(input)?;

    // `struct`
    let (input, _) = tag("struct").parse(input)?;
    let (input, _) = multispace1.parse(input)?;

    // Struct name
    let (input, name) = identifier(input)?;

    let (input, _) = multispace0.parse(input)?;

    // Opening brace
    let (input, _) = char('{').parse(input)?;

    // Parse fields until closing brace
    let (input, fields_content) = take_until("}").parse(input)?;
    let (input, _) = char('}').parse(input)?;

    // Parse individual fields from the content
    let mut fields = Vec::new();
    let mut remaining = fields_content;
    while !remaining.trim().is_empty() {
        // Skip comments and whitespace
        let trimmed = remaining.trim_start();
        if trimmed.starts_with("//") {
            // Skip to end of line
            if let Some(nl) = trimmed.find('\n') {
                remaining = &trimmed[nl + 1..];
                continue;
            } else {
                break;
            }
        }

        if trimmed.is_empty() {
            break;
        }

        match parse_field(trimmed) {
            Ok((rest, field)) => {
                fields.push(field);
                remaining = rest;
            }
            Err(_) => {
                // Skip this line and try the next
                if let Some(nl) = trimmed.find('\n') {
                    remaining = &trimmed[nl + 1..];
                } else {
                    break;
                }
            }
        }
    }

    Ok((
        input,
        ParsedTable {
            name: name.to_string(),
            attr: attr.to_string(),
            fields,
            dialect: drizzle_types::Dialect::default(),
        },
    ))
}

// =============================================================================
// Index Parsing
// =============================================================================

/// Parse an index struct: #[SQLiteIndex(unique)] struct IdxName(Table::col1, Table::col2);
pub fn parse_index_struct(input: &str) -> IResult<&str, ParsedIndex> {
    // Parse the index attribute
    let (input, attr) = parse_attribute(input)?;

    let (input, _) = multispace0.parse(input)?;

    // Optional `pub`
    let (input, _) = opt(pair(tag("pub"), multispace1)).parse(input)?;

    // `struct`
    let (input, _) = tag("struct").parse(input)?;
    let (input, _) = multispace1.parse(input)?;

    // Struct name
    let (input, name) = identifier(input)?;

    let (input, _) = multispace0.parse(input)?;

    // Opening paren
    let (input, _) = char('(').parse(input)?;

    // Column references
    let (input, cols_content) = take_until(")").parse(input)?;
    let (input, _) = char(')').parse(input)?;

    // Parse columns (comma-separated path expressions)
    let columns: Vec<String> = cols_content
        .split(',')
        .map(|s| s.trim().trim_end_matches(';').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Optional semicolon
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = opt(char(';')).parse(input)?;

    Ok((
        input,
        ParsedIndex {
            name: name.to_string(),
            attr: attr.to_string(),
            columns,
            dialect: drizzle_types::Dialect::default(),
        },
    ))
}

// =============================================================================
// Schema Parsing
// =============================================================================

/// Parse a schema struct: #[derive(SQLiteSchema)] struct Name { members... }
pub fn parse_schema_struct(input: &str) -> IResult<&str, ParsedSchema> {
    // Skip to after the derive attribute
    let (input, _) = take_until("struct").parse(input)?;

    // Optional `pub`
    let (input, _) = opt(pair(tag("pub"), multispace1)).parse(input)?;

    // `struct`
    let (input, _) = tag("struct").parse(input)?;
    let (input, _) = multispace1.parse(input)?;

    // Struct name
    let (input, name) = identifier(input)?;

    let (input, _) = multispace0.parse(input)?;

    // Opening brace
    let (input, _) = char('{').parse(input)?;

    // Parse fields until closing brace
    let (input, fields_content) = take_until("}").parse(input)?;
    let (input, _) = char('}').parse(input)?;

    // Parse members
    let mut members = HashMap::new();
    for line in fields_content.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some(colon) = line.find(':') {
            let name_part = line[..colon]
                .trim()
                .strip_prefix("pub ")
                .unwrap_or(line[..colon].trim());
            let type_part = line[colon + 1..].trim();
            members.insert(name_part.to_string(), type_part.to_string());
        }
    }

    Ok((
        input,
        ParsedSchema {
            name: name.to_string(),
            members,
            dialect: drizzle_types::Dialect::default(),
        },
    ))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_identifier() {
        assert_eq!(identifier("foo_bar"), Ok(("", "foo_bar")));
        assert_eq!(identifier("FooBar"), Ok(("", "FooBar")));
        assert_eq!(identifier("_private"), Ok(("", "_private")));
    }

    #[test]
    fn test_parse_type() {
        assert_eq!(parse_type("i64"), Ok(("", "i64")));
        assert_eq!(parse_type("String"), Ok(("", "String")));
        assert_eq!(parse_type("Option<String>"), Ok(("", "Option<String>")));
        assert_eq!(parse_type("Vec<u8>"), Ok(("", "Vec<u8>")));
    }

    #[test]
    fn test_parse_attribute() {
        assert_eq!(
            parse_attribute("#[column(primary)]"),
            Ok(("", "#[column(primary)]"))
        );
        assert_eq!(
            parse_attribute("#[SQLiteTable]"),
            Ok(("", "#[SQLiteTable]"))
        );
        assert_eq!(
            parse_attribute("#[column(default = 42)]"),
            Ok(("", "#[column(default = 42)]"))
        );
    }

    #[test]
    fn test_parse_field() {
        let (_, field) = parse_field("id: i64,").unwrap();
        assert_eq!(field.name, "id");
        assert_eq!(field.ty, "i64");

        let (_, field) = parse_field("pub name: String,").unwrap();
        assert_eq!(field.name, "name");
        assert_eq!(field.ty, "String");

        let (_, field) = parse_field("#[column(primary)]\n    id: i64,").unwrap();
        assert_eq!(field.name, "id");
        assert!(field.has_attr("primary"));
    }

    #[test]
    fn test_parse_table_struct() {
        let code = r#"#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
    name: String,
}"#;
        let (_, table) = parse_table_struct(code).unwrap();
        assert_eq!(table.name, "Users");
        assert_eq!(table.fields.len(), 2);
        assert!(table.fields[0].has_attr("primary"));
    }

    #[test]
    fn test_parse_index_struct() {
        let code = "#[SQLiteIndex(unique)]\nstruct IdxUsersEmail(Users::email);";
        let (_, index) = parse_index_struct(code).unwrap();
        assert_eq!(index.name, "IdxUsersEmail");
        assert!(index.attr.contains("unique"));
        assert_eq!(index.columns, vec!["Users::email"]);
    }

    #[test]
    fn test_nullable_detection() {
        let (_, field) = parse_field("email: Option<String>,").unwrap();
        assert!(field.is_nullable());
        assert_eq!(field.ty, "Option<String>");

        let (_, field) = parse_field("email: String,").unwrap();
        assert!(!field.is_nullable());
        assert_eq!(field.ty, "String");
    }
}
