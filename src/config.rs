use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct UserConfig {
    pub irc: Option<IrcConfig>,
    pub theme: Option<ThemeConfig>,
    pub emojis: Option<EmojiConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub accent: Option<String>,
    pub muted: Option<String>,
    pub icons: Option<bool>, // ← moved here
}

#[derive(Debug, Deserialize, Clone)]
pub struct IrcConfig {
    pub nick: Option<String>,
    pub port: Option<u16>,
    pub tls: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmojiConfig {
    #[serde(flatten)]
    pub aliases: std::collections::HashMap<String, String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            irc: None,
            theme: None,
            emojis: None,
        }
    }
}

impl UserConfig {
    pub fn load() -> Option<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return None;
        }
        let contents = fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/"))
            .join(".meow")
            .join("config.toml")
    }
}
