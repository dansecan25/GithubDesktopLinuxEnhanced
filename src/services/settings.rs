use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::window_state::WindowGeometry;
use crate::constants::app::{CONFIG_DIR_NAME, SETTINGS_FILE_NAME};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub repositories: Vec<PathBuf>,
    pub selected_repository: Option<PathBuf>,
    pub window: WindowGeometry,
    pub theme: ThemeChoice,
    /// Folder new clones and repositories are created in: the last one used, or the home folder.
    pub projects_directory: Option<PathBuf>,
    /// Command that opens the code editor, for example `code`. `None` picks the first installed one.
    pub editor: Option<String>,
    /// Login of the signed-in GitHub account. Its token is in the system keychain, never here.
    pub github_login: Option<String>,
    /// Check for and download new versions automatically.
    pub auto_update: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            repositories: Vec::new(),
            selected_repository: None,
            window: WindowGeometry::default(),
            theme: ThemeChoice::default(),
            projects_directory: None,
            editor: None,
            github_login: None,
            auto_update: true,
        }
    }
}

impl Settings {
    pub fn projects_directory(&self) -> PathBuf {
        self.projects_directory.clone().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("."))
    }

    /// Adds a repository (if new) and selects it.
    pub fn add_repository(&mut self, path: PathBuf) {
        if !self.repositories.contains(&path) {
            self.repositories.push(path.clone());
            self.repositories.sort_by_key(|repo| repo.to_string_lossy().to_lowercase());
        }
        self.selected_repository = Some(path);
    }

    pub fn remove_repository(&mut self, path: &Path) {
        self.repositories.retain(|repo| repo != path);
        if self.selected_repository.as_deref() == Some(path) {
            self.selected_repository = self.repositories.first().cloned();
        }
    }
}

pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(CONFIG_DIR_NAME).join(SETTINGS_FILE_NAME))
}

/// Loads settings, falling back to defaults when the file is missing or unreadable, so a broken
/// settings file can never stop the app from opening.
pub fn load() -> Settings {
    let Some(path) = settings_path() else { return Settings::default() };
    let Ok(text) = std::fs::read_to_string(&path) else { return Settings::default() };
    match serde_json::from_str(&text) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("Ignoring unreadable settings at {}: {error}", path.display());
            Settings::default()
        }
    }
}

/// Writes to a temporary file and renames it, so a crash mid-write can't leave a corrupt file.
pub fn save(settings: &Settings) -> std::io::Result<()> {
    let Some(path) = settings_path() else { return Ok(()) };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, path)
}
