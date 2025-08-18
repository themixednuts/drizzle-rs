use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::quote;
use syn::{
    Expr, LitStr, Result,
    parse::{Parse, ParseStream},
};

/// Input for the sql! procedural macro
pub enum SqlInput {
    /// String literal input: sql!("SELECT * FROM {table}")
    StringLiteral(LitStr),
    /// Token stream input: sql!(SELECT * FROM {table})
    TokenStream(TokenStream),
    /// Printf-style input: sql!("SELECT * FROM {} WHERE {} = {}", table, column, value)
    Printf { template: LitStr, args: Vec<Expr> },
}

impl Parse for SqlInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // Try to parse as string literal first
        if input.peek(LitStr) {
            let template = input.parse::<LitStr>()?;

            // Check if there are comma-separated arguments after the template
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?; // consume comma

                let mut args = Vec::new();

                // Parse first argument
                if !input.is_empty() {
                    args.push(input.parse::<Expr>()?);

                    // Parse remaining arguments
                    while input.peek(syn::Token![,]) {
                        input.parse::<syn::Token![,]>()?; // consume comma
                        if !input.is_empty() {
                            args.push(input.parse::<Expr>()?);
                        }
                    }
                }

                Ok(SqlInput::Printf { template, args })
            } else {
                Ok(SqlInput::StringLiteral(template))
            }
        } else {
            // Parse the rest as a token stream
            let tokens: TokenStream = input.parse()?;
            Ok(SqlInput::TokenStream(tokens))
        }
    }
}

/// A parsed segment of the SQL template
#[derive(Clone)]
enum SqlSegment {
    /// Raw SQL text that becomes SQL::text()
    Text(String),
    /// An expression inside {braces} that should have .to_sql() called on it
    Expression(Expr),
}

impl std::fmt::Debug for SqlSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlSegment::Text(text) => f.debug_tuple("Text").field(text).finish(),
            SqlSegment::Expression(_) => f.debug_tuple("Expression").field(&"<expr>").finish(),
        }
    }
}

