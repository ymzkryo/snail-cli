use anyhow::Result;
use crate::config::Config;
use crate::utils::{create_file_from_template, get_current_date, open_editor, sanitize_filename};

pub fn new(title: &str, no_edit: bool, config: &Config) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let sanitized_title = sanitize_filename(title);
    let filename = format!("{}-{}.md", date, sanitized_title);

    let inbox_dir = config.inbox_dir()?;
    let file_path = inbox_dir.join(&filename);

    let template_path = config.get_template_path("memo")?;

    let replacements = vec![
        ("title", title),
        ("date", &date),
        ("kind", "memo"),
    ];

    create_file_from_template(&template_path, &file_path, &replacements)?;

    println!("Created memo: {}", file_path.display());

    if !no_edit {
        open_editor(&file_path, &config.general.editor)?;
    }

    Ok(())
}

pub fn list(_config: &Config) -> Result<()> {
    println!("Memo list command - not yet implemented");
    Ok(())
}

pub fn search(_keyword: &str, _config: &Config) -> Result<()> {
    println!("Memo search command - not yet implemented");
    Ok(())
}
