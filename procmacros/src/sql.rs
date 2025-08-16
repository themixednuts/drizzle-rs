use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Expr, LitStr, Result,
    parse::{Parse, ParseStream},
};

/// Input for the sql! procedural macro
pub struct SqlInput {
    pub template: LitStr,
}

impl Parse for SqlInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let template = input.parse::<LitStr>()?;
        Ok(SqlInput { template })
    }
}

/// A parsed segment of the SQL template
#[derive(Debug, Clone)]
enum SqlSegment {
    /// Raw SQL text that becomes SQL::text()
    Text(String),
    /// An expression inside {braces} that should have .to_sql() called on it
    Expression(String),
}

/// Parse the template string into text and expression segments
fn parse_template(template: &str) -> Result<Vec<SqlSegment>> {
    let mut segments = Vec::new();
    let mut chars = template.chars().peekable();
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                // Check for escaped brace {{
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume second {
                    current_text.push('{');
                    continue;
                }

                // Add any accumulated text as a text segment
                if !current_text.is_empty() {
                    segments.push(SqlSegment::Text(current_text.clone()));
                    current_text.clear();
                }

                // Parse the expression inside braces
                let mut expr_content = String::new();
                let mut brace_count = 1;

                while let Some(inner_ch) = chars.next() {
                    match inner_ch {
                        '{' => {
                            brace_count += 1;
                            expr_content.push(inner_ch);
                        }
                        '}' => {
                            brace_count -= 1;
                            if brace_count == 0 {
                                break;
                            }
                            expr_content.push(inner_ch);
                        }
                        _ => expr_content.push(inner_ch),
                    }
                }

                if brace_count != 0 {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "Unmatched braces in SQL template",
                    ));
                }

                if expr_content.is_empty() {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "Empty expression in SQL template",
                    ));
                }

                segments.push(SqlSegment::Expression(expr_content));
            }
            '}' => {
                // Check for escaped brace }}
                if chars.peek() == Some(&'}') {
                    chars.next(); // consume second }
                    current_text.push('}');
                    continue;
                }

                return Err(syn::Error::new(
                    Span::call_site(),
                    "Unmatched closing brace in SQL template",
                ));
            }
            _ => current_text.push(ch),
        }
    }

    // Add any remaining text
    if !current_text.is_empty() {
        segments.push(SqlSegment::Text(current_text));
    }

    Ok(segments)
}

/// Generate the TokenStream for the sql! macro implementation
pub fn sql_impl(input: SqlInput) -> Result<TokenStream> {
    let template_str = input.template.value();
    let segments = parse_template(&template_str)?;

    if segments.is_empty() {
        return Ok(quote! {
            drizzle_core::SQL::empty()
        });
    }

    // Generate code for each segment
    let mut segment_tokens = Vec::new();

    for segment in segments {
        match segment {
            SqlSegment::Text(text) => {
                if !text.is_empty() {
                    segment_tokens.push(quote! {
                        drizzle_core::SQL::text(#text)
                    });
                }
            }
            SqlSegment::Expression(expr_str) => {
                // Parse the expression string as a Rust expression
                let expr: Expr = syn::parse_str(&expr_str).map_err(|e| {
                    syn::Error::new(
                        Span::call_site(),
                        format!("Invalid expression in SQL template: {}", e),
                    )
                })?;

                segment_tokens.push(quote! {
                    drizzle_core::ToSQL::to_sql(&#expr)
                });
            }
        }
    }

    // If we only have one segment, return it directly
    if segment_tokens.len() == 1 {
        return Ok(segment_tokens.into_iter().next().unwrap());
    }

    // Chain multiple segments together with .append()
    let mut iter = segment_tokens.into_iter();
    let mut result = iter.next().unwrap();

    for segment in iter {
        result = quote! {
            #result.append(#segment)
        };
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_template() {
        let segments = parse_template("SELECT * FROM {users}").unwrap();
        assert_eq!(segments.len(), 2);

        match &segments[0] {
            SqlSegment::Text(text) => assert_eq!(text, "SELECT * FROM "),
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(expr) => assert_eq!(expr, "users"),
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_parse_multiple_expressions() {
        let segments = parse_template("SELECT {column} FROM {table} WHERE {condition}").unwrap();
        assert_eq!(segments.len(), 5);

        match &segments[0] {
            SqlSegment::Text(text) => assert_eq!(text, "SELECT "),
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(expr) => assert_eq!(expr, "column"),
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_parse_escaped_braces() {
        let segments = parse_template("SELECT {{literal}} FROM {table}").unwrap();
        assert_eq!(segments.len(), 2);

        match &segments[0] {
            SqlSegment::Text(text) => assert_eq!(text, "SELECT {literal} FROM "),
            _ => panic!("Expected text segment"),
        }
    }

    #[test]
    fn test_parse_nested_braces() {
        let segments = parse_template("SELECT {func({inner})} FROM {table}").unwrap();
        assert_eq!(segments.len(), 3);

        match &segments[1] {
            SqlSegment::Expression(expr) => assert_eq!(expr, "func({inner})"),
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_unmatched_braces() {
        assert!(parse_template("SELECT {unclosed FROM table").is_err());
        assert!(parse_template("SELECT closed} FROM table").is_err());
    }
}
