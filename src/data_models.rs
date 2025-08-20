use std::{path::PathBuf, usize};

use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use crate::theme::*;
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
}

pub struct App {
    pub theme: Theme,
    pub selected_theme: usize,
    
    pub clipboard: ClipboardContext,
    pub documents: Vec<Document>,
    pub window_height: u16,

    pub input_buffer: String,

    pub active: usize,
    pub running: bool,
    
    pub popup: Option<Popup>,

    pub focus: Windows,
    pub curs_x: usize,
    pub default_dir: PathBuf,

}
pub enum Windows {
    Editor,
    Command,
}
pub struct LayoutSnapshot {
    pub status_area: Rect,
    pub tab_area: Rect,
    pub editor_area: Rect,
    pub command_area: Rect,
}
pub struct RenderContext<'a> {
    pub theme: &'a Theme,
    pub selected_theme: &'a usize,

    pub popup: &'a Option<Popup>,
    pub documents: &'a Vec<Document>,
    pub input_buffer: &'a String,
    pub active: &'a usize,
    pub running: &'a bool,
    pub focus: &'a Windows,
    pub curs_x: &'a usize,
    pub default_dir: &'a PathBuf,
}

pub struct Document {
    pub file_path: String,
    pub permissions: String,
    pub size: u64,
    pub content: Vec<String>, // Changed from String to Vec<String>
    pub state: EditorState,
}
pub struct EditorState {
    pub curs_x: usize,
    pub curs_y: usize,
    pub is_dirty: bool,
    
    pub window_height: usize,
    pub scroll_offset: usize,
    pub find_active: bool,

    pub current_match: usize,
    pub highlights: Vec<(usize, usize, usize)>,
    
    pub undo_stack: UndoStack,
    pub selection: Option<((usize, usize), (usize, usize))>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EditOp {
    InsertChar {
        line: usize,
        col: usize,
        ch: char,
        applied: bool,
    },
    DeleteChar {
        line: usize,
        col: usize,
        ch: char,
        applied: bool,
    },
    SplitLine {
        first_line: usize,
        split_index: usize,
        second_line: usize,
        applied: bool,
    }, // ← Enter key
    MergeLines {
        merged_line: usize,
        merge_point: usize,
        applied: bool,
    }, // ← Undo of SplitLine
    DeleteSelection{
        start: (usize, usize),
        stop: (usize, usize),
        selection: String,
        applied: bool,
    },
    InsetSelection{
        applied: bool,
        start: (usize, usize),
        stop: (usize, usize),
        selection: String,
    }
    
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UndoStack {
    pub stack: Vec<EditOp>,
    pub cursor: usize,
}

pub enum Operations {
    Open(String),
    WordCount(String),
    Find(String),
    Change(usize),
    List,
    Close,
    Exit,
    None,
    SetDefaultDir(String),
}

#[derive(Serialize, Deserialize)]
pub struct SavedSession {
    pub saved_files: Vec<String>,
    pub undo_bufs: Vec<UndoStack>,
    pub active: usize,
}

pub enum PopupTypes {
    ErrorPopup,
    SaveOnClosePopup,
    ThemeSelectPopup,
    InfoPopup,
}
pub struct Popup {
    pub kind: PopupTypes,
    pub msg: String,
}