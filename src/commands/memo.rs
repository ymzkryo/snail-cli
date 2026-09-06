use anyhow::Result;
use std::io::{self, Write};
use crate::config::Config;
use crate::note::{self, Note};
use crate::tags::Vocabulary;
use crate::utils::{create_note, filename_component, get_current_date, open_editor, yaml_quote_value};

pub fn new(
    title: &str,
    requested_tags: &[String],
    no_edit: bool,
    strict: bool,
    config: &Config,
) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let name = filename_component(title, strict)?;
    let filename = format!("{}-{}.md", date, name);

    let inbox_dir = config.inbox_dir()?;
    let file_path = inbox_dir.join(&filename);

    let note_tags = crate::tags::resolve(&[], requested_tags);
    if let Some(vocabulary) = Vocabulary::load(&config.tag_vocabulary_path()?)? {
        vocabulary.validate(&note_tags)?;
    }
    if !note_tags.iter().any(|tag| tag.starts_with("type/")) {
        eprintln!("note: no type/ tag set; pass --tag type/<...> to satisfy the vault check");
    }

    let title_yaml = yaml_quote_value(title);
    let project_yaml = yaml_quote_value("");
    let replacements = vec![
        ("title", title),
        ("title_yaml", title_yaml.as_str()),
        ("date", &date),
        ("status", ""),
        ("project", ""),
        ("project_yaml", project_yaml.as_str()),
    ];

    let base_path = config.get_template_path("base").ok().filter(|p| p.exists());
    let snip_path = config.get_template_path("memo")?;

    create_note(base_path.as_deref(), &snip_path, &file_path, &replacements, &note_tags)?;

    println!("Created memo: {}", file_path.display());

    if !no_edit {
        open_editor(&file_path, &config.general.editor)?;
    }

    Ok(())
}

pub fn list(config: &Config) -> Result<()> {
    let root_dir = config.root_dir()?;

    let mut memos: Vec<Note> = Vec::new();
    note::collect(&config.inbox_dir()?, false, &mut memos)?;
    note::collect(&config.next_dir()?, false, &mut memos)?;
    note::collect(&config.project_dir()?, true, &mut memos)?;

    // A memo is a note with no task status.
    memos.retain(|memo| memo.is_memo());

    if memos.is_empty() {
        println!("No memos found.");
        return Ok(());
    }

    // Sort by created date (newest first)
    memos.sort_by(|a, b| b.created.cmp(&a.created));

    // Display memos
    for (i, memo) in memos.iter().enumerate() {
        let display_path = memo.path.strip_prefix(&root_dir)
            .unwrap_or(&memo.path)
            .display()
            .to_string();
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

pub fn search(_keyword: &str, _config: &Config) -> Result<()> {
    println!("Memo search command - not yet implemented");
    Ok(())
}
