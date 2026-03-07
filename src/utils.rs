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
