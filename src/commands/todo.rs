use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use crate::config::Config;
use crate::utils::{create_file_from_template, get_current_date, open_editor, sanitize_filename};

pub fn new(title: &str, project: Option<&str>, no_edit: bool, config: &Config) -> Result<()> {
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

    if !no_edit {
        open_editor(&file_path, &config.general.editor)?;
    }

    Ok(())
}

pub fn list(filters: &[String], config: &Config) -> Result<()> {
    let mut todos: Vec<TodoItem> = Vec::new();
    let root_dir = config.root_dir()?;
    let today = get_current_date(&config.general.date_format);

    // Parse filters
    let mut status_filter: Option<String> = None;
    let mut due_filter: Option<String> = None;

    for filter in filters {
        if let Some((key, value)) = filter.split_once(':') {
            match key {
                "status" => status_filter = Some(value.to_string()),
                "due" => due_filter = Some(value.to_string()),
                _ => {}
            }
        }
    }

    // Search in INBOX, NEXTACTION, and project directories
    let search_dirs = vec![
        config.inbox_dir()?,
        config.next_dir()?,
    ];

    for dir in search_dirs {
        if dir.exists() {
            collect_todos(&dir, &mut todos)?;
        }
    }

    // Search in project directories (recursive)
    let project_dir = config.project_dir()?;
    if project_dir.exists() {
        collect_todos_recursive(&project_dir, &mut todos)?;
    }

    // Apply filters
    if let Some(ref status) = status_filter {
        todos.retain(|t| t.status == *status);
    }

    if let Some(ref due) = due_filter {
        match due.as_str() {
            "today" => {
                todos.retain(|t| t.due == today);
            }
            "overdue" => {
                todos.retain(|t| !t.due.is_empty() && t.due < today);
            }
            _ => {
                // Treat as exact date match
                todos.retain(|t| t.due == *due);
            }
        }
    }

    if todos.is_empty() {
        println!("No active todos found.");
        return Ok(());
    }

    // Sort by created date (newest first)
    todos.sort_by(|a, b| b.created.cmp(&a.created));

    // Display todos
    for (i, todo) in todos.iter().enumerate() {
        let project_str = if todo.project.is_empty() {
            String::new()
        } else {
            format!(" [{}]", todo.project)
        };
        let due_str = if todo.due.is_empty() {
            String::new()
        } else {
            format!(" (due: {})", todo.due)
        };
        // Show path relative to root_dir
        let display_path = todo.path.strip_prefix(&root_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| todo.path.display().to_string());
        println!("{}: {} - {}{}{}", i + 1, todo.created, todo.title, project_str, due_str);
        println!("   {}", display_path);
    }

    println!("\nTotal: {} todo(s)", todos.len());

    // Prompt for selection
    print!("Open file (1-{}, or Enter to skip): ", todos.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if !input.is_empty() {
        if let Ok(selection) = input.parse::<usize>() {
            if selection >= 1 && selection <= todos.len() {
                open_editor(&todos[selection - 1].path, &config.general.editor)?;
            } else {
                println!("Invalid selection: {}", selection);
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct TodoItem {
    title: String,
    status: String,
    project: String,
    due: String,
    created: String,
    path: PathBuf,
}

fn parse_frontmatter(content: &str) -> Option<(String, String, String, String, String)> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0] != "---" {
        return None;
    }

    let mut end_index = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            end_index = Some(i);
            break;
        }
    }

    let end_index = end_index?;

    let mut kind = String::new();
    let mut status = String::new();
    let mut project = String::new();
    let mut due = String::new();
    let mut created = String::new();

    for line in &lines[1..end_index] {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "kind" => kind = value.to_string(),
                "status" => status = value.to_string(),
                "project" => project = value.to_string(),
                "due" | "due_date" => due = value.to_string(),
                "created" | "date" => created = value.to_string(),
                _ => {}
            }
        }
    }

    Some((kind, status, project, due, created))
}

fn extract_title(content: &str) -> String {
    for line in content.lines() {
        if line.starts_with("# ") {
            return line[2..].trim().to_string();
        }
    }
    String::new()
}

