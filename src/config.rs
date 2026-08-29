//! Settings and reading positions, persisted between runs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ui::skin::{Accent, FocusMark, SkinId};

pub const WPM_RANGE: std::ops::RangeInclusive<u32> = 60..=1200;
pub const FONT_RANGE: std::ops::RangeInclusive<u32> = 18..=180;
pub const CHUNK_RANGE: std::ops::RangeInclusive<usize> = 1..=4;

const MAX_RECENTS: usize = 12;

/// One remembered document, so reopening resumes where you stopped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recent {
    pub path: PathBuf,
    pub title: String,
    pub index: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub wpm: u32,
    pub font_size: u32,
    pub chunk: usize,
    /// Hold longer on punctuation and long words.
    pub smart_pacing: bool,
    pub skin: SkinId,
    pub accent: Accent,
    /// How the pivot character is marked.
    pub focus_mark: FocusMark,
    /// Show the surrounding-text panel.
    pub show_context: bool,
    pub recents: Vec<Recent>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            wpm: 300,
            font_size: 64,
            chunk: 1,
            smart_pacing: true,
            skin: SkinId::default(),
            accent: Accent::default(),
            focus_mark: FocusMark::default(),
            show_context: false,
            recents: Vec::new(),
        }
    }
}

impl Config {
    /// `~/Library/Application Support/Sxarty/config.json` on macOS,
    /// `~/.config/sxarty/config.json` on Linux.
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "Sxarty")
            .map(|dirs| dirs.config_dir().join("config.json"))
    }

    /// Loads the stored config, falling back to defaults on anything unexpected.
    ///
    /// A corrupt or older config must never stop the app from starting, so all
    /// failures here are silent and non-fatal.
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Config::default();
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Config::default();
        };

        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Records a reading position, moving the document to the top of the list.
    pub fn remember(&mut self, path: &Path, title: &str, index: usize, total: usize) {
        self.recents.retain(|recent| recent.path != path);
        self.recents.insert(
            0,
            Recent {
                path: path.to_path_buf(),
                title: title.to_string(),
                index,
                total,
            },
        );
        self.recents.truncate(MAX_RECENTS);
    }

    pub fn resume_index(&self, path: &Path) -> Option<usize> {
        self.recents
            .iter()
            .find(|recent| recent.path == path)
            .map(|recent| recent.index)
    }

    pub fn clamp(&mut self) {
        self.wpm = self.wpm.clamp(*WPM_RANGE.start(), *WPM_RANGE.end());
        self.font_size = self.font_size.clamp(*FONT_RANGE.start(), *FONT_RANGE.end());
        self.chunk = self.chunk.clamp(*CHUNK_RANGE.start(), *CHUNK_RANGE.end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_or_missing_fields_fall_back_to_defaults() {
        let config: Config = serde_json::from_str(r#"{"wpm": 450, "nonsense": true}"#).unwrap();
        assert_eq!(config.wpm, 450);
        assert_eq!(config.font_size, Config::default().font_size);
    }

    #[test]
    fn remembering_the_same_file_twice_keeps_one_entry_on_top() {
        let mut config = Config::default();
        config.remember(Path::new("/a.txt"), "a", 10, 100);
        config.remember(Path::new("/b.txt"), "b", 5, 50);
        config.remember(Path::new("/a.txt"), "a", 42, 100);

        assert_eq!(config.recents.len(), 2);
        assert_eq!(config.recents[0].index, 42);
        assert_eq!(config.resume_index(Path::new("/a.txt")), Some(42));
    }
}
