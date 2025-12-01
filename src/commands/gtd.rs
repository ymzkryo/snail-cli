use anyhow::{Context, Result};
use std::fs;
use crate::config::Config;
use crate::utils::{get_current_date, open_editor};

pub fn today_list(_config: &Config) -> Result<()> {
    println!("GTD today list command - not yet implemented");
    Ok(())
}

pub fn today_add(task: &str, config: &Config) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let filename = format!("{}-daily_report.md", date);

    let inbox_dir = config.inbox_dir()?;
    let file_path = inbox_dir.join(&filename);

    if !file_path.exists() {
        // Create new daily report from template
        let template_path = config.get_template_path("daily_report")?;

        let content = if template_path.exists() {
            fs::read_to_string(&template_path)
                .with_context(|| format!("Failed to read template: {:?}", template_path))?
                .replace("{{date}}", &date)
        } else {
            // Default template
            format!(
                "---\ndate: {}\n---\n\n# {} Daily Report\n\n## TODO\n\n## Done\n\n## Memo\n",
                date, date
            )
        };

        fs::write(&file_path, content)
            .with_context(|| format!("Failed to create daily report: {:?}", file_path))?;

        println!("Created daily report: {}", file_path.display());
    }

    // Add task to TODO section
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read daily report: {:?}", file_path))?;

    let new_task = format!("- [ ] {}", task);
    let updated_content = add_to_todo_section(&content, &new_task);

    fs::write(&file_path, updated_content)
        .with_context(|| format!("Failed to update daily report: {:?}", file_path))?;

    println!("Added task to daily report: {}", task);

    open_editor(&file_path, &config.general.editor)?;

    Ok(())
}

fn add_to_todo_section(content: &str, task: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_todo_section = false;
    let mut task_added = false;

    for line in lines {
        if line.trim() == "## TODO" {
            in_todo_section = true;
            result.push(line.to_string());
            continue;
        }

        if in_todo_section && line.starts_with("##") {
            // Reached next section, add task before it
            if !task_added {
                result.push(task.to_string());
                task_added = true;
            }
            in_todo_section = false;
        }

        result.push(line.to_string());
    }

    // If TODO section was last section
    if in_todo_section && !task_added {
        result.push(task.to_string());
    }

    result.join("\n")
}

pub fn weekly(_config: &Config) -> Result<()> {
    println!("GTD weekly review command - not yet implemented");
    Ok(())
}

pub fn monthly(_config: &Config) -> Result<()> {
    println!("GTD monthly review command - not yet implemented");
    Ok(())
}
