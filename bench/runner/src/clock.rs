//! Timestamps for artifacts and events.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Current UTC time as RFC 3339, the format every artifact field uses.
///
/// Formatting cannot fail for a valid `OffsetDateTime`, but a benchmark run is
/// not worth aborting over a clock quirk, so the epoch stands in.
pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
