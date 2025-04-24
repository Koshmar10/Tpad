use std::{fs, path::PathBuf};

use dirs::config_dir;
use serde::{Deserialize, Deserializer, Serialize};
use ratatui::style::Color;
use serde_json::from_str;
use crate::tpad_error;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    pub status: StatusSytle,
    pub tabs: TabsStyle,
    pub editor: EditorStyle,
    pub command: CommandStyle
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusSytle {
    pub foreground: String,
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
fn get_theme_file_path() ->PathBuf {
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
        let theme_conf: Theme = match fs::read_to_string(&path){
            Ok(str) => {
                let tm: Theme= toml::from_str(&str).unwrap();
                //fs::write("log.txt", format!("{:?}", tm)).unwrap();
                Theme {
                    editor: tm.editor,
                    status: tm.status,
                    command: tm.command,
                    tabs:tm.tabs,
                }
            }
            Err(_) => {
                let x= Theme::default();
                fs::write(
                    &path,
                    toml::to_string(&x).expect("Failed to serialize theme")
                )
                .expect("Failed to write theme file");
                //fs::write("log.txt", format!("{:?}", x)).unwrap();
                x
            }
        };
        theme_conf
    }
}