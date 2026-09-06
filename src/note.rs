use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// A note's YAML frontmatter, kept as raw key / value pairs in source order.
///
/// This is deliberately not a full YAML parser: notes in the vault use
/// single-line scalars plus flow sequences (`tags: [a, b]`), which is all the
/// CLI needs to read.
#[derive(Debug, Default)]
pub struct Frontmatter {
    fields: Vec<(String, String)>,
}

impl Frontmatter {
    /// Parse the frontmatter block. Returns `None` when the file does not open
    /// with a `---` fence or the fence is never closed.
    pub fn parse(content: &str) -> Option<Self> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.first() != Some(&"---") {
            return None;
        }

        let end_index = lines.iter().skip(1).position(|l| *l == "---")? + 1;

        let mut fields: Vec<(String, String)> = Vec::new();
        for line in &lines[1..end_index] {
            // Continuation lines of block sequences ("  - value") carry no key.
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                if key.is_empty() || key.starts_with('-') || key.starts_with('#') {
                    continue;
                }
                if fields.iter().any(|(k, _)| k == key) {
                    continue;
                }
                fields.push((key.to_string(), value.trim().to_string()));
            }
        }

        Some(Self { fields })
    }

    /// The unquoted value for `key`, or `""` when the key is absent or empty.
    pub fn value(&self, key: &str) -> String {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| unquote(v))
            .unwrap_or_default()
    }

    /// The first key that is present with a non-empty value, unquoted.
    pub fn value_of_any(&self, keys: &[&str]) -> String {
        for key in keys {
            let value = self.value(key);
            if !value.is_empty() {
                return value;
            }
        }
        String::new()
    }

    /// A flow sequence value (`tags: [type/todo, topic/rust]`) as items.
    pub fn list(&self, key: &str) -> Vec<String> {
        let raw = self.value(key);
        let inner = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')).unwrap_or(&raw);

        inner
            .split(',')
            .map(|item| unquote(item.trim()))
            .filter(|item| !item.is_empty())
            .collect()
    }
}

/// Strip one layer of matching quotes and surrounding whitespace.
fn unquote(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

/// A note on disk, with the frontmatter fields the CLI filters and reports on.
#[derive(Debug)]
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub status: String,
    pub project: String,
    pub context: String,
    pub estimate: String,
    pub due_date: String,
    pub review_date: String,
    pub created: String,
    pub tags: Vec<String>,
}

impl Note {
    pub fn from_content(path: PathBuf, content: &str) -> Option<Self> {
        let fm = Frontmatter::parse(content)?;

        Some(Self {
            title: note_title(&fm, content),
            status: fm.value("status"),
            project: fm.value("project"),
            context: fm.value("context"),
            estimate: fm.value("estimate"),
            due_date: fm.value_of_any(&["due_date", "due"]),
            review_date: fm.value("review_date"),
            created: fm.value_of_any(&["date", "created"]),
            tags: fm.list("tags"),
            path,
        })
    }

    /// True for notes that represent an open task.
    pub fn is_active_todo(&self) -> bool {
        !self.status.is_empty() && self.status != "done" && self.status != "canceled"
    }

    /// True for notes with no task status, i.e. plain memos.
    pub fn is_memo(&self) -> bool {
        self.status.is_empty()
    }
}

/// Prefer the first `# ` heading, falling back to the frontmatter title so a
/// note without a heading still shows something useful.
fn note_title(fm: &Frontmatter, content: &str) -> String {
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            return heading.trim().to_string();
        }
    }
    fm.value("title")
}

/// Collect notes from `dir`, descending into subdirectories when `recursive`.
pub fn collect(dir: &Path, recursive: bool, notes: &mut Vec<Note>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        if path.is_dir() {
            if recursive {
                collect(&path, recursive, notes)?;
            }
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(note) = Note::from_content(path, &content) {
                    notes.push(note);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\n\
title: \"Phase 4: publish the blog\"\n\
date: 2026-05-03\n\
status: next\n\
review_date: 2026-09-29\n\
due_date: 2026-09-30\n\
estimate: 2h\n\
project: カタツムリワークス\n\
tags: [type/todo, topic/rust]\n\
context: \"@computer\"\n\
---\n\
\n\
# Phase 4: publish the blog\n";

    #[test]
    fn parses_every_frontmatter_field() {
        let note = Note::from_content(PathBuf::from("a.md"), SAMPLE).unwrap();
        assert_eq!(note.status, "next");
        assert_eq!(note.project, "カタツムリワークス");
        assert_eq!(note.context, "@computer");
        assert_eq!(note.estimate, "2h");
        assert_eq!(note.due_date, "2026-09-30");
        assert_eq!(note.review_date, "2026-09-29");
        assert_eq!(note.created, "2026-05-03");
        assert_eq!(note.tags, vec!["type/todo", "topic/rust"]);
        assert_eq!(note.title, "Phase 4: publish the blog");
    }

    #[test]
    fn colons_inside_a_quoted_value_are_kept() {
        let fm = Frontmatter::parse(SAMPLE).unwrap();
        assert_eq!(fm.value("title"), "Phase 4: publish the blog");
    }

    #[test]
    fn missing_and_empty_fields_read_as_empty() {
        let content = "---\nstatus: inbox\ncontext:\n---\n\n# t\n";
        let note = Note::from_content(PathBuf::from("a.md"), content).unwrap();
        assert_eq!(note.context, "");
        assert_eq!(note.due_date, "");
        assert!(note.tags.is_empty());
    }

    #[test]
    fn empty_tag_list_yields_no_tags() {
        let fm = Frontmatter::parse("---\ntags: []\n---\n").unwrap();
        assert!(fm.list("tags").is_empty());
    }

    #[test]
    fn due_falls_back_to_the_legacy_key() {
        let content = "---\nstatus: next\ndue: 2026-01-01\n---\n\n# t\n";
        let note = Note::from_content(PathBuf::from("a.md"), content).unwrap();
        assert_eq!(note.due_date, "2026-01-01");
    }

    #[test]
    fn files_without_frontmatter_are_skipped() {
        assert!(Note::from_content(PathBuf::from("a.md"), "# just a heading\n").is_none());
        assert!(Note::from_content(PathBuf::from("a.md"), "---\nstatus: next\n").is_none());
    }

    #[test]
    fn title_falls_back_to_frontmatter_when_no_heading() {
        let content = "---\ntitle: from frontmatter\nstatus: next\n---\n\nbody\n";
        let note = Note::from_content(PathBuf::from("a.md"), content).unwrap();
        assert_eq!(note.title, "from frontmatter");
    }

    #[test]
    fn active_todo_excludes_finished_and_statusless_notes() {
        let of = |status: &str| {
            Note::from_content(PathBuf::from("a.md"), &format!("---\nstatus: {}\n---\n", status)).unwrap()
        };
        assert!(of("next").is_active_todo());
        assert!(of("scheduled").is_active_todo());
        assert!(!of("done").is_active_todo());
        assert!(!of("canceled").is_active_todo());
        assert!(!of("").is_active_todo());
        assert!(of("").is_memo());
    }
}
