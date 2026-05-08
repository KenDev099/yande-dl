/// Tags are normalized to trimmed lowercase. Moebooru tags are already
/// lowercase by convention; this only regularizes user input so that
/// `Foo` and `foo` are treated as the same subscription.
pub fn normalize_tag(input: &str) -> String {
    input.trim().to_lowercase()
}

/// Convert a tag to a cross-platform-safe folder segment.
///
/// The original tag is preserved in `tags.json`'s `tag` field; this output
/// is used only for the on-disk folder name.
///
/// Rules:
/// 1. Replace Windows-unsafe `< > : " / \ | ? *` and control chars with `_`.
/// 2. Replace any whitespace with `_`.
/// 3. Collapse runs of `_` into a single `_`.
/// 4. Strip leading/trailing `.` and `_` (Windows disallows them as filename
///    boundaries).
/// 5. Truncate to 120 unicode code points.
/// 6. Empty result falls back to `_`.
pub fn safe_folder_segment(tag: &str) -> String {
    let bad = |c: char| {
        matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 0x20
    };

    let mut out: String = tag
        .chars()
        .map(|c| if bad(c) || c.is_whitespace() { '_' } else { c })
        .collect();

    // Collapse repeated underscores.
    while out.contains("__") {
        out = out.replace("__", "_");
    }

    // Strip leading/trailing `.` and `_`.
    let trimmed: String = out.trim_matches(|c: char| c == '.' || c == '_').to_string();

    // Truncate (unicode-safe).
    let truncated: String = if trimmed.chars().count() > 120 {
        trimmed.chars().take(120).collect()
    } else {
        trimmed
    };

    if truncated.is_empty() {
        "_".to_string()
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercases_and_trims() {
        assert_eq!(normalize_tag("  Stella_Sora  "), "stella_sora");
        assert_eq!(normalize_tag("\tFOO\n"), "foo");
    }

    #[test]
    fn normalize_idempotent() {
        let n = normalize_tag(" Hatsune Miku ");
        assert_eq!(n, normalize_tag(&n));
    }

    #[test]
    fn safe_replaces_windows_unsafe() {
        assert_eq!(safe_folder_segment("rating:safe"), "rating_safe");
        assert_eq!(safe_folder_segment("a*b?c|d"), "a_b_c_d");
        assert_eq!(safe_folder_segment("foo<>bar"), "foo_bar");
    }

    #[test]
    fn safe_replaces_whitespace() {
        assert_eq!(safe_folder_segment("hatsune miku"), "hatsune_miku");
        assert_eq!(safe_folder_segment("a\tb"), "a_b");
    }

    #[test]
    fn safe_collapses_runs() {
        assert_eq!(safe_folder_segment("a   b"), "a_b");
        assert_eq!(safe_folder_segment("a***b"), "a_b");
    }

    #[test]
    fn safe_strips_leading_trailing() {
        assert_eq!(safe_folder_segment(".hidden."), "hidden");
        assert_eq!(safe_folder_segment("__a__"), "a");
        assert_eq!(safe_folder_segment("._mixed_."), "mixed");
    }

    #[test]
    fn safe_never_empty() {
        assert_eq!(safe_folder_segment(""), "_");
        assert_eq!(safe_folder_segment("***"), "_");
        assert_eq!(safe_folder_segment("..."), "_");
        assert_eq!(safe_folder_segment("___"), "_");
    }

    #[test]
    fn safe_truncates_long_input() {
        let long: String = "a".repeat(200);
        let out = safe_folder_segment(&long);
        assert_eq!(out.chars().count(), 120);
    }

    #[test]
    fn safe_unicode_safe_truncation() {
        // Each "你" is 3 bytes but 1 char.
        let long: String = "你".repeat(150);
        let out = safe_folder_segment(&long);
        assert_eq!(out.chars().count(), 120);
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn safe_preserves_unicode_chars() {
        assert_eq!(safe_folder_segment("初音ミク"), "初音ミク");
    }

    #[test]
    fn safe_replaces_control_chars() {
        assert_eq!(safe_folder_segment("a\x00b"), "a_b");
        assert_eq!(safe_folder_segment("a\x1fb"), "a_b");
    }
}
