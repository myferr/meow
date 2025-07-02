use crossterm::style::Color;
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
        #[cfg(target_os = "windows")]
        {
            // Use %USERPROFILE%\meowconf\config.toml
            if let Some(home) = std::env::var_os("USERPROFILE") {
                return PathBuf::from(home).join("meowconf").join("config.toml");
            }
        }

        // Default to ~/.meow/config.toml on *NIX systems
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(home) = dirs::home_dir() {
                return home.join(".meow").join("config.toml");
            }
        }

        // Fallback
        PathBuf::from("config.toml")
    }
}

pub fn parse_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb { r, g, b })
    } else {
        None
    }
}
