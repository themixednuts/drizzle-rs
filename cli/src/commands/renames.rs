//! Rename-or-create prompts for `drizzle generate` and `drizzle push`, ported
//! from drizzle-kit's `resolver` (`cli/prompts.ts`).
//!
//! When the diff drops one entity and creates another of the same kind in
//! the same scope, it cannot tell a rename from a drop plus a create. The
//! CLI never guesses:
//!
//! - With a terminal on stdin, it asks, one question per created entity
//!   that has deleted candidates, in drizzle-kit's order (schemas, enums,
//!   tables, columns, indexes, views). The first choice creates; each other
//!   choice renames from one candidate. A rename consumes its candidate.
//! - Without one, it answers from `--hints` / `--hints-file` (drizzle-kit's
//!   JSON hint format). Questions no hint answers are listed and the command
//!   exits with code 2 without changing anything.
//!
//! Hints are consulted in interactive runs too, so a hinted question is not
//! asked. `push --force` only skips the data-loss confirmation, not these
//! questions, as in drizzle-kit.

use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::PathBuf;

use colored::Colorize;
use drizzle_migrations::{DiffOptions, RenameAnswer, RenameKind, RenameQuestion, Snapshot};
use serde_json::Value;

use crate::error::CliError;

/// `--hints` / `--hints-file`, shared by `generate` and `push`.
#[derive(clap::Args, Debug, Clone, Default)]
pub struct HintArgs {
    /// Inline JSON array of hints answering rename-or-create questions
    /// without prompting, e.g.
    /// '[{"type":"rename","kind":"table","from":["public","users"],"to":["public","accounts"]}]'
    #[arg(long, value_name = "JSON", conflicts_with = "hints_file")]
    pub hints: Option<String>,

    /// Path to a JSON file containing a hints array (same format as --hints)
    #[arg(long = "hints-file", value_name = "PATH")]
    pub hints_file: Option<PathBuf>,
}

/// One parsed hint (drizzle-kit's `Hint`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hint {
    /// `{ "type": "rename", "kind", "from", "to" }`
    Rename {
        /// drizzle-kit kind name (`table`, `column`, ...).
        kind: String,
        /// Identifier tuple of the deleted entity.
        from: Vec<String>,
        /// Identifier tuple of the created entity.
        to: Vec<String>,
    },
    /// `{ "type": "create", "kind", "entity" }`
    Create {
        /// drizzle-kit kind name.
        kind: String,
        /// Identifier tuple of the created entity.
        entity: Vec<String>,
    },
    /// `{ "type": "confirm_data_loss", "kind", "entity" }`. Accepted so
    /// drizzle-kit hint files parse; data-loss approval is `push --force`.
    ConfirmDataLoss,
}

/// drizzle-kit's rename-or-create kinds and their identifier arity.
const RENAME_KINDS: &[(&str, usize)] = &[
    ("table", 2),
    ("column", 3),
    ("default", 3),
    ("schema", 1),
    ("enum", 2),
    ("sequence", 2),
    ("view", 2),
    ("policy", 3),
    ("role", 1),
    ("privilege", 5),
    ("check", 3),
    ("index", 3),
    ("unique", 3),
    ("primary_key", 3),
    ("foreign key", 3),
];

/// drizzle-kit's `confirm_data_loss` kinds and their identifier arity.
const CONFIRM_KINDS: &[(&str, usize)] = &[
    ("table", 2),
    ("column", 3),
    ("schema", 1),
    ("view", 2),
    ("primary_key", 3),
    ("add_not_null", 3),
    ("add_unique", 3),
];

impl HintArgs {
    /// Reads and parses the hints (none when neither flag is set).
    ///
    /// # Errors
    ///
    /// Returns [`CliError::InvalidHints`] if the file cannot be read, the
    /// text is not JSON, or an item does not have drizzle-kit's hint shape.
    pub fn load(&self) -> Result<Vec<Hint>, CliError> {
        let (source, text) = if let Some(path) = &self.hints_file {
            let text = std::fs::read_to_string(path).map_err(|error| {
                CliError::InvalidHints(format!(
                    "Failed to read hints file '{}': {error}",
                    path.display()
                ))
            })?;
            ("file", text)
        } else if let Some(text) = &self.hints {
            ("inline", text.clone())
        } else {
            return Ok(Vec::new());
        };
        let json: Value = serde_json::from_str(&text).map_err(|error| {
            CliError::InvalidHints(format!("Failed to parse hints JSON from {source}: {error}"))
        })?;
        parse_hints(&json)
    }
}

