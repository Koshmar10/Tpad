use dirs::config_dir;
use serde_json::json;
use std::{error::Error, fs, path::PathBuf};
use serde::{Serialize, Deserialize};
use crate::{App};
#[derive(Serialize, Deserialize)]
pub struct SavedSession {
    pub saved_files: Vec<String>,
    pub active: usize,
}
fn get_session_file_path() -> PathBuf {
    let base_dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
    base_dir.join("tpad").join("session.json")
}

pub fn save_session(app: &mut App) ->Result<(), Box<dyn Error>> {
    let session   = SavedSession{
        saved_files: app.documents.iter().map(
            |doc| doc.file_path.clone()
        ).collect(),
        active: app.active,
    };
    let path = get_session_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&session)?;
    fs::write(path, json)?; 
    
    Ok(())
}
pub fn load_session() -> Option<SavedSession> {
    let path = get_session_file_path();
    let content = fs::read_to_string(path).ok()?;        // read file or return None
    let session: SavedSession = serde_json::from_str(&content).ok()?; // parse JSON or return None
    Some(session)
}