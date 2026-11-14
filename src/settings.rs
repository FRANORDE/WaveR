// Settings persistence for WaveR.
// Full design notes: agent/settings-design.md (explanations, patterns,
// gotchas). Rules here: load() never panics (any failure = defaults),
// save() logs errors but never crashes, defaults mirror WaveR::new().

use std::path::{Path, PathBuf};

use configparser::ini::Ini;

use crate::{Language, RepeatMode};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub language: Language,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub show_shuffle_button: bool,
    pub show_loop_button: bool,
    pub show_queue: bool,
    pub show_history: bool,
    // steps 2-3: theme + colors, commented until those types exist
    // pub theme: Theme,
    // pub colors: Colors,
}

impl Settings {
    /// Load the real settings file; any failure = defaults.
    pub fn load() -> Self {
        Self::load_from(&settings_path())
    }

    /// Save to the real settings file; a failed write is logged only.
    pub fn save(&self) {
        self.save_to(&settings_path());
    }

    /// Same as load(), but from an explicit path (used by the tests).
    fn load_from(path: &Path) -> Self {
        let mut ini = Ini::new();
        ini.load(path).ok(); // missing/corrupt file -> empty ini -> defaults

        let language = match ini.get("wave-r", "language").as_deref() {
            Some("it") => Language::Italian,
            _ => Language::English, // fallback for unknown/missing values
        };

        Self {
            language,
            shuffle: read_bool(&ini, "shuffle", false),
            repeat: match ini.get("wave-r", "repeat").as_deref() {
                Some("one") => RepeatMode::One,
                Some("all") => RepeatMode::All,
                _ => RepeatMode::Off,
            },
            show_shuffle_button: read_bool(&ini, "show_shuffle_button", true),
            show_loop_button: read_bool(&ini, "show_loop_button", true),
            show_queue: read_bool(&ini, "show_queue", true),
            show_history: read_bool(&ini, "show_history", false),
        }
    }

    /// Same as save(), but to an explicit path (used by the tests).
    fn save_to(&self, path: &Path) {
        let mut ini = Ini::new();
        // set(): the value is Option<String>, None would DELETE the key
        ini.set("wave-r", "language", Some(match self.language {
            Language::English => "en".to_string(),
            Language::Italian => "it".to_string(),
        }));
        ini.set("wave-r", "shuffle", Some(self.shuffle.to_string()));
        ini.set("wave-r", "repeat", Some(match self.repeat {
            RepeatMode::Off => "off".to_string(),
            RepeatMode::One => "one".to_string(),
            RepeatMode::All => "all".to_string(),
        }));
        ini.set("wave-r", "show_shuffle_button", Some(self.show_shuffle_button.to_string()));
        ini.set("wave-r", "show_loop_button", Some(self.show_loop_button.to_string()));
        ini.set("wave-r", "show_queue", Some(self.show_queue.to_string()));
        ini.set("wave-r", "show_history", Some(self.show_history.to_string()));

        // the ~/.waver/config chain may not exist on the first run
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("settings: cannot create folder {:?}: {e}", parent);
            }
        }
        if let Err(e) = ini.write(path) {
            eprintln!("settings: cannot write file {:?}: {e}", path);
        }
    }
}

/// Read one boolean key, with a default for anything unexpected.
fn read_bool(ini: &Ini, key: &str, default: bool) -> bool {
    match ini.get("wave-r", key).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    }
}

/// Where the settings file lives: ~/.waver/config/settings.ini
pub fn settings_path() -> PathBuf {
    // home_dir() is Option<PathBuf>; "." is a harmless never-panic fallback
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".waver").join("config").join("settings.ini")
}

#[cfg(test)] // compiled only for `cargo test`
mod tests {
    use super::*;

    // tests must never touch the real home: temp files only
    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    #[test]
    fn roundtrip_every_setting() {
        // non-default values on purpose: every field must survive
        let original = Settings {
            language: Language::Italian,
            shuffle: true,
            repeat: RepeatMode::All,
            show_shuffle_button: false,
            show_loop_button: false,
            show_queue: false,
            show_history: true,
        };
        let path = temp_path("waver-test-roundtrip.ini");
        let _ = std::fs::remove_file(&path);

        original.save_to(&path);
        assert_eq!(Settings::load_from(&path), original);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_falls_back_to_app_defaults() {
        // these values must mirror WaveR::new(): the test fails if the
        // two sides drift apart
        let path = temp_path("waver-test-missing.ini");
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            Settings::load_from(&path),
            Settings {
                language: Language::English,
                shuffle: false,
                repeat: RepeatMode::Off,
                show_shuffle_button: true,
                show_loop_button: true,
                show_queue: true,
                show_history: false,
            }
        );
    }

    #[test]
    fn unknown_keys_are_ignored() {
        // a FUTURE version may write keys this version does not know:
        // upgrades must not require deleting the INI by hand
        let path = temp_path("waver-test-future-keys.ini");
        let _ = std::fs::remove_file(&path);

        let mut ini = Ini::new();
        ini.set("wave-r", "language", Some("it".to_string()));
        ini.set("wave-r", "theme", Some("dark".to_string())); // unknown key
        ini.write(&path).unwrap();

        // the unknown key is invisible, the known ones still load
        assert_eq!(Settings::load_from(&path).language, Language::Italian);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_value_falls_back_to_english() {
        let path = temp_path("waver-test-unknown.ini");
        let _ = std::fs::remove_file(&path);

        // hand-edited file with an unknown value
        let mut ini = Ini::new();
        ini.set("wave-r", "language", Some("fr".to_string()));
        ini.write(&path).unwrap(); // unwrap is fine in a test

        assert_eq!(Settings::load_from(&path).language, Language::English);
        let _ = std::fs::remove_file(&path);
    }
}