/// Parses a JSON hints array.
///
/// # Errors
///
/// Returns [`CliError::InvalidHints`] naming the first malformed item.
pub fn parse_hints(json: &Value) -> Result<Vec<Hint>, CliError> {
    let invalid = |path: &str, message: &str| {
        CliError::InvalidHints(format!("Invalid hint shape at {path}: {message}"))
    };
    let items = json
        .as_array()
        .ok_or_else(|| invalid("<root>", "expected an array"))?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let at = |field: &str| format!("[{index}]{field}");
            let object = item
                .as_object()
                .ok_or_else(|| invalid(&at(""), "expected an object"))?;
            let text = |field: &str| -> Result<&str, CliError> {
                object
                    .get(field)
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid(&at(&format!(".{field}")), "expected a string"))
            };
            let kind = text("kind")?;
            let hint_type = text("type")?;
            let (kinds, fields): (&[(&str, usize)], &[&str]) = match hint_type {
                "rename" => (RENAME_KINDS, &["type", "kind", "from", "to"]),
                "create" => (RENAME_KINDS, &["type", "kind", "entity"]),
                "confirm_data_loss" => (CONFIRM_KINDS, &["type", "kind", "entity"]),
                _ => {
                    return Err(invalid(
                        &at(".type"),
                        "expected \"rename\", \"create\" or \"confirm_data_loss\"",
                    ));
                }
            };
            if let Some(extra) = object.keys().find(|key| !fields.contains(&key.as_str())) {
                return Err(invalid(&at(""), &format!("unrecognized key \"{extra}\"")));
            }
            let arity = kinds
                .iter()
                .find(|(name, _)| *name == kind)
                .map(|(_, arity)| *arity)
                .ok_or_else(|| {
                    invalid(
                        &at(".kind"),
                        &format!("unknown kind \"{kind}\" for a {hint_type} hint"),
                    )
                })?;
            let tuple = |field: &str| -> Result<Vec<String>, CliError> {
                let values = object
                    .get(field)
                    .and_then(Value::as_array)
                    .filter(|values| values.len() == arity)
                    .ok_or_else(|| {
                        invalid(
                            &at(&format!(".{field}")),
                            &format!("expected an array of {arity} strings"),
                        )
                    })?;
                values
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_string)
                            .ok_or_else(|| invalid(&at(&format!(".{field}")), "expected strings"))
                    })
                    .collect()
            };
            Ok(match hint_type {
                "rename" => Hint::Rename {
                    kind: kind.to_string(),
                    from: tuple("from")?,
                    to: tuple("to")?,
                },
                "create" => Hint::Create {
                    kind: kind.to_string(),
                    entity: tuple("entity")?,
                },
                _ => {
                    tuple("entity")?;
                    Hint::ConfirmDataLoss
                }
            })
        })
        .collect()
}

/// Asks one rename-or-create question.
pub trait RenamePrompt {
    /// Returns the answer to `question`.
    ///
    /// # Errors
    ///
    /// Returns [`CliError`] when the prompt fails or is aborted.
    fn ask(&mut self, question: &RenameQuestion) -> Result<RenameAnswer, CliError>;
}

/// The terminal prompt: drizzle-kit's `ResolveSelect`, drawn with
/// `inquire::Select`.
#[derive(Debug, Default)]
pub struct TerminalPrompt;

impl RenamePrompt for TerminalPrompt {
    fn ask(&mut self, question: &RenameQuestion) -> Result<RenameAnswer, CliError> {
        let choices = prompt_choices(question);
        let selected = inquire::Select::new(&prompt_message(question), choices)
            .without_filtering()
            .without_help_message()
            .with_page_size(10)
            .raw_prompt()
            .map_err(|error| match error {
                inquire::InquireError::OperationCanceled
                | inquire::InquireError::OperationInterrupted => {
                    CliError::Aborted("Rename prompt aborted, so nothing was changed".to_string())
                }
                other => CliError::Other(format!("Rename prompt failed: {other}")),
            })?;
        Ok(selected.value.answer)
    }
}

/// How unanswered questions are handled.
pub enum Mode<'a> {
    /// Ask each one.
    Interactive(&'a mut dyn RenamePrompt),
    /// Collect them as missing hints (drizzle-kit's non-interactive mode).
    Hints,
}

