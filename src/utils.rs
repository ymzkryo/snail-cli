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

pub fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect()
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
