use anyhow::{Context, Result};
use std::fs;
use crate::config::Config;
use crate::utils::{create_file_from_template, get_current_date, open_editor};

pub fn new(name: &str, config: &Config) -> Result<()> {
    let project_dir = config.project_dir()?;

    // Find the maximum project number
    let max_number = find_max_project_number(&project_dir)?;
    let new_number = max_number + 1;

    let new_project_dir = project_dir.join(format!("{:05}_{}", new_number, name));
    fs::create_dir_all(&new_project_dir)
        .with_context(|| format!("Failed to create project directory: {:?}", new_project_dir))?;

    let date = get_current_date(&config.general.date_format);
    let readme_filename = format!("{}-{}-README.md", date, name);
    let readme_path = new_project_dir.join(&readme_filename);

    let template_path = config.get_template_path("project")?;

    let replacements = vec![
        ("{{name}}", name),
        ("{{date}}", &date),
    ];

    create_file_from_template(&template_path, &readme_path, &replacements)?;

    println!("Created project: {}", new_project_dir.display());
    println!("README: {}", readme_path.display());

    open_editor(&readme_path, &config.general.editor)?;

    Ok(())
}

fn find_max_project_number(project_dir: &std::path::Path) -> Result<u32> {
    if !project_dir.exists() {
        return Ok(800);
    }

    let mut max_number = 800;

    for entry in fs::read_dir(project_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        // Parse pattern: 008XX_*
        if let Some(num_str) = name_str.split('_').next() {
            if let Ok(num) = num_str.parse::<u32>() {
                if num > max_number {
                    max_number = num;
                }
            }
        }
    }

    Ok(max_number)
}

pub fn list(_config: &Config) -> Result<()> {
    println!("Project list command - not yet implemented");
    Ok(())
}

pub fn show(_name: &str, _config: &Config) -> Result<()> {
    println!("Project show command - not yet implemented");
    Ok(())
}
