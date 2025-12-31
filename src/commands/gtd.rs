use anyhow::{Context, Result};
use chrono::{Datelike, Local};
use std::fs::{self, OpenOptions};
use std::io::{self, Write as IoWrite};
use std::time::{Duration, Instant};
use crate::config::Config;
use crate::utils::{get_current_date, open_editor};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};

// Review section structure
#[derive(Debug, Clone)]
struct ReviewSection {
    title: String,
    prompts: Vec<ReviewPrompt>,
}

#[derive(Debug, Clone)]
struct ReviewPrompt {
    text: String,
    indent: usize,
    responses: Vec<String>,
}

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
        Some(weekly_report_path) => {
            // Start weekly session (braindump + review)
            run_weekly_session(config, &week_str, &weekly_report_path)?;
        }
        None => {
            println!("No weekly report found for {} (Week {}).", iso_year, week_number);
            println!("\nPlease merge the weekly report PR first.");
        }
    }

    Ok(())
}

fn run_weekly_session(config: &Config, week_str: &str, weekly_report_path: &std::path::Path) -> Result<()> {
    let date = get_current_date(&config.general.date_format);
    let filename = format!("{}-{}-braindump.md", date, week_str);
    let inbox_dir = config.inbox_dir()?;
    let braindump_path = inbox_dir.join(&filename);

    // Create braindump file with header if it doesn't exist
    if !braindump_path.exists() {
        let header = format!("# {} Braindump\n\n", week_str);
        fs::write(&braindump_path, header)?;
    }

    // Parse weekly report for review sections
    let weekly_content = fs::read_to_string(weekly_report_path)?;
    let mut sections = parse_review_sections(&weekly_content);

    let duration_mins = config.gtd.braindump_duration_mins;
    let duration_secs = duration_mins * 60;
    let start = Instant::now();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let mut item_count: usize = 0;

    // Phase 1: Braindump
    let braindump_result = run_braindump_tui(
        &mut terminal,
        &braindump_path,
        &mut input,
        &mut item_count,
        start,
        duration_secs,
        week_str,
    );

    if braindump_result.is_err() {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        return braindump_result;
    }

    // Phase 2: Review
    input.clear();
    let review_result = run_review_tui(
        &mut terminal,
        &mut sections,
        &mut input,
        week_str,
    );

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    review_result?;

    // Update weekly report with review responses
    let updated_content = build_updated_weekly_report(&weekly_content, &sections);
    fs::write(weekly_report_path, updated_content)?;

    // Add Obsidian-style link to weekly report
    let braindump_link = format!("{}-{}-braindump", date, week_str);
    add_link_to_weekly_report(weekly_report_path, &braindump_link)?;

    println!("\nWeekly session complete!");
    println!("Braindump items: {}", item_count);
    println!("Braindump saved to: {}", braindump_path.display());
    println!("Weekly report updated: {}", weekly_report_path.display());

    Ok(())
}

fn parse_review_sections(content: &str) -> Vec<ReviewSection> {
    let mut sections = Vec::new();
    let mut current_section: Option<ReviewSection> = None;

    // Sections to skip
    let skip_patterns = ["dailyreport", "daily_report", "daily report"];

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous section
            if let Some(section) = current_section.take() {
                if !section.prompts.is_empty() {
                    sections.push(section);
                }
            }
            // Start new section (skip DailyReport sections)
            let title = line[3..].to_string();
            let title_lower = title.to_lowercase();
            let should_skip = skip_patterns.iter().any(|p| title_lower.contains(p));

            if !should_skip {
                current_section = Some(ReviewSection {
                    title,
                    prompts: Vec::new(),
                });
            } else {
                current_section = None;
            }
        } else if let Some(ref mut section) = current_section {
            // Check for prompt lines (lines ending with empty marker)
            let trimmed = line.trim_start_matches(' ');
            let indent = line.len() - trimmed.len();

            if trimmed.starts_with("- ") && trimmed.len() > 2 {
                let prompt_text = trimmed[2..].trim();
                // Only add if it looks like a prompt (not already filled)
                if !prompt_text.is_empty() {
                    section.prompts.push(ReviewPrompt {
                        text: prompt_text.to_string(),
                        indent,
                        responses: Vec::new(),
                    });
                }
            }
        }
    }

    // Don't forget the last section
    if let Some(section) = current_section {
        if !section.prompts.is_empty() {
            sections.push(section);
        }
    }

    sections
}

