//! CLI output helpers for consistent, drizzle-kit-like formatting.

use colored::Colorize;

pub fn heading(text: &str) -> String {
    format!("{}", text.bright_cyan())
}

pub fn label(text: &str) -> String {
    format!("{}", text.bright_blue())
}

pub fn muted(text: &str) -> String {
    format!("{}", text.bright_black())
}

pub fn success(text: &str) -> String {
    format!("{}", text.bright_green())
}

pub fn warning(text: &str) -> String {
    format!("{}", text.yellow())
}

pub fn error(text: &str) -> String {
    format!("{}", text.red())
}

pub fn info(text: &str) -> String {
    format!("{} {}", "Info:".bright_blue().bold(), text)
}

pub fn warn_line(text: &str) -> String {
    format!("[{}] {}", "Warning".yellow(), text)
}

pub fn err_line(text: &str) -> String {
    format!("{} {}", "Error".red().bold(), text)
}

pub fn banner_invalid_input(text: &str) -> String {
    format!("{} {}", " Invalid input ".white().on_red(), text.red())
}

pub fn banner_warning(text: &str) -> String {
    format!("{} {}", " Warning ".white().on_bright_black(), text)
}

pub fn banner_error(text: &str) -> String {
    format!("{} {}", " Error ".white().on_red().bold(), text)
}

pub fn banner_suggestion(text: &str) -> String {
    format!("{} {}", " Suggestion ".white().on_bright_black(), text)
}

pub fn status_ok() -> String {
    format!("{}", "OK".green())
}

pub fn status_error() -> String {
    format!("{}", "ERROR".red())
}

pub fn status_warning(text: &str) -> String {
    format!("{}", text.yellow())
}
