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

/// The candidate closest to `input` (case-insensitive edit distance), when it
/// is close enough to be a likely typo.
fn closest_match<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let input = input.to_ascii_lowercase();
    candidates
        .iter()
        .map(|candidate| {
            let distance = edit_distance(&input, &candidate.to_ascii_lowercase());
            (distance, *candidate)
        })
        .filter(|(distance, candidate)| *distance <= (input.len().max(candidate.len()) / 3).max(1))
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance between two ASCII-ish strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, a_char) in a.chars().enumerate() {
        let mut current = vec![i + 1; b.len() + 1];
        for (j, b_char) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(a_char != *b_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        previous = current;
    }
    previous[b.len()]
}

/// "unknown `kind` `key`", with the closest valid key as a suggestion, or the
/// valid keys when none is close.
pub fn unknown_key_message(kind: &str, key: &str, candidates: &[&str]) -> String {
    closest_match(key, candidates).map_or_else(
        || {
            format!(
                "unknown {kind} `{key}`; expected one of: {}",
                candidates.join(", ")
            )
        },
        |suggestion| format!("unknown {kind} `{key}`; did you mean `{suggestion}`?"),
    )
}

/// Rejects `#[derive(Clone | Copy | Debug | Default)]` beside a schema derive,
/// which implements those traits itself (a second impl is E0119).
///
/// A derive macro sees the other `#[derive]` attributes of its item, but not
/// the traits listed beside it in its own attribute; the derive's docs cover
/// that case.
pub fn reject_schema_trait_derives(input: &syn::DeriveInput, derive: &str) -> syn::Result<()> {
    for attr in &input.attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let Ok(paths) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };
        for path in paths {
            let Some(name) = path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
            else {
                continue;
            };
            if matches!(name.as_str(), "Clone" | "Copy" | "Debug" | "Default") {
                return Err(syn::Error::new_spanned(
                    path,
                    format!(
                        "`{derive}` already implements `{name}` for the schema; remove it from this derive"
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::unknown_key_message;

    #[test]
    fn suggests_the_closest_key() {
        let keys = ["primary", "unique", "autoincrement", "references"];
        assert_eq!(
            unknown_key_message("column attribute", "primay", &keys),
            "unknown column attribute `primay`; did you mean `primary`?"
        );
        assert_eq!(
            unknown_key_message("column attribute", "refrences", &keys),
            "unknown column attribute `refrences`; did you mean `references`?"
        );
        assert!(unknown_key_message("column attribute", "wat", &keys).contains("expected one of"));
    }
}
