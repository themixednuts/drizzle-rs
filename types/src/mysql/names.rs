//! Default names for `MySQL` constraints the schema producers derive.
//!
//! `MySQL` identifiers are at most 64 characters long (error 1059), so a
//! derived name that would be longer is shortened with a deterministic hash,
//! the way drizzle-kit's `defaultNameForFK` does.

use crate::alloc_prelude::{String, Vec, format};

/// Longest identifier `MySQL` accepts.
pub const MAX_IDENTIFIER_LENGTH: usize = 64;

const DICTIONARY: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// drizzle-kit's deterministic alphanumeric hash of `input`, `len`
/// characters long.
#[must_use]
pub fn hash(input: &str, len: usize) -> String {
    let dict_len = DICTIONARY.len() as u128;
    let combinations = dict_len.pow(u32::try_from(len).unwrap_or(u32::MAX));
    let mut power: u128 = 1;
    let mut value: u128 = 0;
    for character in input.chars() {
        value = (value + u128::from(u32::from(character)) * power) % combinations;
        power = (power * 53) % combinations;
    }
    let mut digits = Vec::with_capacity(len);
    for _ in 0..len {
        digits.push(char::from(
            DICTIONARY[usize::try_from(value % dict_len).unwrap_or(0)],
        ));
        value /= dict_len;
    }
    digits.into_iter().rev().collect()
}

/// The default name of a foreign key on `table` over `columns`:
/// `{table}_{columns joined by _}_fkey`, hash-shortened to fit in 64
/// characters.
///
/// # Examples
///
/// ```
/// use drizzle_types::mysql::names::foreign_key_name;
///
/// assert_eq!(foreign_key_name("posts", &["author_id"]), "posts_author_id_fkey");
/// assert!(foreign_key_name(&"t".repeat(40), &["a_rather_long_column_name"]).len() <= 64);
/// ```
#[must_use]
pub fn foreign_key_name(table: &str, columns: &[&str]) -> String {
    let desired = format!("{table}_{}_fkey", columns.join("_"));
    if desired.chars().count() <= MAX_IDENTIFIER_LENGTH {
        return desired;
    }
    // `_` + 12-character hash + `_fkey`.
    if table.chars().count() + 18 <= MAX_IDENTIFIER_LENGTH {
        format!("{table}_{}_fkey", hash(&desired, 12))
    } else {
        format!("{}_fkey", hash(&desired, 12))
    }
}

pub use crate::names::composite_foreign_key_name_columns;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_drizzle_kit() {
        // Vectors computed with drizzle-kit's `hash` (dialects/common.ts).
        assert_eq!(hash("users_id_fkey", 12), "IFSdtU1x7svu");
        assert_eq!(
            hash(
                "audit_my_l1_organization_membership_invitations_inviting_organization_parent_id_fkey",
                12
            ),
            "vjgRHL25Jj6Q"
        );
        assert_eq!(hash("", 12), "000000000000");
        assert_eq!(hash("a", 12).len(), 12);
        assert_eq!(hash("abc", 12), hash("abc", 12));
        assert_ne!(hash("abc", 12), hash("abd", 12));
    }

    #[test]
    fn long_foreign_key_names_fit_mysql_identifiers() {
        let table = "audit_my_l1_organization_membership_invitations";
        let name = foreign_key_name(table, &["inviting_organization_parent_id"]);
        assert!(name.len() <= MAX_IDENTIFIER_LENGTH, "{name}");
        assert_eq!(name, "vjgRHL25Jj6Q_fkey");
        assert_eq!(
            name,
            foreign_key_name(table, &["inviting_organization_parent_id"])
        );

        let medium = foreign_key_name("audit_my_l1_children", &["x".repeat(50).as_str()]);
        assert!(medium.starts_with("audit_my_l1_children_"), "{medium}");
        assert_eq!(medium.len(), "audit_my_l1_children".len() + 18);

        let long_table = "t".repeat(60);
        let name = foreign_key_name(&long_table, &["id"]);
        assert_eq!(name.len(), 17);
        assert!(name.ends_with("_fkey"));

        let exact = format!("{}_id_fkey", "x".repeat(56));
        assert_eq!(exact.len(), 64);
        assert_eq!(foreign_key_name(&"x".repeat(56), &["id"]), exact);
    }
}