/// Result of [`resolve_renames`].
#[derive(Debug)]
pub struct Resolution {
    /// The options to diff with: the base options, `infer_renames(false)`,
    /// and one hint per decided question.
    pub options: DiffOptions,
    /// Questions no hint answered (only in [`Mode::Hints`]).
    pub missing: Vec<RenameQuestion>,
}

/// Decides every rename-or-create question between `prev` and `current`
/// from `hints`, then from `mode`, the way drizzle-kit's resolver does.
///
/// # Errors
///
/// Returns [`CliError::InvalidHints`] if a rename hint targets a created
/// entity but its `from` is not one of the deleted candidates, and
/// [`CliError`] if listing the questions or a prompt fails.
pub fn resolve_renames(
    prev: &Snapshot,
    current: &Snapshot,
    base: DiffOptions,
    hints: &[Hint],
    mut mode: Mode<'_>,
) -> Result<Resolution, CliError> {
    let mut options = base.infer_renames(false);
    let mut missing = Vec::new();
    let mut asked = HashSet::new();
    // drizzle-kit prints `--- all <kind> conflicts resolved ---` after each
    // group (one kind in one scope) it decided something in.
    let mut group = Group::default();

    loop {
        let questions = drizzle_migrations::rename_questions(prev, current, &options)
            .map_err(|error| CliError::MigrationError(error.to_string()))?;
        let Some(question) = questions.into_iter().next() else {
            break;
        };
        group.enter(&question);

        let key = (question.kind, entity_id(&question, &question.name));
        if !asked.insert(key) {
            return Err(CliError::Other(format!(
                "internal error: the answer to {} `{}` did not apply",
                question.kind, question.name
            )));
        }

        let answer = if let Some(answer) = answer_from_hints(&question, hints)? {
            answer
        } else {
            match &mut mode {
                Mode::Interactive(prompt) => prompt.ask(&question)?,
                Mode::Hints => {
                    // Assume "create" so later questions are listed as well.
                    missing.push(question.clone());
                    question
                        .answer(&mut options.renames, &RenameAnswer::Create)
                        .map_err(|error| CliError::Other(error.to_string()))?;
                    continue;
                }
            }
        };
        println!("{}", selection_line(&question, &answer));
        group.decided = true;
        question
            .answer(&mut options.renames, &answer)
            .map_err(|error| CliError::Other(error.to_string()))?;
    }
    group.finish();

    Ok(Resolution { options, missing })
}

/// The group of questions being decided, for the end-of-group line.
#[derive(Default)]
struct Group {
    scope: Option<(RenameKind, Option<String>, Option<String>)>,
    decided: bool,
}

impl Group {
    fn enter(&mut self, question: &RenameQuestion) {
        let scope = (
            question.kind,
            question.schema.clone(),
            question.table.clone(),
        );
        if self.scope.as_ref() != Some(&scope) {
            self.finish();
            self.scope = Some(scope);
        }
    }

    fn finish(&mut self) {
        if let Some((kind, _, _)) = self.scope.take()
            && std::mem::take(&mut self.decided)
        {
            println!(
                "{}",
                format!("--- all {} conflicts resolved ---\n", humanize(kind)).bright_black()
            );
        }
    }
}

/// Resolves renames for a command: interactive when stdin is a terminal,
/// hints mode otherwise.
///
/// # Errors
///
/// Returns [`CliError::MissingHints`] (exit code 2) when hints mode leaves
/// questions unanswered, and the errors of [`HintArgs::load`] and
/// [`resolve_renames`].
pub fn resolve_for_command(
    prev: &Snapshot,
    current: &Snapshot,
    base: DiffOptions,
    hint_args: &HintArgs,
) -> Result<DiffOptions, CliError> {
    let hints = hint_args.load()?;
    if hints
        .iter()
        .any(|hint| matches!(hint, Hint::ConfirmDataLoss))
    {
        println!(
            "{}",
            crate::output::warn_line(
                "confirm_data_loss hints are ignored; use `push --force` to approve data loss"
            )
        );
    }
    let mut prompt = TerminalPrompt;
    let mode = if std::io::stdin().is_terminal() {
        Mode::Interactive(&mut prompt)
    } else {
        Mode::Hints
    };
    let resolution = resolve_renames(prev, current, base, &hints, mode)?;
    if !resolution.missing.is_empty() {
        return Err(CliError::MissingHints(missing_hints_report(
            &resolution.missing,
        )));
    }
    Ok(resolution.options)
}

