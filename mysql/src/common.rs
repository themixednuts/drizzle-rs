use crate::prelude::{String, Vec};
use drizzle_core::{SQLIndexInfo, SQLSchemaType, SQLViewInfo, TableRef};
use drizzle_types::mysql::ddl::{ViewAlgorithm, ViewCheckOption, ViewSqlSecurity};

/// Saves the session's foreign-key check setting before
/// [`DISABLE_FOREIGN_KEY_CHECKS`].
///
/// A schema whose tables reference each other in a cycle cannot create them
/// one after another while `MySQL` checks that each reference exists, so its
/// `create_statements` create those tables with the checks off, the way
/// `mysqldump` does, and restore the setting after.
pub const SAVE_FOREIGN_KEY_CHECKS: &str =
    "SET @drizzle_foreign_key_checks = @@SESSION.foreign_key_checks";

/// Turns foreign-key checks off for the session; see
/// [`SAVE_FOREIGN_KEY_CHECKS`].
pub const DISABLE_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 0";

/// Restores the setting [`SAVE_FOREIGN_KEY_CHECKS`] saved and forgets it.
/// With nothing saved it leaves the setting as it is, so a failed `create`
/// can always run it.
pub const RESTORE_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = \
     COALESCE(@drizzle_foreign_key_checks, @@SESSION.foreign_key_checks), \
     @drizzle_foreign_key_checks = NULL";

/// `MySQL`-specific options of a generated view.
pub trait MySQLViewInfo: SQLViewInfo + core::fmt::Debug {
    /// The `ALGORITHM` option, if set.
    fn algorithm(&self) -> Option<ViewAlgorithm>;
    /// The `SQL SECURITY` option, if set.
    fn sql_security(&self) -> Option<ViewSqlSecurity>;
    /// The `WITH .. CHECK OPTION` mode, if set.
    fn check_option(&self) -> Option<ViewCheckOption>;
}

/// Renders the `CREATE VIEW` statement for a generated view.
///
/// Writes `CREATE [ALGORITHM=..] [SQL SECURITY ..] VIEW name AS definition
/// [WITH .. CHECK OPTION];`, with the name backtick-quoted and qualified by
/// its database when it has one. Returns an empty string for a view marked
/// `EXISTING`.
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

/// Renders `CREATE VIEW` statements for the managed views of a schema, each
/// after the views it references.
///
/// Views marked `EXISTING` are skipped. A view counts as referencing another
/// when the other's name appears as an identifier in its definition. Public
/// only for code emitted by `drizzle-macros`.
///
/// # Errors
///
/// Returns [`DrizzleError::Statement`](drizzle_core::error::DrizzleError::Statement)
/// if the views reference each other in a cycle.
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

/// The `MySQL` dialect marker for schema items.
///
/// Used as a type parameter (`SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>`)
/// to tie tables, indexes and views to `MySQL`. The variants name the kinds
/// of schema object.
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
