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

/// Error when `relation = "..."` or `many_to_many = "..."` is set without a
/// foreign key reference. `key` is the attribute key, lowercase.
pub fn relation_requires_references_message(key: &str) -> String {
    format!(
        "{key} requires a references attribute.\n\
         Example: #[column(references = Table::column, {key} = \"posts\")]"
    )
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

/// A compile error at every schema field whose type an earlier field already
/// has: the schema would create that table, index or enum twice.
pub fn duplicate_schema_fields(fields: &[(&syn::Ident, &syn::Type)]) -> proc_macro2::TokenStream {
    let mut seen: Vec<(String, &syn::Ident)> = Vec::new();
    let mut errors = proc_macro2::TokenStream::new();
    for (field, ty) in fields {
        let key = quote::quote!(#ty).to_string();
        if let Some((_, first)) = seen.iter().find(|(seen, _)| *seen == key) {
            let msg = format!(
                "`{}` is already in this schema as `{first}`; list each table, index and enum once",
                key.replace(' ', "")
            );
            errors.extend(quote::quote_spanned! {syn::spanned::Spanned::span(ty)=>
                ::core::compile_error!(#msg);
            });
        } else {
            seen.push((key, field));
        }
    }
    errors
}

/// A compile-time check that no two schema items claim one name in one scope
/// (`SQLSchema::NAME_SCOPE`), such as two index structs both named
/// `users_email_idx`. The database would reject the second `CREATE`.
pub fn schema_name_check(
    schema: &syn::Ident,
    fields: &[(&syn::Ident, &syn::Type)],
    dialect: &crate::common::constraints::DialectTypes,
) -> proc_macro2::TokenStream {
    let sql_schema = &dialect.sql_schema;
    let schema_type = &dialect.schema_type;
    let value_type = &dialect.value_type;
    let types = fields.iter().map(|(_, ty)| ty);
    let messages = fields.iter().map(|(field, ty)| {
        format!(
            "`{field}` ({}) in `{schema}` has the same SQL name as an earlier item; give one \
             of them another `name`",
            quote::quote!(#ty).to_string().replace(' ', "")
        )
    });
    quote::quote! {
        const _: () = {
            const NAMES: &[::core::option::Option<(&str, &str)>] = &[
                #(<#types as #sql_schema<'static, #schema_type, #value_type<'static>>>::NAME_SCOPE,)*
            ];
            const MESSAGES: &[&str] = &[#(#messages),*];
            if let ::core::option::Option::Some(index) = drizzle::core::first_duplicate_name(NAMES) {
                ::core::panic!("{}", MESSAGES[index]);
            }
        };
    }
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
