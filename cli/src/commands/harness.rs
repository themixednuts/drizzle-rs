//! Shared command preamble helpers.
//!
//! Every database-touching command opens with the same multi-db header
//! ("Database: foo" when the config has multiple `[databases.*]` blocks).
//! Concentrating that here removes the repeated five-line block from each
//! command body — they call [`print_db_header`] and move on.

use crate::config::Config;
use crate::output;

/// Print the "Database: <name>" line when the config holds more than one
/// database. No-op on single-database configs.
pub fn print_db_header(config: &Config, db_name: Option<&str>) {
    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", output::label("Database"), name);
    }
}
