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
base = "~/custom-templates/base.md"
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

[tags]
# Where the tag vocabulary lives. Defaults to <root_dir>/.github/scripts/config.toml
vocabulary = "~/memo/.github/scripts/config.toml"
# Tags applied by `snail todo new` unless --tag supplies its own type/
todo_default = ["type/todo"]
```

### Tags

New notes get a frontmatter `tags:` list. `snail todo new` applies
`tags.todo_default` (`type/todo`); `snail memo new` starts empty and takes
`--tag`. An explicit `--tag type/...` replaces the default, so a note never
ends up with two `type/` tags.

Tags are checked against the `[tags]` section of the vocabulary file — the
vault's own config — and an unknown `type/` or `topic/`, or more than
`max_topics` topics, is rejected before the file is written. When the file does
not exist, validation is skipped.

```bash
snail memo new "Design notes" -t type/tech -t topic/rust
snail todo new "Interview prep" -t type/meeting     # replaces type/todo
```

## Usage

### Filenames

Titles are converted into UNIX-safe filenames before a file is created. Only
ASCII alphanumerics, `-`, `_`, and Japanese characters (kana / kanji) are kept;
anything else — full-width punctuation, spaces, and ASCII that needs shell
escaping — becomes `-`, with runs collapsed and edges trimmed. Full-width ASCII
is folded to half-width first, so `ＡＰＩ２０２６` stays `API2026`.

```bash
$ snail memo new "面談メモ（AI活用プロダクト開発案件）" -n
note: filename sanitized: "面談メモ（AI活用プロダクト開発案件）" -> "面談メモ-AI活用プロダクト開発案件"
Created memo: .../2026-08-19-面談メモ-AI活用プロダクト開発案件.md
```

The frontmatter `title:` keeps the original text — only the filename is
sanitized. Pass `--strict` to `memo new` / `todo new` / `project new` to fail
instead of rewriting, and fix the title by hand.

### Memo Commands

```bash
# Create a new memo
snail memo new "Meeting notes"

# Create without opening editor
snail memo new "Meeting notes" -n

# Reject an unsafe title instead of sanitizing it
snail memo new "面談メモ（案件）" --strict

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

# Reject an unsafe title instead of sanitizing it
snail todo new "出張メモ（8/27-29）" --strict

# List all active todos (interactive selection to open in editor).
# Scans 00000_INBOX, 00100_NEXTACTION, 00500_いつかやる (recursive) and 00800_プロジェクト (recursive).
snail todo list

# List todos with filters
snail todo list -f status:next
snail todo list -f project:myproject
snail todo list -f context:@computer      # the @ is optional
snail todo list -f due:today
snail todo list -f due:overdue
snail todo list -f due:2026-09-30
snail todo list -f review:overdue
snail todo list -f review:missing

# Combine filters: comma-separated, or repeat -f. All terms must match (AND).
snail todo list -f "status:next,context:@computer"
snail todo list -f status:next -f due:today

# Scheduled tasks whose start date has arrived
snail todo list -f "status:scheduled,due:reached"

# Order by due date instead of creation date
snail todo list -f status:next --sort due

# Machine-readable output (skips the interactive prompt)
snail todo list -f "status:next,context:@computer" --format json

# Create a todo with extra tags
snail todo new "Task" -t topic/rust

# Mark a todo as done (updates status, adds completed date, moves to archive)
snail todo done 2025-12-31                    # by date
snail todo done 2025-12-31-task-name.md       # by filename
snail todo done path/to/todo.md               # by path
```

#### Filter reference

A filter is one or more `key:value` terms. Terms may be comma-separated inside
one `-f`, or spread over several `-f` flags; every term must match.

| Key | Matches against | Values |
| --- | --- | --- |
| `status` | frontmatter `status` | any value, case-insensitive; `missing` |
| `project` | frontmatter `project` | any value, case-insensitive; `missing` |
| `context` | frontmatter `context` | e.g. `@computer` / `computer`; `missing` |
| `due` | frontmatter `due_date` | `today`, `overdue`, `reached`, `missing`, `YYYY-MM-DD` |
| `review` | frontmatter `review_date` | same as `due` |

Date values: `overdue` is strictly before today, `reached` is today or earlier
(the date has arrived), and `missing` is an unset field.

An unknown key or an unsupported value is an error and exits non-zero, rather
than quietly returning zero results:

```bash
$ snail todo list -f due:week
Error: unsupported value for `due:`: "week"
  expected a YYYY-MM-DD date or one of: today, overdue, reached, missing
```

### Project Commands

```bash
# Create a new project
snail project new myproject
# Creates: 00800_プロジェクト/00831_myproject/YYYY-MM-DD-myproject-README.md
# (Project number is auto-incremented from existing projects)

# Create without opening editor
snail project new myproject -n

# Reject an unsafe name instead of sanitizing it
snail project new "新規案件（PoC）" --strict

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
- `{{title_yaml}}`: Title, quoted for YAML frontmatter
- `{{name}}`: Project name (for project template)
- `{{project}}`: Project name (for todo template)
- `{{project_yaml}}`: Project name, quoted for YAML frontmatter
- `{{status}}`: Initial status (`inbox` for todos, empty for memos)
- `{{body}}`: The rendered snippet template (base template only)

Frontmatter lines are trimmed of trailing whitespace after substitution, so a
placeholder that expands to nothing leaves `status:` rather than `status: `.
The note body is left untouched, so Markdown two-space line breaks survive.

The `tags:` line is set from `--tag` / `tags.todo_default`; if the template has
no `tags:` entry, one is added.

## Development Status

### Implemented
- ✅ `snail memo new` (`-t` for tags, `-n` to skip editor, `--strict` to reject unsafe titles)
- ✅ `snail memo list`
- ✅ `snail todo new` (`-p` for project, `-t` for tags, `-n` to skip editor, `--strict` to reject unsafe titles)
- ✅ `snail todo list` (`-f status|project|context|due|review`, `--sort`, `--format json`)
- ✅ `snail todo done`
- ✅ `snail project new` (`-n` to skip editor, `--strict` to reject unsafe names)
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
│   ├── filter.rs            # `todo list` filter parsing / matching / sorting
│   ├── note.rs              # Frontmatter parsing and note collection
│   ├── tags.rs              # Tag vocabulary and validation
│   ├── utils.rs             # Utility functions
│   └── commands/
│       ├── mod.rs
│       ├── memo.rs          # Memo commands
│       ├── todo.rs          # Todo commands
│       ├── project.rs       # Project commands
│       └── gtd.rs           # GTD commands
├── templates/               # Default templates
│   ├── base.md
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
