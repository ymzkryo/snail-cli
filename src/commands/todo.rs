use anyhow::Result;
use crate::config::Config;
use crate::utils::{create_file_from_template, get_current_date, open_editor, sanitize_filename};

pub fn new(title: &str, project: Option<&str>, config: &Config) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let sanitized_title = sanitize_filename(title);
    let filename = format!("{}-{}.md", date, sanitized_title);

    let inbox_dir = config.inbox_dir()?;
    let file_path = inbox_dir.join(&filename);

    let template_path = config.get_template_path("todo")?;

    let project_str = project.unwrap_or("");
    let replacements = vec![
        ("title", title),
        ("date", &date),
        ("kind", "todo"),
        ("project", project_str),
    ];

    create_file_from_template(&template_path, &file_path, &replacements)?;

    println!("Created todo: {}", file_path.display());

    open_editor(&file_path, &config.general.editor)?;

    Ok(())
}

pub fn list(filters: &[String], _config: &Config) -> Result<()> {
    if filters.is_empty() {
        println!("Todo list command - not yet implemented");
    } else {
        println!("Todo list with filters: {:?} - not yet implemented", filters);
    }
    Ok(())
}

pub fn done(_file: &str, _config: &Config) -> Result<()> {
    println!("Todo done command - not yet implemented");
    Ok(())
}
