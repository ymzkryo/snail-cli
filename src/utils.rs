use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn get_current_date(format: &str) -> String {
    Local::now().format(format).to_string()
}

/// Render a note from an optional base template plus a snippet template, then
/// write it out.
///
/// `tags` sets the frontmatter `tags:` list when non-empty; an empty slice
/// leaves whatever the template declares.
pub fn create_note(
    base_path: Option<&Path>,
    snip_path: &Path,
    output_path: &Path,
    replacements: &[(&str, &str)],
    tags: &[String],
) -> Result<()> {
    let content = match base_path {
        Some(base_path) => render_base_and_snip(base_path, snip_path, replacements)?,
        None => render_template(snip_path, replacements)?,
    };

    let content = set_frontmatter_tags(&content, tags);
    let content = trim_frontmatter_line_ends(&content);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::write(output_path, content)
        .with_context(|| format!("Failed to write file: {:?}", output_path))?;

    Ok(())
}

fn render_template(template_path: &Path, replacements: &[(&str, &str)]) -> Result<String> {
    if !template_path.exists() {
        anyhow::bail!("Template file not found: {:?}", template_path);
    }

    let template_content = fs::read_to_string(template_path)
        .with_context(|| format!("Failed to read template: {:?}", template_path))?;

    Ok(apply_replacements(&template_content, replacements))
}

fn render_base_and_snip(
    base_path: &Path,
    snip_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<String> {
    let base_content = fs::read_to_string(base_path)
        .with_context(|| format!("Failed to read base template: {:?}", base_path))?;
    let snip_content = fs::read_to_string(snip_path)
        .with_context(|| format!("Failed to read snip template: {:?}", snip_path))?;

    let snip_replaced = apply_replacements(&snip_content, replacements);
    let mut all_replacements: Vec<(&str, &str)> = replacements.to_vec();
    all_replacements.push(("body", &snip_replaced));

    Ok(apply_replacements(&base_content, &all_replacements))
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

/// Locate the frontmatter block, returning `(first_field_line, closing_fence)`
/// indices into `lines`.
fn frontmatter_bounds(lines: &[&str]) -> Option<(usize, usize)> {
    if lines.first() != Some(&"---") {
        return None;
    }
    let end = lines.iter().skip(1).position(|l| *l == "---")? + 1;
    Some((1, end))
}

/// Join lines back into a document, restoring the trailing newline when the
/// original had one.
fn join_lines(lines: &[String], had_trailing_newline: bool) -> String {
    let mut out = lines.join("\n");
    if had_trailing_newline {
        out.push('\n');
    }
    out
}

/// Drop trailing whitespace from every frontmatter line.
///
/// A template placeholder that expands to nothing (`status: {{STATUS}}` for a
/// memo) otherwise leaves `status: ` behind, which the vault's validator flags.
/// Only the frontmatter is touched, so a body relying on two-space Markdown
/// line breaks is left alone.
fn trim_frontmatter_line_ends(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some((start, end)) = frontmatter_bounds(&lines) else {
        return content.to_string();
    };

    let trimmed: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if (start..end).contains(&i) {
                line.trim_end().to_string()
            } else {
                line.to_string()
            }
        })
        .collect();

    join_lines(&trimmed, content.ends_with('\n'))
}

/// Set the frontmatter `tags:` list, replacing an existing entry (including a
/// block sequence) or inserting one before the closing fence.
fn set_frontmatter_tags(content: &str, tags: &[String]) -> String {
    if tags.is_empty() {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let Some((start, end)) = frontmatter_bounds(&lines) else {
        return content.to_string();
    };

    let rendered = format!("tags: [{}]", tags.join(", "));
    let mut out: Vec<String> = lines[..start].iter().map(|l| l.to_string()).collect();
    let mut replaced = false;
    let mut skipping_sequence = false;

    for line in &lines[start..end] {
        if line.starts_with("tags:") {
            out.push(rendered.clone());
            replaced = true;
            // A block sequence continues on the following indented `- ` lines.
            skipping_sequence = line.trim_end() == "tags:";
            continue;
        }
        if skipping_sequence {
            if line.starts_with(char::is_whitespace) && line.trim_start().starts_with('-') {
                continue;
            }
            skipping_sequence = false;
        }
        out.push(line.to_string());
    }

    if !replaced {
        out.push(rendered);
    }
    out.extend(lines[end..].iter().map(|l| l.to_string()));

    join_lines(&out, content.ends_with('\n'))
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
    fn frontmatter_trailing_space_is_trimmed() {
        let content = "---\nstatus: \ndate: 2026-09-06\n---\n\nbody\n";
        assert_eq!(
            trim_frontmatter_line_ends(content),
            "---\nstatus:\ndate: 2026-09-06\n---\n\nbody\n"
        );
    }

    #[test]
    fn trailing_space_outside_frontmatter_is_left_alone() {
        // Two trailing spaces are a Markdown line break; only frontmatter is touched.
        let content = "---\nstatus: \n---\n\nline one  \nline two\n";
        assert_eq!(
            trim_frontmatter_line_ends(content),
            "---\nstatus:\n---\n\nline one  \nline two\n"
        );
    }

    #[test]
    fn documents_without_frontmatter_are_untouched() {
        let content = "# heading  \nbody  \n";
        assert_eq!(trim_frontmatter_line_ends(content), content);
    }

    #[test]
    fn tags_replace_the_existing_frontmatter_entry() {
        let content = "---\nstatus: inbox\ntags: []\ncontext:\n---\n\n# t\n";
        assert_eq!(
            set_frontmatter_tags(content, &["type/todo".to_string(), "topic/rust".to_string()]),
            "---\nstatus: inbox\ntags: [type/todo, topic/rust]\ncontext:\n---\n\n# t\n"
        );
    }

    #[test]
    fn tags_are_inserted_when_the_template_has_none() {
        let content = "---\nstatus: inbox\n---\n\n# t\n";
        assert_eq!(
            set_frontmatter_tags(content, &["type/todo".to_string()]),
            "---\nstatus: inbox\ntags: [type/todo]\n---\n\n# t\n"
        );
    }

    #[test]
    fn tags_replace_a_block_sequence_entry() {
        let content = "---\ntags:\n  - old\n  - older\nstatus: inbox\n---\n";
        assert_eq!(
            set_frontmatter_tags(content, &["type/todo".to_string()]),
            "---\ntags: [type/todo]\nstatus: inbox\n---\n"
        );
    }

    #[test]
    fn no_tags_leaves_the_template_as_is() {
        let content = "---\ntags: []\n---\n";
        assert_eq!(set_frontmatter_tags(content, &[]), content);
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
