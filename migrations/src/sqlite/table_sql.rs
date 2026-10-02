//! Reads the parts of a stored `CREATE TABLE` statement that SQLite's
//! PRAGMAs do not report: CHECK constraints, column collations, generated
//! column expressions, and the names of UNIQUE and FOREIGN KEY constraints.
//!
//! SQLite keeps each table's original `CREATE TABLE` text in
//! `sqlite_schema`. This scanner understands enough of its grammar to find
//! those clauses: it respects quoting (`'...'`, `"..."`, `` `...` ``,
//! `[...]`), comments and nested parentheses, so a `(` inside a string
//! default or a comma inside an expression does not throw it off.

/// What a table's `CREATE TABLE` text declares beyond the PRAGMAs.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedTableSql {
    /// One entry per column definition, in order.
    pub columns: Vec<ParsedColumnSql>,
    /// Every CHECK constraint, column-level and table-level, in order:
    /// `(explicit name, expression)`.
    pub checks: Vec<(Option<String>, String)>,
    /// Named UNIQUE constraints: `(name, columns)`.
    pub uniques: Vec<(String, Vec<String>)>,
    /// Named FOREIGN KEY constraints: `(name, source columns)`.
    pub foreign_keys: Vec<(String, Vec<String>)>,
}

/// One column definition.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedColumnSql {
    pub name: String,
    pub collate: Option<String>,
    /// `(expression, stored)` of a generated column.
    pub generated: Option<(String, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token<'a> {
    /// A bare word or number.
    Word(&'a str),
    /// A quoted identifier, unquoted.
    Ident(String),
    /// A string literal.
    Str,
    /// A parenthesized group: its inner text, as written.
    Group(&'a str),
    /// Any other character.
    Other,
}

impl Token<'_> {
    fn is_word(&self, word: &str) -> bool {
        matches!(self, Token::Word(w) if w.eq_ignore_ascii_case(word))
    }

    /// The identifier this token names, quoted or bare.
    fn identifier(&self) -> Option<String> {
        match self {
            Token::Word(word) => Some((*word).to_string()),
            Token::Ident(name) => Some(name.clone()),
            _ => None,
        }
    }
}

/// Index just past the quoted section starting at `start` (which holds the
/// opening quote), where a doubled closing quote is an escaped one.
fn skip_quoted(bytes: &[u8], start: usize, close: u8) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == close {
            if close != b']' && bytes.get(i + 1) == Some(&close) {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// Index just past a comment starting at `start`, or `start` when there is
/// none.
fn skip_comment(bytes: &[u8], start: usize) -> usize {
    if bytes[start..].starts_with(b"--") {
        bytes[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |end| start + end + 1)
    } else if bytes[start..].starts_with(b"/*") {
        bytes[start + 2..]
            .windows(2)
            .position(|w| w == b"*/")
            .map_or(bytes.len(), |end| start + 2 + end + 2)
    } else {
        start
    }
}

/// Index of the `)` closing the `(` at `open`, skipping quotes and comments.
fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => i = skip_quoted(bytes, i, b'\''),
            b'"' => i = skip_quoted(bytes, i, b'"'),
            b'`' => i = skip_quoted(bytes, i, b'`'),
            b'[' => i = skip_quoted(bytes, i, b']'),
            b'-' | b'/' if skip_comment(bytes, i) != i => i = skip_comment(bytes, i),
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// Splits `text` at commas outside quotes, comments and parentheses.
fn split_top_level(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => i = skip_quoted(bytes, i, b'\''),
            b'"' => i = skip_quoted(bytes, i, b'"'),
            b'`' => i = skip_quoted(bytes, i, b'`'),
            b'[' => i = skip_quoted(bytes, i, b']'),
            b'-' | b'/' if skip_comment(bytes, i) != i => i = skip_comment(bytes, i),
            b'(' => i = matching_paren(text, i).map_or(bytes.len(), |close| close + 1),
            b',' => {
                parts.push(text[start..i].trim());
                start = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    parts.push(text[start..].trim());
    parts.retain(|part| !part.is_empty());
    parts
}

fn unquote(quoted: &str, close: char) -> String {
    let inner = &quoted[1..quoted.len() - 1];
    if close == ']' {
        inner.to_string()
    } else {
        inner.replace(&format!("{close}{close}"), &close.to_string())
    }
}

fn tokenize(text: &str) -> Vec<Token<'_>> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
        } else if (b == b'-' || b == b'/') && skip_comment(bytes, i) != i {
            i = skip_comment(bytes, i);
        } else if b == b'\'' {
            i = skip_quoted(bytes, i, b'\'');
            tokens.push(Token::Str);
        } else if matches!(b, b'"' | b'`' | b'[') {
            let close = if b == b'[' { b']' } else { b };
            let end = skip_quoted(bytes, i, close);
            if end <= bytes.len() && end >= i + 2 {
                tokens.push(Token::Ident(unquote(&text[i..end], close as char)));
            }
            i = end;
        } else if b == b'(' {
            let close = matching_paren(text, i).unwrap_or(bytes.len() - 1);
            tokens.push(Token::Group(text[i + 1..close].trim()));
            i = close + 1;
        } else if b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80 {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || bytes[i] == b'$'
                    || bytes[i] >= 0x80)
            {
                i += 1;
            }
            tokens.push(Token::Word(&text[start..i]));
        } else {
            tokens.push(Token::Other);
            i += 1;
        }
    }
    tokens
}

