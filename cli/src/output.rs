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

/// Hides the secrets in a connection URL: the password of `user:password@`
/// and the value of `authToken`, `password`, `token` and similar query
/// parameters.
#[must_use]
pub fn mask_url(url: &str) -> String {
    let (base, query) = match url.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (url, None),
    };

    let mut masked = String::with_capacity(url.len());
    let scheme_end = base.find("://").map_or(0, |position| position + 3);
    match base
        .rfind('@')
        .filter(|&at| at > scheme_end)
        .and_then(|at| {
            base[scheme_end..at]
                .find(':')
                .map(|colon| (scheme_end + colon, at))
        }) {
        Some((colon, at)) => {
            masked.push_str(&base[..=colon]);
            masked.push_str("****");
            masked.push_str(&base[at..]);
        }
        None => masked.push_str(base),
    }

    if let Some(query) = query {
        masked.push('?');
        for (index, pair) in query.split('&').enumerate() {
            if index > 0 {
                masked.push('&');
            }
            match pair.split_once('=') {
                Some((key, _)) if is_secret_parameter(key) => {
                    masked.push_str(key);
                    masked.push_str("=****");
                }
                _ => masked.push_str(pair),
            }
        }
    }
    masked
}

fn is_secret_parameter(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace(['_', '-'], "");
    matches!(
        key.as_str(),
        "authtoken" | "token" | "password" | "pass" | "passwd" | "pwd" | "secret" | "apikey"
    )
}

#[cfg(test)]
mod mask_url_tests {
    use super::mask_url;

    #[test]
    fn masks_passwords_and_secret_parameters() {
        assert_eq!(
            mask_url("postgres://user:secret@localhost:5432/db"),
            "postgres://user:****@localhost:5432/db"
        );
        assert_eq!(
            mask_url("postgres://user:pa:ss@localhost/db?sslmode=require"),
            "postgres://user:****@localhost/db?sslmode=require"
        );
        assert_eq!(
            mask_url("libsql://app.turso.io?authToken=abc.def&tls=true"),
            "libsql://app.turso.io?authToken=****&tls=true"
        );
        assert_eq!(
            mask_url("mysql://root@localhost/app?password=hunter2"),
            "mysql://root@localhost/app?password=****"
        );
        assert_eq!(mask_url("file:local.db"), "file:local.db");
    }
}