fn build_updated_weekly_report(original: &str, sections: &[ReviewSection]) -> String {
    let mut result = String::new();
    let mut current_section_idx: Option<usize> = None;
    let mut prompt_idx = 0;

    for line in original.lines() {
        if line.starts_with("## ") {
            let title = &line[3..];
            current_section_idx = sections.iter().position(|s| s.title == title);
            prompt_idx = 0;
            result.push_str(line);
            result.push('\n');
        } else if let Some(section_idx) = current_section_idx {
            let section = &sections[section_idx];
            let trimmed = line.trim_start_matches(' ');

            if trimmed.starts_with("- ") && prompt_idx < section.prompts.len() {
                let prompt = &section.prompts[prompt_idx];
                let indent = " ".repeat(prompt.indent);

                if !prompt.responses.is_empty() {
                    // Write prompt with responses
                    result.push_str(&format!("{}- {}\n", indent, prompt.text));
                    for response in &prompt.responses {
                        result.push_str(&format!("{}  - {}\n", indent, response));
                    }
                } else {
                    result.push_str(line);
                    result.push('\n');
                }
                prompt_idx += 1;
            } else {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn add_link_to_weekly_report(weekly_report_path: &std::path::Path, link_name: &str) -> Result<()> {
    // Check if link already exists
    let content = fs::read_to_string(weekly_report_path)?;
    let link = format!("[[{}]]", link_name);

    if content.contains(&link) {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(weekly_report_path)?;
    writeln!(file, "\n{}", link)?;
    Ok(())
}

fn run_braindump_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    file_path: &std::path::Path,
    input: &mut String,
    item_count: &mut usize,
    start: Instant,
    duration_secs: u64,
    week_str: &str,
) -> Result<()> {
    loop {
        let elapsed = start.elapsed().as_secs();
        if elapsed >= duration_secs {
            break;
        }

        let remaining = duration_secs - elapsed;
        let remaining_mins = remaining / 60;
        let remaining_secs = remaining % 60;
        let progress = (elapsed as f64 / duration_secs as f64 * 100.0) as u16;

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Length(3),  // Timer
                    Constraint::Length(3),  // Stats
                    Constraint::Min(3),     // Spacer
                    Constraint::Length(3),  // Input
                    Constraint::Length(2),  // Help
                ])
                .split(frame.area());

            // Title
            let title = Paragraph::new(format!(" {} Weekly Braindump", week_str))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(title, chunks[0]);

            // Timer gauge
            let gauge = Gauge::default()
                .block(Block::default().title(" Time Remaining ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Green))
                .percent(100 - progress)
                .label(format!("{:02}:{:02}", remaining_mins, remaining_secs));
            frame.render_widget(gauge, chunks[1]);

            // Stats
            let stats = Paragraph::new(format!(" Items recorded: {}", item_count))
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(stats, chunks[2]);

            // Input field
            let input_widget = Paragraph::new(format!(" > {}_", input))
                .style(Style::default().fg(Color::White))
                .block(Block::default()
                    .title(" What's on your mind? ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)));
            frame.render_widget(input_widget, chunks[4]);

            // Help text
            let help = Paragraph::new(" Enter: Save thought | Esc: End early")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(help, chunks[5]);
        })?;

        // Handle input with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            if !input.is_empty() {
                                // Save to file
                                let mut file = OpenOptions::new()
                                    .append(true)
                                    .open(file_path)?;
                                writeln!(file, "- {}", input)?;
                                *item_count += 1;
                                input.clear();
                            }
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

fn run_review_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sections: &mut [ReviewSection],
    input: &mut String,
    week_str: &str,
) -> Result<()> {
    if sections.is_empty() {
        return Ok(());
    }

    let mut section_idx = 0;
    let mut prompt_idx = 0;

    loop {
        let section = &sections[section_idx];
        let total_prompts: usize = sections.iter().map(|s| s.prompts.len()).sum();
        let current_prompt_num: usize = sections[..section_idx].iter().map(|s| s.prompts.len()).sum::<usize>() + prompt_idx + 1;

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Length(3),  // Progress
                    Constraint::Length(3),  // Section
                    Constraint::Min(5),     // Prompt list
                    Constraint::Length(3),  // Input
                    Constraint::Length(2),  // Help
                ])
                .split(frame.area());

            // Title
            let title = Paragraph::new(format!(" {} Weekly Review", week_str))
                .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(title, chunks[0]);

            // Progress
            let progress = Paragraph::new(format!(" Question {}/{}", current_prompt_num, total_prompts))
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(progress, chunks[1]);

            // Current section
            let section_widget = Paragraph::new(format!(" {}", section.title))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().title(" Section ").borders(Borders::ALL));
            frame.render_widget(section_widget, chunks[2]);

            // Prompt list with current highlighted
            let items: Vec<ListItem> = section.prompts.iter().enumerate().map(|(i, p)| {
                let style = if i == prompt_idx {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else if !p.responses.is_empty() {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let marker = if i == prompt_idx {
                    "▶ "
                } else if !p.responses.is_empty() {
                    "✓ "
                } else {
                    "  "
                };

                let count = if !p.responses.is_empty() {
                    format!(" ({})", p.responses.len())
                } else {
                    String::new()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(&p.text, style),
                    Span::styled(count, Style::default().fg(Color::DarkGray)),
                ]))
            }).collect();

            let list = List::new(items)
                .block(Block::default().title(" Prompts ").borders(Borders::ALL));
            frame.render_widget(list, chunks[3]);

            // Input field
            let current_prompt = &section.prompts[prompt_idx];
            let input_widget = Paragraph::new(format!(" > {}_", input))
                .style(Style::default().fg(Color::White))
                .block(Block::default()
                    .title(format!(" {} ", current_prompt.text))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)));
            frame.render_widget(input_widget, chunks[4]);

            // Help text
            let help = Paragraph::new(" Enter: Save & Next | Tab: Skip | Esc: Finish")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(help, chunks[5]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            // Add response to current prompt (stay on same prompt)
                            if !input.is_empty() {
                                sections[section_idx].prompts[prompt_idx].responses.push(input.clone());
                                input.clear();
                            }
                        }
                        KeyCode::Tab => {
                            // Move to next prompt
                            input.clear();
                            if prompt_idx + 1 < sections[section_idx].prompts.len() {
                                prompt_idx += 1;
                            } else if section_idx + 1 < sections.len() {
                                section_idx += 1;
                                prompt_idx = 0;
                            } else {
                                // All done
                                break;
                            }
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Esc => {
                            // Save current input if any
                            if !input.is_empty() {
                                sections[section_idx].prompts[prompt_idx].responses.push(input.clone());
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn monthly(_config: &Config) -> Result<()> {
    println!("GTD monthly review command - not yet implemented");
    Ok(())
}
