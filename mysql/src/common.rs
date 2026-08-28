use crate::prelude::{String, Vec};
use drizzle_core::{SQLIndexInfo, SQLSchemaType, SQLViewInfo, TableRef};
use drizzle_types::mysql::ddl::{ViewAlgorithm, ViewCheckOption, ViewSqlSecurity};

/// MySQL-specific metadata attached to a generated view.
pub trait MySQLViewInfo: SQLViewInfo + core::fmt::Debug {
    /// The optional view evaluation algorithm.
    fn algorithm(&self) -> Option<ViewAlgorithm>;
    /// The optional SQL security context.
    fn sql_security(&self) -> Option<ViewSqlSecurity>;
    /// The optional updatability check mode.
    fn check_option(&self) -> Option<ViewCheckOption>;
}

/// Render a generated MySQL view's `CREATE VIEW` statement.
#[must_use]
pub fn create_view_sql(view: &dyn MySQLViewInfo) -> String {
    if view.is_existing() {
        return String::new();
    }

    let mut sql = String::from("CREATE ");
    if let Some(algorithm) = view.algorithm() {
        sql.push_str(match algorithm {
            ViewAlgorithm::Undefined => "ALGORITHM=UNDEFINED ",
            ViewAlgorithm::Merge => "ALGORITHM=MERGE ",
            ViewAlgorithm::Temptable => "ALGORITHM=TEMPTABLE ",
        });
    }
    if let Some(security) = view.sql_security() {
        sql.push_str(match security {
            ViewSqlSecurity::Definer => "SQL SECURITY DEFINER ",
            ViewSqlSecurity::Invoker => "SQL SECURITY INVOKER ",
        });
    }
    sql.push_str("VIEW ");
    if let Some(database) = drizzle_core::SQLTableInfo::schema(view) {
        push_identifier(&mut sql, database);
        sql.push('.');
    }
    push_identifier(&mut sql, drizzle_core::SQLTableInfo::name(view));
    sql.push_str(" AS ");
    sql.push_str(&view.definition_sql());
    if let Some(check_option) = view.check_option() {
        sql.push_str(match check_option {
            ViewCheckOption::Cascaded => " WITH CASCADED CHECK OPTION",
            ViewCheckOption::Local => " WITH LOCAL CHECK OPTION",
        });
    }
    sql.push(';');
    sql
}

fn flush_identifier(tokens: &mut Vec<String>, token: &mut String) {
    if !token.is_empty() {
        tokens.push(core::mem::take(token));
    }
}

fn identifier_tokens(sql: &str) -> Vec<String> {
    #[derive(Clone, Copy)]
    enum State {
        Sql,
        QuotedIdentifier,
        SingleQuotedString,
        DoubleQuotedString,
        LineComment,
        BlockComment,
    }

    let mut tokens = Vec::new();
    let mut token = String::new();
    let characters: Vec<_> = sql.chars().collect();
    let mut state = State::Sql;
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];
        match state {
            State::Sql => match character {
                '`' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::QuotedIdentifier;
                }
                '\'' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::SingleQuotedString;
                }
                '"' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::DoubleQuotedString;
                }
                '#' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::LineComment;
                }
                '-' if characters.get(index + 1) == Some(&'-')
                    && characters
                        .get(index + 2)
                        .is_none_or(|next| next.is_whitespace() || next.is_control()) =>
                {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::LineComment;
                    index += 1;
                }
                '/' if characters.get(index + 1) == Some(&'*') => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::BlockComment;
                    index += 1;
                }
                character if character.is_alphanumeric() || matches!(character, '_' | '$') => {
                    token.push(character);
                }
                _ => flush_identifier(&mut tokens, &mut token),
            },
            State::QuotedIdentifier => {
                if character == '`' {
                    if characters.get(index + 1) == Some(&'`') {
                        token.push('`');
                        index += 1;
                    } else {
                        flush_identifier(&mut tokens, &mut token);
                        state = State::Sql;
                    }
                } else {
                    token.push(character);
                }
            }
            State::SingleQuotedString | State::DoubleQuotedString => {
                let quote = match state {
                    State::SingleQuotedString => '\'',
                    State::DoubleQuotedString => '"',
                    _ => unreachable!(),
                };
                if character == '\\' {
                    index += usize::from(index + 1 < characters.len());
                } else if character == quote {
                    if characters.get(index + 1) == Some(&quote) {
                        index += 1;
                    } else {
                        state = State::Sql;
                    }
                }
            }
            State::LineComment => {
                if matches!(character, '\n' | '\r') {
                    state = State::Sql;
                }
            }
            State::BlockComment => {
                if character == '*' && characters.get(index + 1) == Some(&'/') {
                    state = State::Sql;
                    index += 1;
                }
            }
        }
        index += 1;
    }
    flush_identifier(&mut tokens, &mut token);
    tokens
}

/// Order managed schema views so every referenced view is created first.
///
/// This is public only for code emitted by `drizzle-macros`.
#[doc(hidden)]
pub fn order_schema_views(
    views: &[&'static dyn MySQLViewInfo],
) -> Result<Vec<String>, drizzle_core::error::DrizzleError> {
    let mut pending: Vec<_> = views
        .iter()
        .copied()
        .filter(|view| !view.is_existing())
        .collect();
    pending.sort_by(|left, right| left.qualified_name().cmp(&right.qualified_name()));

    let mut statements = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let ready = pending.iter().position(|view| {
            let tokens = identifier_tokens(&view.definition_sql());
            !pending.iter().any(|candidate| {
                !core::ptr::eq(*view, *candidate)
                    && tokens.iter().any(|token| token == candidate.name())
            })
        });
        let Some(ready) = ready else {
            let names = pending
                .iter()
                .map(|view| view.qualified_name().into_owned())
                .collect::<Vec<_>>()
                .join(", ");
            let mut message = String::from("Cyclic view dependency detected in MySQLSchema: ");
            message.push_str(&names);
            return Err(drizzle_core::error::DrizzleError::Statement(message.into()));
        };
        statements.push(create_view_sql(pending.remove(ready)));
    }
    Ok(statements)
}

fn push_identifier(sql: &mut String, identifier: &str) {
    sql.push('`');
    for character in identifier.chars() {
        if character == '`' {
            sql.push('`');
        }
        sql.push(character);
    }
    sql.push('`');
}

/// The kind of object contributed by a generated MySQL schema item.
#[derive(Debug, Clone)]
pub enum MySQLSchemaType {
    /// A table definition.
    Table(&'static TableRef),
    /// An index definition.
    Index(&'static dyn SQLIndexInfo),
    /// A view definition.
    View(&'static dyn MySQLViewInfo),
}

impl SQLSchemaType for MySQLSchemaType {}
