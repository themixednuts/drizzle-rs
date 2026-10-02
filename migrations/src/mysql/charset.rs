//! MySQL 8 character-set facts the planner and introspection need.

/// The collation `MySQL` 8 gives a column or table that declares
/// `CHARACTER SET charset` without `COLLATE`
/// (`INFORMATION_SCHEMA.CHARACTER_SETS.DEFAULT_COLLATE_NAME`). `None` for a
/// character set this table does not know.
pub(crate) fn default_collation(charset: &str) -> Option<&'static str> {
    const DEFAULTS: &[(&str, &str)] = &[
        ("armscii8", "armscii8_general_ci"),
        ("ascii", "ascii_general_ci"),
        ("big5", "big5_chinese_ci"),
        ("binary", "binary"),
        ("cp1250", "cp1250_general_ci"),
        ("cp1251", "cp1251_general_ci"),
        ("cp1256", "cp1256_general_ci"),
        ("cp1257", "cp1257_general_ci"),
        ("cp850", "cp850_general_ci"),
        ("cp852", "cp852_general_ci"),
        ("cp866", "cp866_general_ci"),
        ("cp932", "cp932_japanese_ci"),
        ("dec8", "dec8_swedish_ci"),
        ("eucjpms", "eucjpms_japanese_ci"),
        ("euckr", "euckr_korean_ci"),
        ("gb18030", "gb18030_chinese_ci"),
        ("gb2312", "gb2312_chinese_ci"),
        ("gbk", "gbk_chinese_ci"),
        ("geostd8", "geostd8_general_ci"),
        ("greek", "greek_general_ci"),
        ("hebrew", "hebrew_general_ci"),
        ("hp8", "hp8_english_ci"),
        ("keybcs2", "keybcs2_general_ci"),
        ("koi8r", "koi8r_general_ci"),
        ("koi8u", "koi8u_general_ci"),
        ("latin1", "latin1_swedish_ci"),
        ("latin2", "latin2_general_ci"),
        ("latin5", "latin5_turkish_ci"),
        ("latin7", "latin7_general_ci"),
        ("macce", "macce_general_ci"),
        ("macroman", "macroman_general_ci"),
        ("sjis", "sjis_japanese_ci"),
        ("swe7", "swe7_swedish_ci"),
        ("tis620", "tis620_thai_ci"),
        ("ucs2", "ucs2_general_ci"),
        ("ujis", "ujis_japanese_ci"),
        ("utf16", "utf16_general_ci"),
        ("utf16le", "utf16le_general_ci"),
        ("utf32", "utf32_general_ci"),
        ("utf8", "utf8mb3_general_ci"),
        ("utf8mb3", "utf8mb3_general_ci"),
        ("utf8mb4", "utf8mb4_0900_ai_ci"),
    ];
    DEFAULTS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(charset.trim()))
        .map(|(_, collation)| *collation)
}

/// The collation that applies to an object declaring `charset` and
/// `collation`, inheriting `inherited` when it declares neither.
pub(crate) fn effective_collation(
    charset: Option<&str>,
    collation: Option<&str>,
    inherited: Option<&str>,
) -> Option<String> {
    match (charset, collation) {
        (_, Some(collation)) => Some(collation.to_ascii_lowercase()),
        (Some(charset), None) => default_collation(charset).map(str::to_string),
        (None, None) => inherited.map(str::to_ascii_lowercase),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_sets_resolve_their_default_collation() {
        assert_eq!(default_collation("latin1"), Some("latin1_swedish_ci"));
        assert_eq!(default_collation("UTF8MB4"), Some("utf8mb4_0900_ai_ci"));
        assert_eq!(default_collation("unknown"), None);
        assert_eq!(
            effective_collation(Some("latin1"), None, Some("utf8mb4_0900_ai_ci")).as_deref(),
            Some("latin1_swedish_ci")
        );
        assert_eq!(
            effective_collation(None, None, Some("utf8mb4_bin")).as_deref(),
            Some("utf8mb4_bin")
        );
    }
}
