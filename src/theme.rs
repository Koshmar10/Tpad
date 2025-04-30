use std::{ffi::OsString, fs::{self, DirEntry}, path::PathBuf};

use dirs::config_dir;
use serde::{Deserialize, Deserializer, Serialize};
use ratatui::style::Color;
use serde_json::from_str;
use crate::{ Popup};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    pub status: StatusSytle,
    pub tabs: TabsStyle,
    pub editor: EditorStyle,
    pub command: CommandStyle,
    pub popup: PopupStyle,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusSytle {
    pub foreground: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PopupStyle {
    pub bg: String,
    pub fg: String,
    pub error_fg: String,
    pub error_bg: String,
}

impl Default for PopupStyle {
    fn default() -> Self {
        PopupStyle {
            bg: String::from("#ffffff"),
            fg: String::from("#000000"),
            error_fg: String::from("#ff0000"),
            error_bg: String::from("#000000"),
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TabsStyle {

    pub active_bg: String,
    pub active_fg:String,
    pub inactive_bg: String,
    pub inactive_fg:String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditorStyle {

    pub background: String,
    pub foreground: String,
    pub highlights: String,
    pub cursor: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct  CommandStyle{
    pub background: String,
    pub foreground: String,
    pub cursor: String,

}

impl Default for Theme {
   fn default() -> Self {
        Theme {
            editor: EditorStyle::default(),
            command:CommandStyle::default(),
            tabs : TabsStyle::default(),
            status : StatusSytle::default(),
            popup : PopupStyle::default(),
        }
    }
}



impl Default for CommandStyle {
    fn default() -> Self {
        CommandStyle { 
            background: String::from("#ffffff"), 
            foreground: String::from("#ffffff"), 
            cursor: String::from("#0087f9")}
    }
}
impl Default for EditorStyle{
    fn default() -> Self {
        EditorStyle { 
            background: String::from("#ffffff"), 
            foreground: String::from("#ffffff"), 
            highlights: String::from("#f9d800"),
            cursor: String::from("#0087f9")}
    }
}
impl Default for TabsStyle { 
    fn default() -> Self {
        TabsStyle { 
            active_fg: String::from("#ffffff"),
            active_bg: String::from("#ffffff"),
            inactive_fg: String::from("#e7e7e7"),
            inactive_bg: String::from("#d5d5d5"),
        }
    }
}
impl Default for StatusSytle {
    fn default() -> Self {
        StatusSytle { foreground: String::from("#ffffff") }
    }
}
pub fn get_theme_file_path() ->PathBuf {
    let base_dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
    base_dir.join("tpad").join("theme.toml")
}

pub fn hex_to_color(hex: String) -> Color {
    if hex.starts_with('#') && hex.len() == 7 {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        return Color::Rgb(r, g, b);
    }
    Color::White
}
impl Theme {
    pub fn load() -> Theme {
        let path = get_theme_file_path();
        match fs::read_to_string(&path) {
            Ok(content) if !content.is_empty() => match toml::from_str(&content) {
                Ok(theme) => theme,
                Err(_parse_err) => {
                    let default_theme = Theme::default();
                    let _ = fs::write(&path, toml::to_string(&default_theme).expect("Serialization failed"));
                    default_theme
                }
            },
            _ => {
                let default_theme = Theme::default();
                let _ = fs::write(&path, toml::to_string(&default_theme).expect("Serialization failed"));
                default_theme
            }
        }
    }
    pub fn list_themes(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let base_dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
        let themes_dir = base_dir.join("tpad").join("themes");
        let entries = fs::read_dir(themes_dir)?;
        entries.map(|entry| entry.map(|e| e.path())).collect()
    }
}
