use clap::{Parser, Subcommand, ValueEnum};

use crate::filter::SortKey;

#[derive(Parser)]
#[command(name = "snail", version)]
#[command(about = "A CLI tool for managing notes, tasks, and GTD workflow", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage general memos
    Memo {
        #[command(subcommand)]
        action: MemoAction,
    },
    /// Manage todo tasks
    Todo {
        #[command(subcommand)]
        action: TodoAction,
    },
    /// Manage projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// GTD review and daily management
    Gtd {
        #[command(subcommand)]
        action: GtdAction,
    },
}

#[derive(Subcommand)]
pub enum MemoAction {
    /// Create a new memo
    New {
        /// Title of the memo
        title: String,
        /// Frontmatter tag, repeatable (e.g. "type/tech", "topic/rust")
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        /// Do not open editor after creating
        #[arg(short = 'n', long)]
        no_edit: bool,
        /// Reject titles that are not already filename-safe instead of sanitizing
        #[arg(long)]
        strict: bool,
    },
    /// List all memos
    List,
    /// Search memos
    Search {
        /// Search keyword
        keyword: String,
    },
}

#[derive(Subcommand)]
pub enum TodoAction {
    /// Create a new todo task
    New {
        /// Title of the task
        title: String,
        /// Project name
        #[arg(short, long)]
        project: Option<String>,
        /// Extra frontmatter tag, repeatable (defaults to "type/todo")
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        /// Do not open editor after creating
        #[arg(short = 'n', long)]
        no_edit: bool,
        /// Reject titles that are not already filename-safe instead of sanitizing
        #[arg(long)]
        strict: bool,
    },
    /// List all todo tasks
    List {
        /// Filter terms, comma-separated and repeatable; all terms must match.
        ///
        /// Keys: status, project, context, due, review.
        /// Date values: today, overdue, reached, missing, or YYYY-MM-DD.
        /// Text values: the frontmatter value, or missing.
        /// Unknown keys and values are errors, not empty results.
        ///
        /// e.g. "status:next,context:@computer", "status:scheduled,due:reached"
        #[arg(short, long, verbatim_doc_comment)]
        filter: Vec<String>,
        /// Field to order by
        #[arg(short, long, value_enum, default_value_t = SortKey::Created)]
        sort: SortKey,
        /// Output format; json skips the interactive prompt
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Mark a todo as done
    Done {
        /// Path to the todo file
        file: String,
    },
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// Create a new project
    New {
        /// Project name
        name: String,
        /// Do not open editor after creating
        #[arg(short = 'n', long)]
        no_edit: bool,
        /// Reject titles that are not already filename-safe instead of sanitizing
        #[arg(long)]
        strict: bool,
    },
    /// List all projects
    List,
    /// Show project details
    Show {
        /// Project name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum GtdAction {
    /// Today's task management
    Today {
        #[command(subcommand)]
        action: TodayAction,
    },
    /// Weekly review
    Weekly,
    /// Monthly review
    Monthly,
}

#[derive(Subcommand)]
pub enum TodayAction {
    /// List today's tasks
    List,
    /// Add a task to today's daily report
    Add {
        /// Task description
        task: String,
    },
}

/// How `snail todo list` prints its results.
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable listing with an interactive prompt to open a file.
    Text,
    /// One JSON array of note objects, for scripts and skills.
    Json,
}
