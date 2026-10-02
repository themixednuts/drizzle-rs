//! Reads enum-like `CHECK` constraints, `col IN ('a', 'b')`, so seeded
//! values satisfy them.

/// One lexical token of a `CHECK` expression. Parentheses, brackets and
/// commas are dropped, which is safe because only a single
/// `column IN (strings)` shape is accepted afterwards.
#[derive(Debug, PartialEq)]
enum Token {
    /// A bare word: an unquoted identifier or a keyword.
    Word(String),
    /// A quoted identifier.
    Quoted(String),
    /// A string literal.
    Str(String),
    /// Any other punctuation, such as `=` or `::`.
    Punct(char),
}

/// Splits `expression` into tokens, or `None` on anything unexpected (an
/// unterminated quote, a number, an operator other than `=` and `::`).
/// With `backslash_escapes`, string literals follow MySQL's rules, where
/// `\'` and `\\` stand for a quote and a backslash.
fn tokenize(expression: &str, backslash_escapes: bool) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = expression.chars().peekable();
    while let Some(&character) = chars.peek() {
        match character {
            c if c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | ',') => {
                chars.next();
            }
            '\'' => {
                chars.next();
                tokens.push(Token::Str(quoted(&mut chars, '\'', backslash_escapes)?));
            }
            '"' | '`' => {
                chars.next();
                tokens.push(Token::Quoted(quoted(&mut chars, character, false)?));
            }
            ':' => {
                chars.next();
                if chars.next() != Some(':') {
                    return None;
                }
                tokens.push(Token::Punct(':'));
            }
            '=' => {
                chars.next();
                tokens.push(Token::Punct('='));
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' || c == '$' {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Word(word));
            }
            _ => return None,
        }
    }
    Some(tokens)
}

/// Reads up to the closing `quote`, where a doubled quote is a literal one
/// (and, with `backslash_escapes`, a backslash escapes the next character).
fn quoted(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    quote: char,
    backslash_escapes: bool,
) -> Option<String> {
    let mut value = String::new();
    loop {
        let character = chars.next()?;
        if backslash_escapes && character == '\\' {
            value.push(chars.next()?);
        } else if character == quote {
            if chars.peek() == Some(&quote) {
                chars.next();
                value.push(quote);
            } else {
                return Some(value);
            }
        } else {
            value.push(character);
        }
    }
}

/// The column and its allowed values when `expression` (with or without a
/// leading `CHECK`) only restricts one column to a list of strings:
/// `"status" IN ('a', 'b')`, PostgreSQL's
/// `((status)::text = ANY ((ARRAY['a'::character varying])::text[]))`, or
/// MySQL's `` (`status` in (_utf8mb4'a',_utf8mb4'b')) ``. Anything else,
/// including `NOT IN`, other conditions joined with `AND`/`OR`, and numeric
/// lists, gives `None`.
pub(super) fn enum_check(expression: &str) -> Option<(String, Vec<String>)> {
    // MySQL's `CHECK_CLAUSE` escapes the expression once more, writing each
    // quote as `\'` and each backslash as `\\`. Undo that, then read its
    // string literals with MySQL's backslash escapes.
    let mysql_escaped = expression.contains("\\'");
    let unescaped;
    let expression = if mysql_escaped {
        let mut out = String::with_capacity(expression.len());
        let mut chars = expression.chars();
        while let Some(character) = chars.next() {
            if character == '\\' {
                out.push(chars.next()?);
            } else {
                out.push(character);
            }
        }
        unescaped = out;
        unescaped.as_str()
    } else {
        expression
    };
    let mut tokens = tokenize(expression, mysql_escaped)?;
    if matches!(tokens.first(), Some(Token::Word(word)) if word.eq_ignore_ascii_case("check")) {
        tokens.remove(0);
    }

    // Drop casts (`::text`, `::character varying`, `::text[]`) and charset
    // introducers (`_utf8mb4'a'`).
    let mut cleaned = Vec::with_capacity(tokens.len());
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            Token::Punct(':') => {
                while matches!(tokens.peek(), Some(Token::Word(_))) {
                    tokens.next();
                }
            }
            Token::Word(word)
                if word.starts_with('_') && matches!(tokens.peek(), Some(Token::Str(_))) => {}
            token => cleaned.push(token),
        }
    }

    let mut cleaned = cleaned.into_iter();
    let column = match cleaned.next()? {
        Token::Quoted(name) => name,
        Token::Word(name) if !is_keyword(&name) => name,
        _ => return None,
    };
    match cleaned.next()? {
        Token::Word(word) if word.eq_ignore_ascii_case("in") => {}
        Token::Punct('=') => {
            let any = cleaned.next()?;
            let array = cleaned.next()?;
            if !matches!(&any, Token::Word(word) if word.eq_ignore_ascii_case("any"))
                || !matches!(&array, Token::Word(word) if word.eq_ignore_ascii_case("array"))
            {
                return None;
            }
        }
        _ => return None,
    }
    let values: Vec<String> = cleaned
        .map(|token| match token {
            Token::Str(value) => Some(value),
            _ => None,
        })
        .collect::<Option<_>>()?;
    (!values.is_empty()).then_some((column, values))
}

fn is_keyword(word: &str) -> bool {
    ["not", "and", "or", "in", "any", "array", "null", "check"]
        .iter()
        .any(|keyword| word.eq_ignore_ascii_case(keyword))
}

#[cfg(test)]
mod tests {
    use super::enum_check;

    fn values(expression: &str) -> Option<(String, Vec<String>)> {
        enum_check(expression)
    }

    #[test]
    fn reads_each_dialects_spelling() {
        let expected = Some((
            "status".to_string(),
            vec!["a".to_string(), "it's".to_string()],
        ));
        for expression in [
            r#""status" IN ('a', 'it''s')"#,
            "status in ('a','it''s')",
            "CHECK (status IN ('a', 'it''s'))",
            "CHECK ((status = ANY (ARRAY['a'::text, 'it''s'::text])))",
            "CHECK (((status)::text = ANY ((ARRAY['a'::character varying, 'it''s'::character varying])::text[])))",
            "(`status` in (_utf8mb4'a',_utf8mb4'it''s'))",
            // MySQL's information_schema.CHECK_CONSTRAINTS.CHECK_CLAUSE.
            r"(`status` in (_latin1\'a\',_utf8mb4\'it\\\'s\'))",
        ] {
            assert_eq!(values(expression), expected, "{expression}");
        }
    }

    #[test]
    fn rejects_anything_but_a_plain_string_list() {
        for expression in [
            "status NOT IN ('a', 'b')",
            "status IN ('a') OR status IS NULL",
            "status IN ('a') AND other IN ('b')",
            "priority IN (1, 2, 3)",
            "length(name) > 3",
            "age >= 18",
            "status IN ()",
            "status IN ('unterminated)",
        ] {
            assert_eq!(values(expression), None, "{expression}");
        }
    }
}
