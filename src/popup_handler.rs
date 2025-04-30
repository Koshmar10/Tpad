use std::{fs, io::Write, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{data_models::*, theme::get_theme_file_path};
use crate::theme::*;


impl Popup {
    pub fn new(msg: String, kind: PopupTypes) -> Popup {
        
        match kind {
            other => Popup {
                kind: other,
                msg,
            }
        }
    }   
}
impl App {
    pub fn show_popup(&mut self, msg: String, kind: PopupTypes) {
        self.popup = Some(Popup::new(msg, kind));
    }
    pub fn dismiss_popup(&mut self) {
        self.popup = None;
    }
    pub fn handle_popup(&mut self, mut popup: Popup, key_event: KeyEvent) -> Option<Popup> {
        match popup.kind {
            PopupTypes::ErrorPopup => {
                // Dismiss the popup by returning None.
                None
            }
            PopupTypes::SaveOnClosePopup => {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('y'), KeyModifiers::NONE) => {
                        if let Some(active_doc) = self.documents.get_mut(self.active) {
                            active_doc.save_file().unwrap();
                        }
                        self.exit().ok();
                        // Dismiss the popup by returning None.
                        None
                    }
                    (KeyCode::Char('n'), KeyModifiers::NONE) => {
                        self.exit().ok();
                        // Dismiss the popup by returning None.
                        None
                    }
                    _ => {
                        // Return the unchanged popup.
                        Some(popup)
                    }
                }
            }
            PopupTypes::ThemeSelectPopup => {
                let themes = self.theme.list_themes();
                let theme_count = themes.as_ref().map(|t| t.len()).unwrap_or(0);
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        if theme_count > 0 {
                            self.selected_theme = (self.selected_theme + 1) % theme_count;
                        }
                        Some(popup)
                    }
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        self.selected_theme = self.selected_theme.saturating_sub(1);
                        Some(popup)
                    }
                    (KeyCode::Enter, KeyModifiers::NONE) => {
                        // Apply selected theme if needed.
                        let mut theme_to_apply : PathBuf;
                        match themes {
                            Ok(tms) => {
                            match tms.get(self.selected_theme)  {
                               Some(v) => {
                                    theme_to_apply = v.clone();
                               }
                               None =>{ 
                                return Some(Popup::new(String::from("unavailable index"), PopupTypes::ErrorPopup))
                               }
                            }
                            }
                            Err(e) =>{
                                return Some(Popup::new(e.to_string(), PopupTypes::ErrorPopup))
                            }
                        };
                        match fs::read_to_string(theme_to_apply.clone()) {
                            Ok(to_write) => {
                                match std::fs::OpenOptions::new().write(true).truncate(true).open(get_theme_file_path().clone()) {
                                    Ok(mut theme_file) => {
                                        if let Err(e) = write!(theme_file, "{}", to_write.as_str()) {
                                            return Some(Popup::new(e.to_string(), PopupTypes::ErrorPopup));
                                        }
                                        self.theme = Theme::load();
                                    }
                                    Err(_) => {
                                        match fs::File::create(get_theme_file_path()) {
                                            Ok(mut fil) => {
                                                if let Err(e) = fil.write_all(to_write.as_bytes()) {
                                                    return Some(Popup::new(e.to_string(), PopupTypes::ErrorPopup));
                                                }
                                            }
                                            Err(e) => {
                                                return Some(Popup::new(e.to_string(), PopupTypes::ErrorPopup));
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                return Some(Popup::new(e.to_string(), PopupTypes::ErrorPopup));
                            }
                        }

                        Some(popup)
                    }
                    (KeyCode::Char('q'), KeyModifiers::NONE) => {
                        // Dismiss the popup.
                        None
                    }
                    _ => Some(popup),
                }
            }
        }
    }
}







