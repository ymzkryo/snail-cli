use anyhow::{Context, Result};
use chrono::{Datelike, Local};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::time::Instant;
use crate::config::Config;
use crate::utils::{get_current_date, open_editor};

pub fn today_list(config: &Config) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let filename = format!("{}-daily_report.md", date);

    let inbox_dir = config.inbox_dir()?;
    let file_path = inbox_dir.join(&filename);

    if !file_path.exists() {
        println!("No daily report found for today ({}).", date);
        return Ok(());
    }

    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read daily report: {:?}", file_path))?;

    println!("{} Daily Report:\n", date);

    let todos = extract_todo_section(&content);
    if todos.is_empty() {
        println!("No tasks in TODO section.");
    } else {
        for task in &todos {
            println!("{}", task);
        }
    }

    Ok(())
}

fn extract_todo_section(content: &str) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut todos = Vec::new();
    let mut in_todo_section = false;

    for line in lines {
        // Match various TODO section headers
        let line_lower = line.to_lowercase();
        if line.starts_with("## ") && (line_lower.contains("todo") || line.contains("ToDo")) {
            in_todo_section = true;
            continue;
        }

        if in_todo_section {
            if line.starts_with("## ") {
                break;
            }
            if line.starts_with("- ") {
                todos.push(line.to_string());
            }
        }
    }

    todos
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

pub fn weekly(config: &Config) -> Result<()> {
    let now = Local::now();
    let iso_week = now.iso_week();
    let iso_year = iso_week.year();
    let week_number = iso_week.week();
    let week_str = format!("W{:02}", week_number);

    let weekly_dir = config.weekly_report_dir()?;

    if !weekly_dir.exists() {
        println!("Weekly report directory not found: {}", weekly_dir.display());
        println!("\nPlease merge the weekly report PR first.");
        return Ok(());
    }

    // Search for file containing this week's number
    let mut found_file = None;
    for entry in fs::read_dir(&weekly_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains(&week_str) && name.ends_with(".md") {
                    found_file = Some(path);
                    break;
                }
            }
        }
    }

    match found_file {
        Some(_file_path) => {
            // Start braindump session
            braindump(config, &week_str)?;
        }
        None => {
            println!("No weekly report found for {} (Week {}).", iso_year, week_number);
            println!("\nPlease merge the weekly report PR first.");
        }
    }

    Ok(())
}

fn braindump(config: &Config, week_str: &str) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let filename = format!("{}-{}-braindump.md", date, week_str);
    let inbox_dir = config.inbox_dir()?;
    let file_path = inbox_dir.join(&filename);

    // Create file with header if it doesn't exist
    if !file_path.exists() {
        let header = format!("# {} Braindump\n\n", week_str);
        fs::write(&file_path, header)?;
    }

    let duration_mins = 10;
    let duration_secs = duration_mins * 60;
    let start = Instant::now();

    println!("\n=== Weekly Braindump ({} minutes) ===", duration_mins);
    println!("Write down everything on your mind. Press Enter after each thought.");
    println!("Session will end automatically after {} minutes.\n", duration_mins);

    loop {
        let elapsed = start.elapsed().as_secs();
        if elapsed >= duration_secs {
            println!("\n\nTime's up! Braindump session complete.");
            println!("Saved to: {}", file_path.display());
            break;
        }

        let remaining = duration_secs - elapsed;
        let remaining_mins = remaining / 60;
        let remaining_secs = remaining % 60;

        print!("[{:02}:{:02}] > ", remaining_mins, remaining_secs);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if !input.is_empty() {
            // Append to file
            let mut file = OpenOptions::new()
                .append(true)
                .open(&file_path)?;
            writeln!(file, "- {}", input)?;

            // Clear the previous line (move up and clear)
            print!("\x1b[1A\x1b[2K");
            io::stdout().flush()?;
        }
    }

    Ok(())
}

pub fn monthly(_config: &Config) -> Result<()> {
    println!("GTD monthly review command - not yet implemented");
    Ok(())
}
