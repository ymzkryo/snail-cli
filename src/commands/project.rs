use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use crate::config::Config;
use crate::utils::{create_file_from_template, get_current_date, open_editor};

pub fn new(name: &str, no_edit: bool, config: &Config) -> Result<()> {
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
        ("name", name),
        ("date", &date),
    ];

    create_file_from_template(&template_path, &readme_path, &replacements)?;

    println!("Created project: {}", new_project_dir.display());
    println!("README: {}", readme_path.display());

    if !no_edit {
        open_editor(&readme_path, &config.general.editor)?;
    }

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

pub fn list(config: &Config) -> Result<()> {
    let project_dir = config.project_dir()?;

    if !project_dir.exists() {
        println!("No projects found.");
        return Ok(());
    }

    let mut projects: Vec<ProjectItem> = Vec::new();

    for entry in fs::read_dir(&project_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Parse project number and name (e.g., "00831_myproject")
            let (number, name) = if let Some(idx) = dir_name.find('_') {
                let num = dir_name[..idx].parse::<u32>().unwrap_or(0);
                let name = dir_name[idx + 1..].to_string();
                (num, name)
            } else {
                (0, dir_name.clone())
            };

            // Find README file
            let readme = find_readme(&path);

            projects.push(ProjectItem {
                number,
                name,
                path,
                readme,
            });
        }
    }

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    // Sort by project number
    projects.sort_by(|a, b| a.number.cmp(&b.number));

    // Display projects
    for (i, project) in projects.iter().enumerate() {
        println!("{}: {:05}_{}", i + 1, project.number, project.name);
    }

    println!("\nTotal: {} project(s)", projects.len());

    // Prompt for selection
    print!("Open README (1-{}, or Enter to skip): ", projects.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if !input.is_empty() {
        if let Ok(selection) = input.parse::<usize>() {
            if selection >= 1 && selection <= projects.len() {
                let project = &projects[selection - 1];
                if let Some(ref readme) = project.readme {
                    open_editor(readme, &config.general.editor)?;
                } else {
                    println!("No README found in project: {}", project.name);
                }
            } else {
                println!("Invalid selection: {}", selection);
            }
        }
    }

    Ok(())
}

struct ProjectItem {
    number: u32,
    name: String,
    path: PathBuf,
    readme: Option<PathBuf>,
}

fn find_readme(project_path: &PathBuf) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(project_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains("README") && name.ends_with(".md") {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

pub fn show(_name: &str, _config: &Config) -> Result<()> {
    println!("Project show command - not yet implemented");
    Ok(())
}