/// Parse token stream into text and expression segments
/// This converts the token stream back to a string representation and then uses the string parser
fn parse_token_stream(tokens: TokenStream) -> Result<Vec<SqlSegment>> {
    // Convert token stream back to string with proper spacing
    let mut sql_string = String::new();
    let mut tokens_iter = tokens.into_iter().peekable();

    while let Some(token) = tokens_iter.next() {
        match token {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                // Add space before brace if needed
                if !sql_string.is_empty() && !sql_string.ends_with(' ') {
                    sql_string.push(' ');
                }
                sql_string.push('{');
                sql_string.push_str(&group.stream().to_string());
                sql_string.push('}');
            }
            _ => {
                // Add space before token if needed
                if !sql_string.is_empty() {
                    match &token {
                        TokenTree::Punct(p) if p.as_char() == '.' => {
                            // No space before dots
                        }
                        _ => {
                            sql_string.push(' ');
                        }
                    }
                }

                sql_string.push_str(&token.to_string());

                // Add space after token if needed
                if let Some(next_token) = tokens_iter.peek() {
                    match (&token, next_token) {
                        (TokenTree::Punct(p), _) if p.as_char() == '.' => {
                            // No space after dots
                        }
                        (_, TokenTree::Punct(p)) if p.as_char() == '.' => {
                            // No space before dots
                        }
                        (_, TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                            // No space before brace groups
                        }
                        (TokenTree::Punct(p), _) if matches!(p.as_char(), '=' | '<' | '>') => {
                            // Space after operators
                            sql_string.push(' ');
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Now use the string parser which handles spacing correctly
    parse_template(&sql_string)
}

/// Parse the template string into text and expression segments
/// If `positional_args` is provided, empty braces {} will be replaced with the arguments
fn parse_template_with_args(
    template: &str,
    positional_args: Option<&[Expr]>,
) -> Result<Vec<SqlSegment>> {
    let mut segments = Vec::new();
    let mut chars = template.chars().peekable();
    let mut current_text = String::new();
    let mut arg_index = 0;

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                // Check for escaped brace {{
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume second {
                    current_text.push('{');
                    continue;
                }

                // Add any accumulated text as a text segment (preserve exact spacing)
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

                // Handle empty braces for positional arguments
                if expr_content.is_empty() {
                    if let Some(args) = positional_args {
                        if arg_index >= args.len() {
                            return Err(syn::Error::new(
                                Span::call_site(),
                                format!(
                                    "Not enough arguments provided. Expected at least {}, got {}",
                                    arg_index + 1,
                                    args.len()
                                ),
                            ));
                        }
                        segments.push(SqlSegment::Expression(args[arg_index].clone()));
                        arg_index += 1;
                    } else {
                        return Err(syn::Error::new(
                            Span::call_site(),
                            "Empty expression in SQL template",
                        ));
                    }
                } else {
                    // Named expression - parse it
                    let expr: Expr = syn::parse_str(&expr_content).map_err(|e| {
                        syn::Error::new(
                            Span::call_site(),
                            format!("Invalid expression in SQL template: {}", e),
                        )
                    })?;

                    segments.push(SqlSegment::Expression(expr));
                }
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

    // Add any remaining text (preserve exact spacing)
    if !current_text.is_empty() {
        segments.push(SqlSegment::Text(current_text));
    }

    // Check if we used all positional arguments
    if let Some(args) = positional_args {
        if arg_index != args.len() {
            return Err(syn::Error::new(
                Span::call_site(),
                format!(
                    "Too many arguments provided. Expected {}, got {}",
                    arg_index,
                    args.len()
                ),
            ));
        }
    }

    Ok(segments)
}

/// Parse the template string into text and expression segments (convenience wrapper)
fn parse_template(template: &str) -> Result<Vec<SqlSegment>> {
    parse_template_with_args(template, None)
}

/// Generate the TokenStream for the sql! macro implementation
pub fn sql_impl(input: SqlInput) -> Result<TokenStream> {
    let segments = match input {
        SqlInput::StringLiteral(template) => {
            let template_str = template.value();
            parse_template(&template_str)?
        }
        SqlInput::TokenStream(tokens) => parse_token_stream(tokens)?,
        SqlInput::Printf { template, args } => {
            let template_str = template.value();
            parse_template_with_args(&template_str, Some(&args))?
        }
    };

    if segments.is_empty() {
        return Ok(quote! {
            ::drizzle_rs::core::SQL::empty()
        });
    }

    // Generate code for each segment
    let mut segment_tokens = Vec::new();

    for segment in segments {
        match segment {
            SqlSegment::Text(text) => {
                if !text.is_empty() {
                    segment_tokens.push(quote! {
                        ::drizzle_rs::core::SQL::text(#text)
                    });
                }
            }
            SqlSegment::Expression(expr) => {
                segment_tokens.push(quote! {
                    ::drizzle_rs::core::ToSQL::to_sql(&#expr)
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
            SqlSegment::Expression(expr) => {
                // Check that it's a path expression with identifier "users"
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments.len(), 1);
                    assert_eq!(path.path.segments[0].ident.to_string(), "users");
                } else {
                    panic!("Expected path expression");
                }
            }
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_parse_multiple_expressions() {
        let segments = parse_template("SELECT {column} FROM {table} WHERE {condition}").unwrap();
        println!("Segments: {:#?}", segments);
        assert_eq!(segments.len(), 6);

        match &segments[0] {
            SqlSegment::Text(text) => assert_eq!(text, "SELECT "),
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "column");
                }
            }
            _ => panic!("Expected expression segment"),
        }

        match &segments[2] {
            SqlSegment::Text(text) => assert_eq!(text, " FROM "),
            _ => panic!("Expected text segment"),
        }

        match &segments[3] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "table");
                }
            }
            _ => panic!("Expected expression segment"),
        }

        match &segments[4] {
            SqlSegment::Text(text) => assert_eq!(text, " WHERE "),
            _ => panic!("Expected text segment"),
        }

        match &segments[5] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "condition");
                }
            }
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
        println!("Nested segments: {:#?}", segments);
        assert_eq!(segments.len(), 4);

        match &segments[0] {
            SqlSegment::Text(text) => assert_eq!(text, "SELECT "),
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(_expr) => {
                // This would be a function call expression - just verify it parsed
                // The exact structure is complex for function calls
            }
            _ => panic!("Expected expression segment"),
        }

        match &segments[2] {
            SqlSegment::Text(text) => assert_eq!(text, " FROM "),
            _ => panic!("Expected text segment"),
        }

        match &segments[3] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "table");
                }
            }
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_unmatched_braces() {
        assert!(parse_template("SELECT {unclosed FROM table").is_err());
        assert!(parse_template("SELECT closed} FROM table").is_err());
    }

    #[test]
    fn test_parse_token_stream_simple() {
        use quote::quote;

        let tokens = quote! { SELECT * FROM {users} };
        let segments = parse_token_stream(tokens).unwrap();
        assert_eq!(segments.len(), 2);

        match &segments[0] {
            SqlSegment::Text(text) => {
                // Token streams don't preserve exact spacing, so we'll be flexible
                assert!(text.contains("SELECT"));
                assert!(text.contains("FROM"));
            }
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments.len(), 1);
                    assert_eq!(path.path.segments[0].ident.to_string(), "users");
                } else {
                    panic!("Expected path expression");
                }
            }
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_parse_token_stream_multiple() {
        use quote::quote;

        let tokens = quote! { SELECT {column} FROM {table} WHERE {condition} };
        let segments = parse_token_stream(tokens).unwrap();
        assert_eq!(segments.len(), 6);

        // Verify some key segments
        match &segments[0] {
            SqlSegment::Text(text) => {
                assert!(text.contains("SELECT"));
            }
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "column");
                }
            }
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_string_vs_token_equivalence() {
        use quote::quote;

        // Test that string literal and token stream approaches produce identical results
        let string_segments =
            parse_template("SELECT * FROM {table} where {table.id} = {id}").unwrap();

        let tokens = quote! { SELECT * FROM {table} where {table.id} = {id} };
        let token_segments = parse_token_stream(tokens).unwrap();

        // Both should have the same number of segments
        assert_eq!(string_segments.len(), token_segments.len());

        // Both should generate equivalent text segments (spacing might differ slightly but structure is same)
        for (string_seg, token_seg) in string_segments.iter().zip(token_segments.iter()) {
            match (string_seg, token_seg) {
                (SqlSegment::Text(_), SqlSegment::Text(_)) => {} // Both are text
                (SqlSegment::Expression(_), SqlSegment::Expression(_)) => {} // Both are expressions
                _ => panic!("Segment types don't match"),
            }
        }
    }

    #[test]
    fn test_parse_printf_style() {
        use syn::parse_str;

        // Test printf-style parsing
        let args = vec![
            parse_str::<Expr>("table").unwrap(),
            parse_str::<Expr>("table.id").unwrap(),
            parse_str::<Expr>("42").unwrap(),
        ];

        let segments =
            parse_template_with_args("SELECT * FROM {} WHERE {} = {}", Some(&args)).unwrap();
        assert_eq!(segments.len(), 6);

        match &segments[0] {
            SqlSegment::Text(text) => assert_eq!(text, "SELECT * FROM "),
            _ => panic!("Expected text segment"),
        }

        match &segments[1] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "table");
                } else {
                    panic!("Expected path expression");
                }
            }
            _ => panic!("Expected expression segment"),
        }

        match &segments[2] {
            SqlSegment::Text(text) => assert_eq!(text, " WHERE "),
            _ => panic!("Expected text segment"),
        }

        match &segments[3] {
            SqlSegment::Expression(expr) => {
                // This should be table.id field access
                matches!(expr, Expr::Field(_));
            }
            _ => panic!("Expected expression segment"),
        }

        match &segments[4] {
            SqlSegment::Text(text) => assert_eq!(text, " = "),
            _ => panic!("Expected text segment"),
        }

        match &segments[5] {
            SqlSegment::Expression(expr) => {
                if let Expr::Lit(lit) = expr {
                    if let syn::Lit::Int(int_lit) = &lit.lit {
                        assert_eq!(int_lit.base10_digits(), "42");
                    } else {
                        panic!("Expected integer literal");
                    }
                } else {
                    panic!("Expected literal expression");
                }
            }
            _ => panic!("Expected expression segment"),
        }
    }

    #[test]
    fn test_printf_argument_count_validation() {
        use syn::parse_str;

        // Test too few arguments
        let args = vec![parse_str::<Expr>("table").unwrap()];
        let result = parse_template_with_args("SELECT * FROM {} WHERE {} = {}", Some(&args));
        assert!(result.is_err());

        // Test too many arguments
        let args = vec![
            parse_str::<Expr>("table").unwrap(),
            parse_str::<Expr>("column").unwrap(),
            parse_str::<Expr>("value").unwrap(),
            parse_str::<Expr>("extra").unwrap(),
        ];
        let result = parse_template_with_args("SELECT * FROM {} WHERE {} = {}", Some(&args));
        assert!(result.is_err());

        // Test exact match should work
        let args = vec![
            parse_str::<Expr>("table").unwrap(),
            parse_str::<Expr>("column").unwrap(),
            parse_str::<Expr>("value").unwrap(),
        ];
        let result = parse_template_with_args("SELECT * FROM {} WHERE {} = {}", Some(&args));
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_named_and_positional() {
        use syn::parse_str;

        // Test mixing named expressions and positional arguments
        let args = vec![
            parse_str::<Expr>("users").unwrap(),
            parse_str::<Expr>("42").unwrap(),
        ];

        let segments =
            parse_template_with_args("SELECT * FROM {} WHERE {id} = {}", Some(&args)).unwrap();
        assert_eq!(segments.len(), 6);

        // First {} should be replaced with "users"
        match &segments[1] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "users");
                }
            }
            _ => panic!("Expected expression segment"),
        }

        // {id} should be parsed as named expression
        match &segments[3] {
            SqlSegment::Expression(expr) => {
                if let Expr::Path(path) = expr {
                    assert_eq!(path.path.segments[0].ident.to_string(), "id");
                }
            }
            _ => panic!("Expected expression segment"),
        }

        // Second {} should be replaced with "42"
        match &segments[5] {
            SqlSegment::Expression(expr) => {
                if let Expr::Lit(_) = expr {
                    // This is the literal 42
                } else {
                    panic!("Expected literal expression");
                }
            }
            _ => panic!("Expected expression segment"),
        }
    }
}
