use anyhow::Result;
use std::fs;
use std::path::Path;
use crate::config::Config;

pub fn frontmatter(dry_run: bool, config: &Config) -> Result<()> {
    let root_dir = config.root_dir()?;

    if !root_dir.exists() {
        anyhow::bail!("Root directory not found: {:?}", root_dir);
    }

    let mut updated = 0;
    let mut skipped = 0;

    process_directory(&root_dir, dry_run, &mut updated, &mut skipped)?;

    if dry_run {
        println!("\n[dry-run] Would update {} file(s), skipped {} file(s)", updated, skipped);
    } else {
        println!("\nUpdated {} file(s), skipped {} file(s)", updated, skipped);
    }

    Ok(())
}

fn process_directory(dir: &Path, dry_run: bool, updated: &mut usize, skipped: &mut usize) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            process_directory(&path, dry_run, updated, skipped)?;
        } else if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            match process_file(&path, dry_run) {
                Ok(true) => *updated += 1,
                Ok(false) => *skipped += 1,
                Err(e) => {
                    eprintln!("Warning: failed to process {:?}: {}", path, e);
                    *skipped += 1;
                }
            }
        }
    }
    Ok(())
}

fn process_file(path: &Path, dry_run: bool) -> Result<bool> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    // Check for front matter
    if lines.is_empty() || lines[0] != "---" {
        return Ok(false);
    }

    let mut end_index = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            end_index = Some(i);
            break;
        }
    }

    let end_index = match end_index {
        Some(i) => i,
        None => return Ok(false),
    };

    // Check if kind already exists
    let mut has_kind = false;
    let mut has_status = false;

    for line in &lines[1..end_index] {
        if let Some((key, _)) = line.split_once(':') {
            match key.trim() {
                "kind" => has_kind = true,
                "status" => has_status = true,
                _ => {}
            }
        }
    }

    if has_kind {
        return Ok(false);
    }

    // Determine kind based on status field presence
    let kind = if has_status { "todo" } else { "memo" };

    if dry_run {
        println!("[dry-run] {} -> kind: {}", path.display(), kind);
        return Ok(true);
    }

    // Insert kind field after opening ---
    let mut new_lines: Vec<String> = Vec::with_capacity(lines.len() + 1);
    new_lines.push(lines[0].to_string()); // ---
    new_lines.push(format!("kind: {}", kind));
    for line in &lines[1..] {
        new_lines.push(line.to_string());
    }

    fs::write(path, new_lines.join("\n"))?;
    println!("Updated: {} -> kind: {}", path.display(), kind);

    Ok(true)
}
