use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn get_current_date(format: &str) -> String {
    Local::now().format(format).to_string()
}

pub fn create_file_from_template(
    template_path: &Path,
    output_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<()> {
    let template_content = if template_path.exists() {
        fs::read_to_string(template_path)
            .with_context(|| format!("Failed to read template: {:?}", template_path))?
    } else {
        anyhow::bail!("Template file not found: {:?}", template_path);
    };

    let content = apply_replacements(&template_content, replacements);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::write(output_path, content)
        .with_context(|| format!("Failed to write file: {:?}", output_path))?;

    Ok(())
}

pub fn create_file_from_base_and_snip(
    base_path: &Path,
    snip_path: &Path,
    output_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<()> {
    let base_content = fs::read_to_string(base_path)
        .with_context(|| format!("Failed to read base template: {:?}", base_path))?;
    let snip_content = fs::read_to_string(snip_path)
        .with_context(|| format!("Failed to read snip template: {:?}", snip_path))?;

    let snip_replaced = apply_replacements(&snip_content, replacements);
    let mut all_replacements: Vec<(&str, &str)> = replacements.to_vec();
    let body_key = "body";
    all_replacements.push((body_key, &snip_replaced));

    let content = apply_replacements(&base_content, &all_replacements);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::write(output_path, content)
        .with_context(|| format!("Failed to write file: {:?}", output_path))?;

    Ok(())
}

fn apply_replacements(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut content = template.to_string();
    for (key, value) in replacements {
        let lowercase_key = format!("{{{{{}}}}}", key);
        content = content.replace(&lowercase_key, value);

        let uppercase_key = format!("{{{{{}}}}}", key.to_uppercase());
        content = content.replace(&uppercase_key, value);
    }
    content
}

pub fn open_editor(file_path: &Path, editor: &str) -> Result<()> {
    Command::new(editor)
        .arg(file_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    Ok(())
}

/// Result of converting a user-supplied title into a filename component.
pub struct SanitizedName {
    /// The UNIX-safe filename component.
    pub value: String,
    /// True when the title had to be altered to become filename-safe.
    pub changed: bool,
}

/// Characters kept as-is in generated filenames: ASCII alphanumerics, `-`, `_`,
/// and Japanese (kana / kanji). Everything else needs escaping in a shell or a
/// path, so it gets replaced.
fn is_filename_safe(c: char) -> bool {
    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
        return true;
    }

    matches!(
        c as u32,
        0x3005..=0x3007         // 々 〆 〇
            | 0x3041..=0x309F   // hiragana
            | 0x30A1..=0x30FA   // katakana (U+30FB `・` deliberately excluded)
            | 0x30FC..=0x30FF   // ー ヽ ヾ ヿ
            | 0x3400..=0x4DBF   // CJK extension A
            | 0x4E00..=0x9FFF   // CJK unified ideographs
            | 0xF900..=0xFAFF   // CJK compatibility ideographs
            | 0x20000..=0x2FA1F // CJK extension B and beyond
    )
}

/// Fold full-width ASCII and the ideographic space to their half-width form, so
/// `ＡＢＣ` survives as `ABC` rather than collapsing into separators.
fn fold_fullwidth(c: char) -> char {
    match c as u32 {
        0x3000 => ' ',
        n @ 0xFF01..=0xFF5E => char::from_u32(n - 0xFEE0).unwrap_or(c),
        _ => c,
    }
}

/// Convert a title into a UNIX-safe filename component.
///
/// Unsafe characters become `-`, runs of `-` collapse into one, and leading /
/// trailing `-` are trimmed. Errors when nothing usable remains.
pub fn sanitize_filename(title: &str) -> Result<SanitizedName> {
    let mut value = String::with_capacity(title.len());

    for c in title.chars().map(fold_fullwidth) {
        let c = if is_filename_safe(c) { c } else { '-' };
        // Collapse separator runs, whether generated or typed by the user.
        if c == '-' && value.ends_with('-') {
            continue;
        }
        value.push(c);
    }

    let value = value.trim_matches('-').to_string();
    if value.is_empty() {
        anyhow::bail!("Cannot derive a filename from title: {:?}", title);
    }

    let changed = value != title;
    Ok(SanitizedName { value, changed })
}

/// Resolve the filename component for a new file.
///
/// Sanitizes by default and reports what changed; with `strict` set, a title
/// that is not already filename-safe is rejected instead.
pub fn filename_component(title: &str, strict: bool) -> Result<String> {
    let sanitized = sanitize_filename(title)?;

    if sanitized.changed {
        if strict {
            anyhow::bail!(
                "Title is not filename-safe: {:?}\n  sanitized form: {:?}\n  allowed: ASCII alphanumerics, '-', '_', and Japanese characters",
                title,
                sanitized.value
            );
        }
        eprintln!(
            "note: filename sanitized: {:?} -> {:?}",
            title, sanitized.value
        );
    }

    Ok(sanitized.value)
}

/// Quote a scalar value for safe inclusion in YAML frontmatter.
///
/// Returns the value unchanged when it is plain enough to be unquoted, and a
/// double-quoted, escaped form otherwise. Designed for single-line scalars
/// (title / project names from the CLI), not for multi-line content.
pub fn yaml_quote_value(value: &str) -> String {
    if needs_yaml_quoting(value) {
        let mut escaped = String::with_capacity(value.len() + 2);
        escaped.push('"');
        for c in value.chars() {
            match c {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(c),
            }
        }
        escaped.push('"');
        escaped
    } else {
        value.to_string()
    }
}

fn needs_yaml_quoting(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }

    // Leading / trailing whitespace would be stripped by the YAML parser.
    if value != value.trim() {
        return true;
    }

    // YAML reserved literals (case-insensitive set used by common parsers).
    const RESERVED: &[&str] = &[
        "null", "Null", "NULL", "~",
        "true", "True", "TRUE", "false", "False", "FALSE",
        "yes", "Yes", "YES", "no", "No", "NO",
        "on", "On", "ON", "off", "Off", "OFF",
    ];
    if RESERVED.contains(&value) {
        return true;
    }

    // Indicators that, as the first non-space char, change scalar meaning.
    if let Some(first) = value.chars().next() {
        if matches!(
            first,
            '-' | '?' | ':' | ',' | '[' | ']' | '{' | '}' | '#' | '&' |
            '*' | '!' | '|' | '>' | '\'' | '"' | '%' | '@' | '`' | '~'
        ) {
            return true;
        }
    }

    // Structural sequences that break flow-scalar parsing.
    if value.contains(": ") || value.ends_with(':') || value.contains(" #") {
        return true;
    }

    // Control characters — keep frontmatter on a single line.
    if value.chars().any(|c| c.is_control()) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sanitized(title: &str) -> String {
        sanitize_filename(title).unwrap().value
    }

    #[test]
    fn safe_titles_pass_through_unchanged() {
        assert_eq!(sanitized("snail-cli"), "snail-cli");
        assert_eq!(sanitized("面談メモ"), "面談メモ");
        assert_eq!(sanitized("snail_cli-v2"), "snail_cli-v2");
        assert!(!sanitize_filename("面談メモ").unwrap().changed);
    }

    #[test]
    fn fullwidth_brackets_and_spaces_are_replaced() {
        assert_eq!(
            sanitized("面談メモ（AI活用プロダクト開発案件）"),
            "面談メモ-AI活用プロダクト開発案件"
        );
        assert_eq!(sanitized("出張メモ（8/27-29 五反田）"), "出張メモ-8-27-29-五反田");
    }

    #[test]
    fn shell_unsafe_ascii_is_replaced() {
        assert_eq!(sanitized("fix: bug #12 (urgent!)"), "fix-bug-12-urgent");
        assert_eq!(sanitized("a/b\\c$d&e;f"), "a-b-c-d-e-f");
        assert_eq!(sanitized("quote \"me\" 'now'"), "quote-me-now");
    }

    #[test]
    fn fullwidth_alphanumerics_fold_to_halfwidth() {
        assert_eq!(sanitized("ＡＰＩ設計２０２６"), "API設計2026");
    }

    #[test]
    fn banned_japanese_punctuation_is_replaced() {
        assert_eq!(sanitized("設計・実装"), "設計-実装");
        assert_eq!(sanitized("メモ：まとめ"), "メモ-まとめ");
        assert_eq!(sanitized("全角\u{3000}スペース"), "全角-スペース");
    }

    #[test]
    fn prolonged_sound_mark_is_kept() {
        assert_eq!(sanitized("サーバーレビュー"), "サーバーレビュー");
    }

    #[test]
    fn separator_runs_collapse_and_edges_are_trimmed() {
        assert_eq!(sanitized("  hello   world  "), "hello-world");
        assert_eq!(sanitized("(((wrapped)))"), "wrapped");
        assert_eq!(sanitized("a - b"), "a-b");
    }

    #[test]
    fn changed_flag_tracks_rewriting() {
        assert!(sanitize_filename("hello world").unwrap().changed);
        assert!(!sanitize_filename("hello-world").unwrap().changed);
    }

    #[test]
    fn title_without_any_safe_character_is_rejected() {
        assert!(sanitize_filename("（）／").is_err());
        assert!(sanitize_filename("").is_err());
    }

    #[test]
    fn strict_mode_rejects_unsafe_titles_only() {
        assert_eq!(filename_component("hello-world", true).unwrap(), "hello-world");
        assert!(filename_component("hello world", true).is_err());
        assert_eq!(filename_component("hello world", false).unwrap(), "hello-world");
    }

    #[test]
    fn plain_values_are_unquoted() {
        assert_eq!(yaml_quote_value("hello"), "hello");
        assert_eq!(yaml_quote_value("hello-world"), "hello-world");
        assert_eq!(yaml_quote_value("日本語タイトル"), "日本語タイトル");
        assert_eq!(yaml_quote_value("2026-05-03"), "2026-05-03");
    }

    #[test]
    fn empty_is_quoted() {
        assert_eq!(yaml_quote_value(""), "\"\"");
    }

    #[test]
    fn colon_followed_by_space_is_quoted() {
        assert_eq!(yaml_quote_value("Phase 1: setup"), "\"Phase 1: setup\"");
    }

    #[test]
    fn trailing_colon_is_quoted() {
        assert_eq!(yaml_quote_value("note:"), "\"note:\"");
    }

    #[test]
    fn leading_indicators_are_quoted() {
        assert_eq!(yaml_quote_value("- item"), "\"- item\"");
        assert_eq!(yaml_quote_value("? maybe"), "\"? maybe\"");
        assert_eq!(yaml_quote_value("# header"), "\"# header\"");
        assert_eq!(yaml_quote_value("@mention"), "\"@mention\"");
    }

    #[test]
    fn reserved_literals_are_quoted() {
        assert_eq!(yaml_quote_value("true"), "\"true\"");
        assert_eq!(yaml_quote_value("Null"), "\"Null\"");
        assert_eq!(yaml_quote_value("yes"), "\"yes\"");
    }

    #[test]
    fn embedded_quotes_and_backslash_are_escaped_when_quoting_triggered() {
        // Triggered by leading double-quote indicator; the embedded `\` and `"`
        // must be escaped inside the resulting double-quoted scalar.
        assert_eq!(yaml_quote_value(r#""a" b \ c"#), r#""\"a\" b \\ c""#);
    }

    #[test]
    fn space_hash_comment_is_quoted() {
        assert_eq!(yaml_quote_value("title #1"), "\"title #1\"");
    }

    #[test]
    fn surrounding_whitespace_is_quoted() {
        assert_eq!(yaml_quote_value(" leading"), "\" leading\"");
        assert_eq!(yaml_quote_value("trailing "), "\"trailing \"");
    }

    #[test]
    fn newline_is_escaped() {
        assert_eq!(yaml_quote_value("line1\nline2"), "\"line1\\nline2\"");
    }
}