fn collect_todos(dir: &Path, todos: &mut Vec<TodoItem>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some((kind, status, project, due, created)) = parse_frontmatter(&content) {
                    // Filter: kind=todo and status is not done/canceled
                    if kind == "todo" && status != "done" && status != "canceled" {
                        let title = extract_title(&content);
                        todos.push(TodoItem {
                            title,
                            status,
                            project,
                            due,
                            created,
                            path,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn collect_todos_recursive(dir: &Path, todos: &mut Vec<TodoItem>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_todos_recursive(&path, todos)?;
        } else if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some((kind, status, project, due, created)) = parse_frontmatter(&content) {
                    if kind == "todo" && status != "done" && status != "canceled" {
                        let title = extract_title(&content);
                        todos.push(TodoItem {
                            title,
                            status,
                            project,
                            due,
                            created,
                            path,
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn done(file: &str, config: &Config) -> Result<()> {
    // If the file contains a path separator or exists as-is, use it directly
    // Otherwise, search in known directories
    let file_path = if file.contains('/') || Path::new(file).exists() {
        PathBuf::from(file)
    } else {
        find_todo_file(file, config)?
    };

    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file);
    }

    // Read the file content
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    // Update frontmatter
    let date = get_current_date(&config.general.date_format);
    let updated_content = update_frontmatter(&content, &date)?;

    // Write updated content back to file
    fs::write(&file_path, &updated_content)
        .with_context(|| format!("Failed to write file: {:?}", file_path))?;

    // Move to archive directory (99999_アーカイブ/99991_task)
    let archive_dir = config.archive_dir()?.join("99991_task");
    fs::create_dir_all(&archive_dir)
        .with_context(|| format!("Failed to create archive directory: {:?}", archive_dir))?;

    let file_name = file_path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
    let dest_path = archive_dir.join(file_name);

    fs::rename(&file_path, &dest_path)
        .with_context(|| format!("Failed to move file to archive: {:?}", dest_path))?;

    println!("Marked as done: {}", file_path.display());
    println!("Archived to: {}", dest_path.display());

    Ok(())
}

/// Check if the string is a date format (YYYY-MM-DD)
fn is_date_format(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 4 && parts[1].len() == 2 && parts[2].len() == 2
        && parts[0].chars().all(|c| c.is_ascii_digit())
        && parts[1].chars().all(|c| c.is_ascii_digit())
        && parts[2].chars().all(|c| c.is_ascii_digit())
}

/// Search for a todo file by filename or date in known directories
fn find_todo_file(query: &str, config: &Config) -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let is_date = is_date_format(query);

    let search_dirs = vec![
        config.inbox_dir()?,
        config.next_dir()?,
    ];

    for dir in search_dirs {
        if dir.exists() {
            search_in_dir(&dir, query, is_date, &mut candidates)?;
        }
    }

    // Search in project directories (recursive)
    let project_dir = config.project_dir()?;
    if project_dir.exists() {
        search_recursive(&project_dir, query, is_date, &mut candidates)?;
    }

    match candidates.len() {
        0 => anyhow::bail!("File not found: {}", query),
        1 => Ok(candidates.remove(0)),
        _ => {
            println!("Multiple files found:");
            for (i, path) in candidates.iter().enumerate() {
                println!("  {}: {}", i + 1, path.display());
            }

            // Prompt for selection
            print!("Select number (1-{}): ", candidates.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let selection: usize = input.trim().parse()
                .with_context(|| "Invalid number")?;

            if selection < 1 || selection > candidates.len() {
                anyhow::bail!("Invalid selection: {}", selection);
            }

            Ok(candidates.remove(selection - 1))
        }
    }
}

fn search_in_dir(dir: &Path, query: &str, is_date: bool, candidates: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if is_date {
                    // Match files starting with the date prefix
                    if name.starts_with(&format!("{}-", query)) {
                        candidates.push(path);
                    }
                } else if name == query {
                    candidates.push(path);
                }
            }
        }
    }
    Ok(())
}

fn search_recursive(dir: &Path, query: &str, is_date: bool, candidates: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            search_recursive(&path, query, is_date, candidates)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if is_date {
                if name.starts_with(&format!("{}-", query)) {
                    candidates.push(path);
                }
            } else if name == query {
                candidates.push(path);
            }
        }
    }

    Ok(())
}

fn update_frontmatter(content: &str, completed_date: &str) -> Result<String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find frontmatter boundaries
    if lines.is_empty() || lines[0] != "---" {
        anyhow::bail!("No frontmatter found");
    }

    let mut end_index = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            end_index = Some(i);
            break;
        }
    }

    let end_index = end_index.ok_or_else(|| anyhow::anyhow!("Invalid frontmatter"))?;

    // Update status and add completed date
    let mut new_lines: Vec<String> = Vec::new();
    let mut completed_added = false;

    for (i, line) in lines.iter().enumerate() {
        if i > 0 && i < end_index {
            if line.starts_with("status:") {
                new_lines.push("status: done".to_string());
            } else if line.starts_with("completed:") {
                new_lines.push(format!("completed: {}", completed_date));
                completed_added = true;
            } else {
                new_lines.push(line.to_string());
            }
        } else if i == end_index {
            // Add completed field before closing --- if not already present
            if !completed_added {
                new_lines.push(format!("completed: {}", completed_date));
            }
            new_lines.push(line.to_string());
        } else {
            new_lines.push(line.to_string());
        }
    }

    Ok(new_lines.join("\n"))
}
