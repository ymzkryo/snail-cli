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

# List all memos (not yet implemented)
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

# List all todos (not yet implemented)
snail todo list

# List todos with filters (not yet implemented)
snail todo list -f due:today
snail todo list -f status:next
snail todo list -f project:myproject

# Mark a todo as done (not yet implemented)
snail todo done path/to/todo.md
```

### Project Commands

```bash
# Create a new project
snail project new myproject
# Creates: 00800_プロジェクト/00831_myproject/YYYY-MM-DD-myproject-README.md
# (Project number is auto-incremented from existing projects)

# List all projects (not yet implemented)
snail project list

# Show project details (not yet implemented)
snail project show myproject
```

### GTD Commands

```bash
# List today's tasks (not yet implemented)
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

### Implemented (v0.1)
- ✅ `snail memo new`
- ✅ `snail todo new`
- ✅ `snail todo new -p <project>`
- ✅ `snail project new`
- ✅ `snail gtd today add`

### Planned (v0.2)
- ⏳ `snail memo list`
- ⏳ `snail memo search`
- ⏳ `snail todo list`
- ⏳ `snail todo list -f <filters>`
- ⏳ `snail todo done`
- ⏳ `snail project list`
- ⏳ `snail project show`
- ⏳ `snail gtd today list`
- ⏳ `snail gtd weekly`
- ⏳ `snail gtd monthly`

### Planned (v0.3)
- ⏳ `snail gtd process` (Interactive INBOX processing)
- ⏳ Project progress calculation
- ⏳ GitHub Actions integration

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
