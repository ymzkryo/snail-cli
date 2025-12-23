use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "snail")]
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
        /// Do not open editor after creating
        #[arg(short = 'n', long)]
        no_edit: bool,
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
        /// Do not open editor after creating
        #[arg(short = 'n', long)]
        no_edit: bool,
    },
    /// List all todo tasks
    List {
        /// Filter option (e.g., "due:today", "status:next", "project:hoge")
        #[arg(short, long)]
        filter: Vec<String>,
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