/// The answer the hints give to `question`, if any.
fn answer_from_hints(
    question: &RenameQuestion,
    hints: &[Hint],
) -> Result<Option<RenameAnswer>, CliError> {
    let kind = question.kind.as_str();
    let id = entity_id(question, &question.name);
    for hint in hints {
        if let Hint::Rename {
            kind: hint_kind,
            from,
            to,
        } = hint
            && hint_kind == kind
            && *to == id
        {
            let source = question
                .candidates
                .iter()
                .find(|candidate| entity_id(question, candidate) == *from);
            return match source {
                Some(source) => Ok(Some(RenameAnswer::RenameFrom(source.clone()))),
                None => Err(CliError::InvalidHints(format!(
                    "rename hint's `from` {} doesn't match any deleted {}",
                    json_tuple(from),
                    humanize(question.kind)
                ))),
            };
        }
    }
    let created = hints.iter().any(|hint| {
        matches!(hint, Hint::Create { kind: hint_kind, entity } if hint_kind == kind && *entity == id)
    });
    Ok(created.then_some(RenameAnswer::Create))
}

/// drizzle-kit's identifier tuple for `name` in `question`'s scope:
/// `[name]` for schemas, `[schema, name]` for enums, tables, and views,
/// `[schema, table, name]` for columns, indexes, and constraints. SQLite and
/// MySQL use the
/// placeholder schema `public`, as drizzle-kit does.
#[must_use]
pub fn entity_id(question: &RenameQuestion, name: &str) -> Vec<String> {
    let schema = question.schema.as_deref().unwrap_or("public").to_string();
    match question.kind {
        RenameKind::Schema => vec![name.to_string()],
        RenameKind::Column
        | RenameKind::Index
        | RenameKind::Unique
        | RenameKind::Check
        | RenameKind::PrimaryKey
        | RenameKind::ForeignKey => vec![
            schema,
            question.table.clone().unwrap_or_default(),
            name.to_string(),
        ],
        _ => vec![schema, name.to_string()],
    }
}

fn humanize(kind: RenameKind) -> String {
    kind.as_str().replace('_', " ")
}

fn json_tuple(segments: &[String]) -> String {
    let quoted: Vec<String> = segments
        .iter()
        .map(|segment| Value::String(segment.clone()).to_string())
        .collect();
    format!("[{}]", quoted.join(", "))
}

/// drizzle-kit's `keyFor`: `schema.` (when not `public`), then `table.`,
/// then the name.
fn key_for(question: &RenameQuestion, name: &str) -> String {
    let mut key = String::new();
    if let Some(schema) = question.schema.as_deref()
        && schema != "public"
    {
        key.push_str(schema);
        key.push('.');
    }
    if let Some(table) = question.table.as_deref() {
        key.push_str(table);
        key.push('.');
    }
    key.push_str(name);
    key
}

/// The question line: `Is <key> <kind> created or renamed from another <kind>?`
#[must_use]
pub fn prompt_message(question: &RenameQuestion) -> String {
    let kind = humanize(question.kind);
    format!(
        "Is {} {kind} created or renamed from another {kind}?",
        key_for(question, &question.name).bold().blue()
    )
}

/// One choice of the prompt; its `Display` is the rendered line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// What picking this choice answers.
    pub answer: RenameAnswer,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// The prompt's choices, as drizzle-kit renders them: first
/// `+ <name>  create <kind>`, then `~ <old> › <new>  rename <kind>` for each
/// candidate, padded to a common width.
#[must_use]
pub fn prompt_choices(question: &RenameQuestion) -> Vec<Choice> {
    let kind = humanize(question.kind);
    let key = key_for(question, &question.name);
    let titles: Vec<String> = question
        .candidates
        .iter()
        .map(|candidate| format!("{} › {key}", key_for(question, candidate)))
        .collect();
    let width = titles
        .iter()
        .map(|title| title.chars().count())
        .max()
        .unwrap_or(0);

    let mut choices = vec![Choice {
        answer: RenameAnswer::Create,
        label: format!(
            "{} {key:<width$} {}",
            "+".green(),
            format!("create {kind}").bright_black()
        ),
    }];
    for (candidate, title) in question.candidates.iter().zip(titles) {
        let pad = width.saturating_sub(title.chars().count());
        choices.push(Choice {
            answer: RenameAnswer::RenameFrom(candidate.clone()),
            label: format!(
                "{} {title}{} {}",
                "~".yellow(),
                " ".repeat(pad),
                format!("rename {kind}").bright_black()
            ),
        });
    }
    choices
}

