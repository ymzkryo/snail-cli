use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub templates: TemplateConfig,
    pub directories: DirectoryConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneralConfig {
    pub root_dir: String,
    pub editor: String,
    pub date_format: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TemplateConfig {
    pub memo: String,
    pub todo: String,
    pub project: String,
    pub daily_report: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectoryConfig {
    pub inbox: String,
    pub next: String,
    pub someday: String,
    pub project: String,
    pub archive: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;

        Ok(config)
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = shellexpand::tilde("~/.config/snail-cli");
        Ok(PathBuf::from(config_dir.as_ref()).join("config.toml"))
    }

    pub fn template_dir() -> Result<PathBuf> {
        let config_dir = shellexpand::tilde("~/.config/snail-cli");
        Ok(PathBuf::from(config_dir.as_ref()).join("templates"))
    }

    pub fn root_dir(&self) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(&self.general.root_dir);
        Ok(PathBuf::from(expanded.as_ref()))
    }

    pub fn inbox_dir(&self) -> Result<PathBuf> {
        Ok(self.root_dir()?.join(&self.directories.inbox))
    }

    pub fn next_dir(&self) -> Result<PathBuf> {
        Ok(self.root_dir()?.join(&self.directories.next))
    }

    pub fn someday_dir(&self) -> Result<PathBuf> {
        Ok(self.root_dir()?.join(&self.directories.someday))
    }

    pub fn project_dir(&self) -> Result<PathBuf> {
        Ok(self.root_dir()?.join(&self.directories.project))
    }

    pub fn archive_dir(&self) -> Result<PathBuf> {
        Ok(self.root_dir()?.join(&self.directories.archive))
    }

    pub fn get_template_path(&self, template_name: &str) -> Result<PathBuf> {
        let template_path = match template_name {
            "memo" => &self.templates.memo,
            "todo" => &self.templates.todo,
            "project" => &self.templates.project,
            "daily_report" => &self.templates.daily_report,
            _ => anyhow::bail!("Unknown template: {}", template_name),
        };

        let expanded = shellexpand::tilde(template_path);
        Ok(PathBuf::from(expanded.as_ref()))
    }
}

impl Default for Config {
    fn default() -> Self {
        // Fallback to repository templates if config file doesn't exist
        Self {
            general: GeneralConfig {
                root_dir: "~/memo".to_string(),
                editor: "vim".to_string(),
                date_format: "%Y-%m-%d".to_string(),
            },
            templates: TemplateConfig {
                memo: "~/.config/snail-cli/templates/memo.md".to_string(),
                todo: "~/.config/snail-cli/templates/todo.md".to_string(),
                project: "~/.config/snail-cli/templates/project.md".to_string(),
                daily_report: "~/.config/snail-cli/templates/daily_report.md".to_string(),
            },
            directories: DirectoryConfig {
                inbox: "00000_INBOX".to_string(),
                next: "00100_NEXTACTION".to_string(),
                someday: "00500_いつかやる".to_string(),
                project: "00800_プロジェクト".to_string(),
                archive: "99999_アーカイブ".to_string(),
            },
        }
    }
}
