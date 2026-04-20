//! CLI output helpers for consistent, drizzle-kit-like formatting.

use colored::Colorize;

#[must_use]
pub fn heading(text: &str) -> String {
    format!("{}", text.bright_cyan())
}

#[must_use]
pub fn label(text: &str) -> String {
    format!("{}", text.bright_blue())
}

#[must_use]
pub fn muted(text: &str) -> String {
    format!("{}", text.bright_black())
}

#[must_use]
pub fn success(text: &str) -> String {
    format!("{}", text.bright_green())
}

#[must_use]
pub fn warning(text: &str) -> String {
    format!("{}", text.yellow())
}

#[must_use]
pub fn error(text: &str) -> String {
    format!("{}", text.red())
}

#[must_use]
pub fn info(text: &str) -> String {
    format!("{} {}", "Info:".bright_blue().bold(), text)
}

#[must_use]
pub fn warn_line(text: &str) -> String {
    format!("[{}] {}", "Warning".yellow(), text)
}

#[must_use]
pub fn err_line(text: &str) -> String {
    format!("{} {}", "Error".red().bold(), text)
}

#[must_use]
pub fn banner_invalid_input(text: &str) -> String {
    format!("{} {}", " Invalid input ".white().on_red(), text.red())
}

#[must_use]
pub fn banner_warning(text: &str) -> String {
    format!("{} {}", " Warning ".white().on_bright_black(), text)
}

#[must_use]
pub fn banner_error(text: &str) -> String {
    format!("{} {}", " Error ".white().on_red().bold(), text)
}

#[must_use]
pub fn banner_suggestion(text: &str) -> String {
    format!("{} {}", " Suggestion ".white().on_bright_black(), text)
}

#[must_use]
pub fn status_ok() -> String {
    format!("{}", "OK".green())
}

#[must_use]
pub fn status_error() -> String {
    format!("{}", "ERROR".red())
}

#[must_use]
pub fn status_warning(text: &str) -> String {
    format!("{}", text.yellow())
}