/// The line printed after a decision (drizzle-kit's `applySelection` log).
fn selection_line(question: &RenameQuestion, answer: &RenameAnswer) -> String {
    let kind = humanize(question.kind);
    match answer {
        RenameAnswer::Create => format!(
            "{} {} {}",
            "+".green(),
            question.name,
            format!("{kind} will be created").bright_black()
        ),
        RenameAnswer::RenameFrom(from) => format!(
            "{} {} › {} {}",
            "~".yellow(),
            key_for(question, from),
            key_for(question, &question.name),
            format!("{kind} will be renamed/moved").bright_black()
        ),
    }
}

/// drizzle-kit's missing-hints report (`missing-hints-report.ts`), plus the
/// deleted candidates of each question.
#[must_use]
pub fn missing_hints_report(missing: &[RenameQuestion]) -> String {
    let mut lines = vec![
        format!("missing_hints: {} unresolved decisions", missing.len())
            .yellow()
            .bold()
            .to_string(),
    ];
    for (index, question) in missing.iter().enumerate() {
        let kind = question.kind.as_str();
        let id = entity_id(question, &question.name);
        let placeholder: Vec<String> = match id.len() {
            1 => vec!["<old_name>".into()],
            2 => vec!["<schema>".into(), "<old_name>".into()],
            _ => vec!["<schema>".into(), "<table>".into(), "<old_name>".into()],
        };
        let candidates: Vec<String> = question
            .candidates
            .iter()
            .map(|candidate| json_tuple(&entity_id(question, candidate)))
            .collect();
        lines.push(String::new());
        lines.push(format!(
            "{}. Rename or create  {}  {}  {}",
            index + 1,
            "—".bright_black(),
            humanize(question.kind),
            id.join(".").bold().cyan()
        ));
        lines.push(format!(
            "   {} {}",
            "Deleted candidates:".bright_black(),
            candidates.join(", ")
        ));
        lines.push("   Add to --hints:".to_string());
        lines.push(format!(
            "     {{ \"type\": \"rename\", \"kind\": \"{kind}\", \"from\": {}, \"to\": {} }}",
            json_tuple(&placeholder),
            json_tuple(&id)
        ));
        lines.push("     OR".to_string());
        lines.push(format!(
            "     {{ \"type\": \"create\", \"kind\": \"{kind}\", \"entity\": {} }}",
            json_tuple(&id)
        ));
    }
    lines.push(String::new());
    lines.push(
        "Re-run with --hints '<json-array>' or --hints-file <path>. Exit code 2."
            .bright_black()
            .to_string(),
    );
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_migrations::sqlite::SQLiteSnapshot;
    use drizzle_migrations::sqlite::ddl::{Column, SqliteEntity, Table};

    fn plain(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn sqlite(tables: &[(&'static str, &[&'static str])]) -> Snapshot {
        let mut snapshot = SQLiteSnapshot::new();
        for (table, columns) in tables {
            snapshot.add_entity(SqliteEntity::Table(Table::new(*table)));
            for column in *columns {
                snapshot.add_entity(SqliteEntity::Column(
                    Column::new(*table, *column, "text").not_null(),
                ));
            }
        }
        Snapshot::Sqlite(snapshot)
    }

    fn question(
        kind: RenameKind,
        schema: Option<&str>,
        table: Option<&str>,
        name: &str,
        candidates: &[&str],
    ) -> RenameQuestion {
        RenameQuestion {
            kind,
            schema: schema.map(str::to_string),
            table: table.map(str::to_string),
            name: name.to_string(),
            candidates: candidates.iter().map(|c| (*c).to_string()).collect(),
        }
    }

    /// Answers from a script and records what it was asked.
    struct Scripted {
        answers: Vec<usize>,
        asked: Vec<(String, Vec<String>)>,
    }

    impl RenamePrompt for Scripted {
        fn ask(&mut self, question: &RenameQuestion) -> Result<RenameAnswer, CliError> {
            let choices = prompt_choices(question);
            self.asked.push((
                plain(&prompt_message(question)),
                choices.iter().map(|c| plain(&c.to_string())).collect(),
            ));
            let pick = self.answers.remove(0);
            Ok(choices[pick].answer.clone())
        }
    }

    #[test]
    fn renders_like_drizzle_kit() {
        let q = question(
            RenameKind::Table,
            None,
            None,
            "accounts",
            &["users", "people"],
        );
        assert_eq!(
            plain(&prompt_message(&q)),
            "Is accounts table created or renamed from another table?"
        );
        let choices: Vec<String> = prompt_choices(&q)
            .iter()
            .map(|c| plain(&c.to_string()))
            .collect();
        assert_eq!(
            choices,
            [
                "+ accounts          create table",
                "~ users › accounts  rename table",
                "~ people › accounts rename table",
            ]
        );

        let q = question(
            RenameKind::Column,
            Some("app"),
            Some("users"),
            "full_name",
            &["name"],
        );
        assert_eq!(
            plain(&prompt_message(&q)),
            "Is app.users.full_name column created or renamed from another column?"
        );
        assert_eq!(
            plain(&prompt_choices(&q)[1].to_string()),
            "~ app.users.name › app.users.full_name rename column"
        );
        assert_eq!(
            entity_id(&q, "name"),
            ["app", "users", "name"].map(String::from)
        );
        assert_eq!(
            entity_id(&question(RenameKind::Table, None, None, "t", &["x"]), "t"),
            ["public", "t"].map(String::from)
        );
    }

    #[test]
    fn interactive_answers_apply_in_order_and_consume_candidates() {
        let prev = sqlite(&[("a", &["id"]), ("b", &["id", "old"])]);
        let cur = sqlite(&[("c", &["id"]), ("d", &["id", "new"])]);
        // c: rename from b (choice 2); d: only `a` is left, create (choice 0).
        // c (formerly b) only lost a column, so no column question follows.
        let mut prompt = Scripted {
            answers: vec![2, 0],
            asked: Vec::new(),
        };
        let resolution = resolve_renames(
            &prev,
            &cur,
            DiffOptions::new(),
            &[],
            Mode::Interactive(&mut prompt),
        )
        .unwrap();

        assert_eq!(prompt.asked.len(), 2);
        assert_eq!(
            prompt.asked[0].1,
            [
                "+ c     create table",
                "~ a › c rename table",
                "~ b › c rename table",
            ]
        );
        assert_eq!(
            prompt.asked[1].1,
            ["+ d     create table", "~ a › d rename table"],
            "b was consumed by the first answer"
        );
        assert!(resolution.missing.is_empty());
        assert!(!resolution.options.infer_renames);
        let plan = drizzle_migrations::diff_with(&prev, &cur, &resolution.options).unwrap();
        assert!(
            plan.statements
                .contains(&"ALTER TABLE `b` RENAME TO `c`;".to_string())
        );
        assert!(
            plan.statements
                .iter()
                .any(|s| s.starts_with("CREATE TABLE `d`"))
        );
        assert!(
            plan.statements
                .iter()
                .any(|s| s.starts_with("DROP TABLE `a`"))
        );
    }

    #[test]
    fn column_questions_use_the_renamed_table() {
        let prev = sqlite(&[("users", &["id", "name"])]);
        let cur = sqlite(&[("accounts", &["id", "full_name"])]);
        let mut prompt = Scripted {
            answers: vec![1, 1],
            asked: Vec::new(),
        };
        let resolution = resolve_renames(
            &prev,
            &cur,
            DiffOptions::new(),
            &[],
            Mode::Interactive(&mut prompt),
        )
        .unwrap();
        assert_eq!(
            prompt.asked[1].0,
            "Is accounts.full_name column created or renamed from another column?"
        );
        let plan = drizzle_migrations::diff_with(&prev, &cur, &resolution.options).unwrap();
        assert_eq!(
            plan.statements,
            [
                "ALTER TABLE `users` RENAME TO `accounts`;",
                "ALTER TABLE `accounts` RENAME COLUMN `name` TO `full_name`;",
            ]
        );
    }

    #[test]
    fn hints_answer_without_prompting_and_unanswered_are_missing() {
        let prev = sqlite(&[("users", &["id", "name"]), ("logs", &["id", "body"])]);
        let cur = sqlite(&[("accounts", &["id", "name"]), ("logs", &["id", "text"])]);
        let hints = parse_hints(&serde_json::json!([
            {"type": "rename", "kind": "table", "from": ["public", "users"], "to": ["public", "accounts"]},
            {"type": "rename", "kind": "table", "from": ["public", "x"], "to": ["public", "unrelated"]},
        ]))
        .unwrap();

        let resolution =
            resolve_renames(&prev, &cur, DiffOptions::new(), &hints, Mode::Hints).unwrap();
        assert_eq!(
            resolution.missing,
            [question(
                RenameKind::Column,
                None,
                Some("logs"),
                "text",
                &["body"]
            )]
        );
        let report = plain(&missing_hints_report(&resolution.missing));
        assert!(
            report.starts_with("missing_hints: 1 unresolved decisions\n"),
            "{report}"
        );
        assert!(report.contains("1. Rename or create  —  column  public.logs.text"));
        assert!(report.contains("Deleted candidates: [\"public\", \"logs\", \"body\"]"));
        assert!(report.contains(
            r#"{ "type": "rename", "kind": "column", "from": ["<schema>", "<table>", "<old_name>"], "to": ["public", "logs", "text"] }"#
        ));
        assert!(report.contains(
            r#"{ "type": "create", "kind": "column", "entity": ["public", "logs", "text"] }"#
        ));

        // The same hints in an interactive run skip the hinted question.
        let mut prompt = Scripted {
            answers: vec![0],
            asked: Vec::new(),
        };
        resolve_renames(
            &prev,
            &cur,
            DiffOptions::new(),
            &hints,
            Mode::Interactive(&mut prompt),
        )
        .unwrap();
        assert_eq!(prompt.asked.len(), 1);
        assert!(prompt.asked[0].0.starts_with("Is logs.text column"));
    }

    #[test]
    fn create_hints_keep_drop_and_create() {
        let prev = sqlite(&[("users", &["id"])]);
        let cur = sqlite(&[("accounts", &["id"])]);
        let hints = parse_hints(&serde_json::json!([
            {"type": "create", "kind": "table", "entity": ["public", "accounts"]},
        ]))
        .unwrap();
        let resolution =
            resolve_renames(&prev, &cur, DiffOptions::new(), &hints, Mode::Hints).unwrap();
        assert!(resolution.missing.is_empty());
        let plan = drizzle_migrations::diff_with(&prev, &cur, &resolution.options).unwrap();
        assert!(!plan.statements.iter().any(|s| s.contains("RENAME")));
    }

    #[test]
    fn rename_hint_from_a_non_candidate_is_rejected() {
        let prev = sqlite(&[("users", &["id"])]);
        let cur = sqlite(&[("accounts", &["id"])]);
        let hints = parse_hints(&serde_json::json!([
            {"type": "rename", "kind": "table", "from": ["public", "people"], "to": ["public", "accounts"]},
        ]))
        .unwrap();
        let error =
            resolve_renames(&prev, &cur, DiffOptions::new(), &hints, Mode::Hints).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid hints: rename hint's `from` [\"public\", \"people\"] doesn't match any deleted table"
        );
    }

    #[test]
    fn hint_shapes_are_validated() {
        for (json, message) in [
            (serde_json::json!({}), "at <root>: expected an array"),
            (
                serde_json::json!([{"type": "rename", "kind": "table", "from": ["users"], "to": ["public", "a"]}]),
                "at [0].from: expected an array of 2 strings",
            ),
            (
                serde_json::json!([{"type": "create", "kind": "tables", "entity": ["public", "a"]}]),
                "at [0].kind: unknown kind \"tables\"",
            ),
            (
                serde_json::json!([{"type": "create", "kind": "table", "entity": ["public", "a"], "x": 1}]),
                "unrecognized key \"x\"",
            ),
        ] {
            let error = parse_hints(&json).unwrap_err().to_string();
            assert!(error.contains(message), "{error}");
        }
        assert_eq!(
            parse_hints(&serde_json::json!([
                {"type": "confirm_data_loss", "kind": "table", "entity": ["public", "a"]},
                {"type": "rename", "kind": "foreign key", "from": ["public", "t", "a"], "to": ["public", "t", "b"]},
            ]))
            .unwrap()
            .len(),
            2,
            "kinds drizzle-kit knows parse even when this CLI never asks about them"
        );
    }
}