/// The column names in a `UNIQUE (...)` / `FOREIGN KEY (...)` list.
fn column_list(group: &str) -> Vec<String> {
    split_top_level(group)
        .into_iter()
        .filter_map(|item| tokenize(item).first().and_then(Token::identifier))
        .collect()
}

/// Parses `sql`, a stored `CREATE TABLE` statement. Returns `None` when it
/// has no column list (for example `CREATE TABLE t AS SELECT ...`).
pub(crate) fn parse_table_sql(sql: &str) -> Option<ParsedTableSql> {
    let open = {
        let bytes = sql.as_bytes();
        let mut i = 0;
        loop {
            match bytes.get(i)? {
                b'\'' => i = skip_quoted(bytes, i, b'\''),
                b'"' => i = skip_quoted(bytes, i, b'"'),
                b'`' => i = skip_quoted(bytes, i, b'`'),
                b'[' => i = skip_quoted(bytes, i, b']'),
                b'(' => break i,
                _ => i += 1,
            }
        }
    };
    // `CREATE TABLE t AS SELECT ...` has no column list.
    if tokenize(&sql[..open])
        .iter()
        .any(|token| token.is_word("AS"))
    {
        return None;
    }
    let close = matching_paren(sql, open)?;
    let mut parsed = ParsedTableSql::default();

    for element in split_top_level(&sql[open + 1..close]) {
        let tokens = tokenize(element);
        let Some(first) = tokens.first() else {
            continue;
        };
        let table_constraint = ["CONSTRAINT", "PRIMARY", "UNIQUE", "CHECK", "FOREIGN"]
            .iter()
            .any(|keyword| first.is_word(keyword));
        if table_constraint {
            parse_constraints(&tokens, None, &mut parsed);
            continue;
        }

        let Some(name) = first.identifier() else {
            continue;
        };
        let mut column = ParsedColumnSql {
            name: name.clone(),
            ..ParsedColumnSql::default()
        };
        let rest = &tokens[1..];
        for (i, token) in rest.iter().enumerate() {
            if token.is_word("COLLATE")
                && let Some(collation) = rest.get(i + 1).and_then(Token::identifier)
            {
                column.collate = Some(collation);
            }
            // `GENERATED ALWAYS AS (expr)`, or the short form `AS (expr)`,
            // optionally followed by STORED or VIRTUAL.
            if token.is_word("AS")
                && let Some(Token::Group(expression)) = rest.get(i + 1)
            {
                let stored = rest.get(i + 2).is_some_and(|next| next.is_word("STORED"));
                column.generated = Some(((*expression).to_string(), stored));
            }
        }
        parse_constraints(rest, Some(&name), &mut parsed);
        parsed.columns.push(column);
    }
    Some(parsed)
}

