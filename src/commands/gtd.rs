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
    widgets::{Block, Borders, Gauge, Paragraph},
    Terminal,
};

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

    let duration_mins: u64 = 10;
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

    let result = run_braindump_tui(
        &mut terminal,
        &file_path,
        &mut input,
        &mut item_count,
        start,
        duration_secs,
        week_str,
    );

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result?;

    println!("\nBraindump session complete!");
    println!("Items recorded: {}", item_count);
    println!("Saved to: {}", file_path.display());

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

pub fn monthly(_config: &Config) -> Result<()> {
    println!("GTD monthly review command - not yet implemented");
    Ok(())
}
