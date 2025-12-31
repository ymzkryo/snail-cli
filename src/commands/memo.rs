use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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

pub fn list(config: &Config) -> Result<()> {
    let mut memos: Vec<MemoItem> = Vec::new();
    let root_dir = config.root_dir()?;

    // Search in INBOX, NEXTACTION, and project directories
    let search_dirs = vec![
        config.inbox_dir()?,
        config.next_dir()?,
    ];

    for dir in search_dirs {
        if dir.exists() {
            collect_memos(&dir, &mut memos)?;
        }
    }

    // Search in project directories (recursive)
    let project_dir = config.project_dir()?;
    if project_dir.exists() {
        collect_memos_recursive(&project_dir, &mut memos)?;
    }

    if memos.is_empty() {
        println!("No memos found.");
        return Ok(());
    }

    // Sort by created date (newest first)
    memos.sort_by(|a, b| b.created.cmp(&a.created));

    // Display memos
    for (i, memo) in memos.iter().enumerate() {
        let display_path = memo.path.strip_prefix(&root_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| memo.path.display().to_string());
        println!("{}: {} - {}", i + 1, memo.created, memo.title);
        println!("   {}", display_path);
    }

    println!("\nTotal: {} memo(s)", memos.len());

    // Prompt for selection
    print!("Open file (1-{}, or Enter to skip): ", memos.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if !input.is_empty() {
        if let Ok(selection) = input.parse::<usize>() {
            if selection >= 1 && selection <= memos.len() {
                open_editor(&memos[selection - 1].path, &config.general.editor)?;
            } else {
                println!("Invalid selection: {}", selection);
            }
        }
    }

    Ok(())
}

struct MemoItem {
    title: String,
    created: String,
    path: PathBuf,
}

fn parse_frontmatter(content: &str) -> Option<(String, String)> {
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
    let mut created = String::new();

    for line in &lines[1..end_index] {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "kind" => kind = value.to_string(),
                "created" | "date" => created = value.to_string(),
                _ => {}
            }
        }
    }

    Some((kind, created))
}

fn extract_title(content: &str) -> String {
    for line in content.lines() {
        if line.starts_with("# ") {
            return line[2..].trim().to_string();
        }
    }
    String::new()
}

fn collect_memos(dir: &Path, memos: &mut Vec<MemoItem>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some((kind, created)) = parse_frontmatter(&content) {
                    // Filter: kind=memo or kind is empty
                    if kind == "memo" || kind.is_empty() {
                        let title = extract_title(&content);
                        memos.push(MemoItem {
                            title,
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

fn collect_memos_recursive(dir: &Path, memos: &mut Vec<MemoItem>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_memos_recursive(&path, memos)?;
        } else if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some((kind, created)) = parse_frontmatter(&content) {
                    if kind == "memo" || kind.is_empty() {
                        let title = extract_title(&content);
                        memos.push(MemoItem {
                            title,
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

pub fn search(_keyword: &str, _config: &Config) -> Result<()> {
    println!("Memo search command - not yet implemented");
    Ok(())
}
