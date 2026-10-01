//! `PostgreSQL`-only operators: arrays (`@>`, `<@`, `&&`), JSON and JSONB
//! (`->`, `->>`, `#>`, `#>>`, `@>`, `?`, ...), `ILIKE`, and POSIX regular
//! expressions (`~`, `~*`, ...).
//!
//! Each operator is a free function and, through an extension trait
//! ([`ArrayExprExt`], [`JsonExprExt`], [`RegexExprExt`]), a method on any
//! `PostgreSQL` expression. Operand types are checked at compile time; each
//! function lists what it accepts. Portable operators (`eq`, `like`, `and`, ...)
//! live in `drizzle_core::expr`.

mod array_ops;
mod ilike;
mod json_ops;
mod regex;

pub use array_ops::*;
pub use ilike::*;
pub use json_ops::*;
pub use regex::*;
