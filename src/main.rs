mod cli;
mod commands;
mod config;
mod filter;
mod note;
mod tags;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, MemoAction, TodoAction, ProjectAction, GtdAction, TodayAction};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Commands::Memo { action } => match action {
            MemoAction::New { title, tags, no_edit, strict } => {
                commands::memo::new(&title, &tags, no_edit, strict, &config)?;
            }
            MemoAction::List => {
                commands::memo::list(&config)?;
            }
            MemoAction::Search { keyword } => {
                commands::memo::search(&keyword, &config)?;
            }
        },
        Commands::Todo { action } => match action {
            TodoAction::New { title, project, tags, no_edit, strict } => {
                commands::todo::new(&title, project.as_deref(), &tags, no_edit, strict, &config)?;
            }
            TodoAction::List { filter, sort, format } => {
                commands::todo::list(&filter, sort, format, &config)?;
            }
            TodoAction::Done { file } => {
                commands::todo::done(&file, &config)?;
            }
        },
        Commands::Project { action } => match action {
            ProjectAction::New { name, no_edit, strict } => {
                commands::project::new(&name, no_edit, strict, &config)?;
            }
            ProjectAction::List => {
                commands::project::list(&config)?;
            }
            ProjectAction::Show { name } => {
                commands::project::show(&name, &config)?;
            }
        },
        Commands::Gtd { action } => match action {
            GtdAction::Today { action } => match action {
                TodayAction::List => {
                    commands::gtd::today_list(&config)?;
                }
                TodayAction::Add { task } => {
                    commands::gtd::today_add(&task, &config)?;
                }
            },
            GtdAction::Weekly => {
                commands::gtd::weekly(&config)?;
            }
            GtdAction::Monthly => {
                commands::gtd::monthly(&config)?;
            }
        },
    }

    Ok(())
}
