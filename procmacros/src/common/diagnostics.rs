//! Shared diagnostic helpers for macros.

pub fn references_required_message(on_delete: bool, on_update: bool) -> String {
    if on_delete && on_update {
        "on_delete and on_update require a references attribute.\n\
         Example: #[column(references = Table::column, on_delete = CASCADE, on_update = CASCADE)]"
            .to_string()
    } else if on_delete {
        "on_delete requires a references attribute.\n\
         Example: #[column(references = Table::column, on_delete = CASCADE)]"
            .to_string()
    } else {
        "on_update requires a references attribute.\n\
         Example: #[column(references = Table::column, on_update = CASCADE)]"
            .to_string()
    }
}

/// Error when `relation = "..."` is set without a foreign key reference.
pub fn relation_requires_references_message() -> &'static str {
    "relation requires a references attribute.\n\
     Example: #[column(references = Table::column, relation = \"posts\")]"
}
