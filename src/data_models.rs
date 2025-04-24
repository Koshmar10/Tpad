use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
}

pub struct App {
    pub documents: Vec<Document>,
    pub window_height: u16,
    pub input_buffer: String,

    pub active: usize,
    pub running: bool,
    pub render_error: bool,
    pub error_msg: String,

    pub exit_requested: bool,
    pub show_popup: bool,
    pub popup_message: String,

    pub focus: Windows,
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
    pub documents: &'a Vec<Document>,
    pub input_buffer: &'a String,
    pub active: &'a usize,
    pub running: &'a bool,
    pub render_error: &'a bool,
    pub error_msg: &'a String,
    pub exit_requested: &'a bool,
    pub show_popup: &'a bool,
    pub popup_message: &'a String,
    pub focus: &'a Windows,
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
}

#[derive(Serialize, Deserialize)]
pub struct SavedSession {
    pub saved_files: Vec<String>,
    pub undo_bufs: Vec<UndoStack>,
    pub active: usize,
}