/// Collects CHECK constraints and the names of UNIQUE and FOREIGN KEY
/// constraints from `tokens`. `column` is set for a column definition,
/// whose constraints apply to that column.
fn parse_constraints(tokens: &[Token<'_>], column: Option<&str>, parsed: &mut ParsedTableSql) {
    // `CONSTRAINT name` names only the constraint keyword right after it.
    let mut named: Option<(usize, String)> = None;
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        if token.is_word("CONSTRAINT") {
            if let Some(name) = tokens.get(i + 1).and_then(Token::identifier) {
                named = Some((i + 2, name));
            }
            i += 2;
            continue;
        }
        let name = match &named {
            Some((at, name)) if *at == i => Some(name.clone()),
            _ => None,
        };
        if token.is_word("CHECK")
            && let Some(Token::Group(expression)) = tokens.get(i + 1)
        {
            parsed.checks.push((name, (*expression).to_string()));
            i += 2;
            continue;
        }
        if token.is_word("UNIQUE") {
            let columns = match (column, tokens.get(i + 1)) {
                (Some(column), _) => Some(vec![column.to_string()]),
                (None, Some(Token::Group(list))) => Some(column_list(list)),
                _ => None,
            };
            if let (Some(name), Some(columns)) = (name, columns) {
                parsed.uniques.push((name, columns));
            }
        } else if token.is_word("FOREIGN") || token.is_word("REFERENCES") {
            let columns = match (column, token.is_word("FOREIGN"), tokens.get(i + 2)) {
                (Some(column), false, _) => Some(vec![column.to_string()]),
                (None, true, Some(Token::Group(list))) => Some(column_list(list)),
                _ => None,
            };
            if let (Some(name), Some(columns)) = (name, columns) {
                parsed.foreign_keys.push((name, columns));
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_checks_collations_generated_columns_and_constraint_names() {
        let sql = r#"CREATE TABLE "t" (
            `id` INTEGER PRIMARY KEY,
            note TEXT DEFAULT '(' COLLATE NOCASE,
            "v" INTEGER CHECK (v > 0) CONSTRAINT v_small CHECK(v < 100),
            b INTEGER AS (id * 2) STORED,
            c TEXT GENERATED ALWAYS AS(length(note)),
            pid INTEGER CONSTRAINT t_pid_fk REFERENCES p(id),
            [x y] TEXT, -- a comment, with (parens
            CONSTRAINT "named_chk" CHECK ((v + id) <> 3),
            CHECK (note <> 'a,b)'),
            CONSTRAINT my_uq UNIQUE ("v", b),
            CONSTRAINT my_fk FOREIGN KEY (b) REFERENCES p(id) ON DELETE CASCADE
        ) STRICT"#;
        let parsed = parse_table_sql(sql).unwrap();
        let names: Vec<&str> = parsed.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["id", "note", "v", "b", "c", "pid", "x y"]);
        assert_eq!(parsed.columns[1].collate.as_deref(), Some("NOCASE"));
        assert_eq!(parsed.columns[1].generated, None);
        assert_eq!(
            parsed.columns[3].generated,
            Some(("id * 2".to_string(), true))
        );
        assert_eq!(
            parsed.columns[4].generated,
            Some(("length(note)".to_string(), false))
        );
        assert_eq!(
            parsed.checks,
            [
                (None, "v > 0".to_string()),
                (Some("v_small".to_string()), "v < 100".to_string()),
                (Some("named_chk".to_string()), "(v + id) <> 3".to_string()),
                (None, "note <> 'a,b)'".to_string()),
            ]
        );
        assert_eq!(
            parsed.uniques,
            [("my_uq".to_string(), vec!["v".to_string(), "b".to_string()])]
        );
        assert_eq!(
            parsed.foreign_keys,
            [
                ("t_pid_fk".to_string(), vec!["pid".to_string()]),
                ("my_fk".to_string(), vec!["b".to_string()]),
            ]
        );
    }

    #[test]
    fn as_select_tables_have_no_column_list() {
        assert_eq!(parse_table_sql("CREATE TABLE t AS SELECT 1"), None);
        assert_eq!(
            parse_table_sql("CREATE TABLE t AS SELECT count(*) FROM u"),
            None
        );
    }

    #[test]
    fn a_constraint_name_applies_to_the_next_constraint_only() {
        let parsed = parse_table_sql(
            "CREATE TABLE t (id INTEGER CONSTRAINT pk PRIMARY KEY CHECK (id > 0) CONSTRAINT u UNIQUE)",
        )
        .unwrap();
        assert_eq!(parsed.checks, [(None, "id > 0".to_string())]);
        assert_eq!(parsed.uniques, [("u".to_string(), vec!["id".to_string()])]);
    }
}
