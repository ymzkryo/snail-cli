# snail-cli

A CLI tool for managing notes, tasks, and GTD workflow.

## Overview

snail-cli is a Rust-based command-line tool that helps you manage:
- **Memos**: General notes and ideas
- **Todos**: Task management with GTD principles
- **Projects**: Project organization with automatic numbering
- **GTD Reviews**: Daily, weekly, and monthly reviews

## Installation

```bash
# Clone the repository
cd ~/PROJECTS/snail/snail-cli

# Build the project
cargo build --release

# The binary will be at target/release/snail
# Optionally, add it to your PATH or create a symlink
```

## Configuration

snail-cli uses a configuration file at `~/.config/snail-cli/config.toml`.

If no config file exists, it will use default settings:
- Root directory: `~/memo`
- Editor: `vim`
- Date format: `%Y-%m-%d`
- Templates: From `snail-cli/templates/` directory

### Example config.toml

```toml
[general]
root_dir = "~/memo"
editor = "vim"
date_format = "%Y-%m-%d"

[templates]
memo = "~/custom-templates/memo.md"
todo = "~/custom-templates/todo.md"
project = "~/custom-templates/project.md"
daily_report = "~/custom-templates/daily_report.md"

[directories]
inbox = "00000_INBOX"
next = "00100_NEXTACTION"
someday = "00500_いつかやる"
project = "00800_プロジェクト"
archive = "99999_アーカイブ"
```

## Usage

### Memo Commands

```bash
# Create a new memo
snail memo new "Meeting notes"

# Create without opening editor
snail memo new "Meeting notes" -n

# List all memos (interactive selection to open in editor)
snail memo list

# Search memos (not yet implemented)
snail memo search "keyword"
```

### Todo Commands

```bash
# Create a new todo task
snail todo new "Implement feature X"

# Create a todo with project assignment
snail todo new "Fix bug" -p myproject

# Create without opening editor
snail todo new "Task" -n

# List all active todos (interactive selection to open in editor)
snail todo list

# List todos with filters
snail todo list -f status:next
snail todo list -f status:inbox
snail todo list -f due:today
snail todo list -f due:overdue
snail todo list -f due:2025-01-15

# Combine multiple filters
snail todo list -f status:next -f due:today

# Mark a todo as done (updates status, adds completed date, moves to archive)
snail todo done 2025-12-31                    # by date
snail todo done 2025-12-31-task-name.md       # by filename
snail todo done path/to/todo.md               # by path
```

### Project Commands

```bash
# Create a new project
snail project new myproject
# Creates: 00800_プロジェクト/00831_myproject/YYYY-MM-DD-myproject-README.md
# (Project number is auto-incremented from existing projects)

# Create without opening editor
snail project new myproject -n

# List all projects (interactive selection to open README)
snail project list

# Show project details (not yet implemented)
snail project show myproject
```

### GTD Commands

```bash
# List today's tasks from daily report
snail gtd today list

# Add a task to today's daily report
snail gtd today add "Review pull requests"

# Weekly review (not yet implemented)
snail gtd weekly

# Monthly review (not yet implemented)
snail gtd monthly
```

## File Structure

All notes are initially saved to `00000_INBOX/`:

```
~/memo/
├── 00000_INBOX/
│   ├── 2025-11-28-meeting-notes.md      # Created with: snail memo new
│   ├── 2025-11-28-implement-feature.md  # Created with: snail todo new
│   ├── 2025-11-28-daily_report.md       # Created with: snail gtd today add
│   └── 2025-W48-weekly_report.md
├── 00100_NEXTACTION/                     # (Future: moved by snail gtd process)
├── 00500_いつかやる/
├── 00800_プロジェクト/
│   ├── 00831_myproject/
│   │   └── 2025-11-28-myproject-README.md
│   └── 00832_anotherproject/
└── 99999_アーカイブ/
```

## Templates

Default templates are located in `templates/` directory:

- `memo.md`: Template for general memos
- `todo.md`: Template for todo tasks
- `project.md`: Template for project README files
- `daily_report.md`: Template for daily reports

### Template Variables

Templates support the following variables:

- `{{date}}`: Current date (formatted according to config)
- `{{title}}`: Title/name provided in command
- `{{name}}`: Project name (for project template)
- `{{project}}`: Project name (for todo template)

## Development Status

### Implemented
- ✅ `snail memo new` (`-n` to skip editor)
- ✅ `snail memo list`
- ✅ `snail todo new` (`-p` for project, `-n` to skip editor)
- ✅ `snail todo list` (`-f status:*`, `-f due:*`)
- ✅ `snail todo done`
- ✅ `snail project new` (`-n` to skip editor)
- ✅ `snail project list`
- ✅ `snail gtd today list`
- ✅ `snail gtd today add`

### Planned
- ⏳ `snail memo search`
- ⏳ `snail project show`
- ⏳ `snail gtd weekly`
- ⏳ `snail gtd monthly`
- ⏳ `snail gtd process` (Interactive INBOX processing)

## Architecture

```
snail-cli/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli.rs               # CLI command definitions (clap)
│   ├── config.rs            # Configuration management
│   ├── utils.rs             # Utility functions
│   └── commands/
│       ├── mod.rs
│       ├── memo.rs          # Memo commands
│       ├── todo.rs          # Todo commands
│       ├── project.rs       # Project commands
│       └── gtd.rs           # GTD commands
├── templates/               # Default templates
│   ├── memo.md
│   ├── todo.md
│   ├── project.md
│   └── daily_report.md
└── Cargo.toml
```

## Contributing

This is a personal productivity tool, but suggestions and improvements are welcome!

## License

MIT
