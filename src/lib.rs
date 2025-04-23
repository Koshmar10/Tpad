use std::fs::metadata;
use std::os::unix::fs::MetadataExt;
use std::os::unix::raw::off_t;
use std::{collections::btree_map::Range, fs};
use std::error::Error;
use ratatui::prelude::Line;
use ratatui::{layout::Rect, style::{Color, Style}, text::Span};
use serde::{Deserialize, Serialize};
use std::io;
use color_eyre:: Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{layout::{Constraint, Direction, Layout}, text::Text, widgets::{Block, Borders, Paragraph}, DefaultTerminal, Frame};

pub mod session;
pub mod error;
pub mod ui;
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
}
pub struct App {
    pub documents: Vec<Document>,
    pub input_buffer: String,
    
    pub active: usize,
    pub running: bool,
    pub render_error: bool,
    pub error_msg: String,

    pub focus: Windows,

}

pub enum Windows {
    Editor,
    Command
}
impl App {
    pub fn new(file_args: Vec<String>) -> App {
        let files: Vec<String> = if !file_args.is_empty() { file_args } else { Vec::new() };

        let mut error_msg = String::new();
        let mut render_error = false;
        let mut undo_history: Vec<UndoStack> = Vec::new();

        let mut  old_docs: Vec<Document> = match session::load_session() {
            Some(session) => {
                undo_history = session.undoBufs;
                session.saved_files.iter()
                    .filter_map(|file_path| {
                        match Document::new(file_path) {
                            Ok(doc) => Some(doc),
                            Err(err) => {
                                error_msg.push_str(&format!("Error loading '{}': {}\n", file_path, err));
                                render_error = true;
                                None
                            }
                        }
                    })
                    .collect()
               
            }
            None => Vec::new(),
        };
        
        for (doc, stack) in old_docs.iter_mut().zip(undo_history) {
            doc.state.undo_stack =stack;
        }
        

        let new_docs: Vec<Document> = files.iter()
            .filter_map(|file_path| {
                match Document::new(file_path) {
                    Ok(doc) => Some(doc),
                    Err(err) => {
                        error_msg.push_str(&format!("Error loading '{}': {}\n", file_path, err));
                        render_error = true;
                        None
                    }
                }
            })
            .collect();

        let mut documents = old_docs;
        for doc in new_docs {
            if !documents.iter().any(|d| d.file_path == doc.file_path) {
                documents.push(doc);
            }
        }

        App {
            documents,
            active: 0,
            render_error,
            error_msg,
            running: true,
            input_buffer: String::new(),
            focus: Windows::Editor,
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|f: &mut Frame<'_>| {
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2),
                        Constraint::Length(2),  // Tabs
                        Constraint::Min(1),     // Editor
                        Constraint::Length(3),  // Command
                    ])
                    .spacing(0)
                    .split(area);

                // Render the tab bar
                self.render_status_bar(f, chunks[0]);
               
                self.render_tab_bar(f, chunks[1]);

                // Render the document view
                self.render_doc_view(f, chunks[2]);

                // Render the command line
                let cmd_text = vec![String::from(": "), self.input_buffer.clone()].join(" ");
                let cmd = Paragraph::new(Text::from(cmd_text))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(cmd, chunks[3]);

                // Render the error popup if `render_error` is true
                if self.render_error {
                    let popup_width = area.width / 2;
                    let popup_height = area.height / 2;
                    let popup_x = area.x + (area.width - popup_width) / 2;
                    let popup_y = area.y + (area.height - popup_height) / 2;

                    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
                    error::render_error_popup(f, popup_area, &self.error_msg);
                }
            })?;

            self.handle_events()?;
        }
        Ok(())
    }
    
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match (key_event.code, key_event.modifiers) {
            // Handle ':' to switch to Command mode
            (KeyCode::Char(':'), KeyModifiers::NONE) => {
                self.focus = Windows::Command;
            }

            // Handle 'Esc' to switch to Editor mode
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.focus = Windows::Editor;
            }

            // Handle 'Enter' to process input
            (KeyCode::Enter, KeyModifiers::NONE) => {
                match self.focus {
                    Windows::Command => {
                        let cmd = self.input_buffer.clone();
                        self.command_run(&cmd);
                        self.input_buffer.clear();
                    }
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                        let offset = active_doc.state.scroll_offset;
                        active_doc.unhighlight();
                        if offset + active_doc.state.curs_y >= active_doc.content.len() {
                            // Add a new empty line if the cursor is beyond the current content
                            active_doc.content.push(String::new());
                            active_doc.state.undo_stack.push(
                                EditOp::SplitLine { 
                                    first_line: offset + active_doc.state.curs_y,
                                    second_line: offset + active_doc.state.curs_y + 1, 
                                    applied: false 
                                }
                            );
                            active_doc.state.curs_y = active_doc.content.len() - offset - 1;
                        } else {
                            let current_line = &mut active_doc.content[offset + active_doc.state.curs_y];
                            if active_doc.state.curs_x >= current_line.len() {
                                // If pressing enter at the end of the line, insert an empty line after
                                active_doc.content.insert(offset + active_doc.state.curs_y + 1, String::new());
                                if active_doc.state.curs_y < active_doc.state.window_height - 2 &&
                                   offset + active_doc.state.curs_y < active_doc.content.len() - 1 {
                                    active_doc.state.curs_y += 1;
                                } else if active_doc.state.curs_y == active_doc.state.window_height - 2 {
                                    active_doc.state.scroll_offset += 1;
                                }
                            } else {
                                // Split the current line at the cursor position
                                let new_line = current_line.split_off(active_doc.state.curs_x);
                                active_doc.content.insert(offset + active_doc.state.curs_y + 1, new_line);
                                if active_doc.state.curs_y < active_doc.state.window_height - 2 &&
                                   offset + active_doc.state.curs_y < active_doc.content.len() - 1 {
                                    active_doc.state.curs_y += 1;
                                } else if active_doc.state.curs_y == active_doc.state.window_height - 2 {
                                    active_doc.state.scroll_offset += 1;
                                }
                            }
                            active_doc.state.undo_stack.push(
                                EditOp::SplitLine { 
                                    first_line: offset + active_doc.state.curs_y.saturating_sub(1),
                                    second_line: offset + active_doc.state.curs_y, 
                                    applied: false 
                                }
                            );
                        }
                        active_doc.state.curs_x = 0;
                        
                        self.documents[self.active].update_content();
                    }
                }
            }

            // Handle Ctrl + Q to quit the application
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.exit().unwrap_or_else(|e| self.throw_error(e)); // Fixed method name
            }

            // Handle Alt + Left Arrow to switch tabs left
            (KeyCode::Left, KeyModifiers::ALT) => {
                if self.active > 0 {
                    self.active -= 1;
                }
            }
            (KeyCode::Char(c), KeyModifiers::ALT) if c.is_ascii_digit() => {
                let index = c.to_digit(10).unwrap() as usize;
                if index > 0 && index <= self.documents.len() {
                    self.change(index-1);
                }
            }

            // Handle Alt + Right Arrow to switch tabs right
            (KeyCode::Right, KeyModifiers::ALT) => {
                if self.active < self.documents.len() - 1 {
                    self.active += 1;
                }
            }

            (KeyCode::Right | KeyCode::Left | KeyCode::Up | KeyCode::Down, KeyModifiers::NONE) => {
                match key_event.code {
                    KeyCode::Right => self.move_curs(CursorDirection::Right),
                    KeyCode::Left => self.move_curs
                    (CursorDirection::Left),
                    KeyCode::Down => self.move_curs(CursorDirection::Down),
                    KeyCode::Up => self.move_curs
                    (CursorDirection::Up),
                    
                    _ => {}
                }
            }

            // Handle Backspace to remove the last character from input buffer
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                match self.focus {
                    Windows::Command => {
                        self.input_buffer.pop();
                    }
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                        active_doc.unhighlight();
                        let offset = active_doc.state.scroll_offset;
                        let idx = offset + active_doc.state.curs_y;
                        // Check if the cursor is at the beginning of the line.
                        if active_doc.state.curs_x == 0 {
                            if active_doc.state.curs_y > 0 {
                                // Not the first visible line: merge with the previous visible line.
                                let merge_point = active_doc.content[idx - 1].len();
                                let to_move_up = active_doc.content[idx].clone();
                                active_doc.content.remove(idx);
                                active_doc.state.curs_y -= 1;
                                let new_idx = active_doc.state.scroll_offset + active_doc.state.curs_y;
                                active_doc.state.undo_stack.push(EditOp::MergeLines {
                                    merged_line: new_idx,
                                    merge_point,
                                    applied: false,
                                });
                                active_doc.content[new_idx].push_str(&to_move_up);
                                active_doc.state.curs_x = active_doc.content[new_idx].len();
                            } else if active_doc.state.scroll_offset > 0 {
                                // At the first visible line but with a scroll offset:
                                // scroll up and merge with the hidden previous line.
                                let new_idx = active_doc.state.scroll_offset - 1;
                                let merge_point = active_doc.content[new_idx].len();
                                let to_move_up = active_doc.content.remove(idx);
                                active_doc.state.scroll_offset -= 1;
                                active_doc.state.undo_stack.push(EditOp::MergeLines {
                                    merged_line: new_idx,
                                    merge_point,
                                    applied: false,
                                });
                                active_doc.content[new_idx].push_str(&to_move_up);
                                // curs_y remains 0 since the new first visible line is now the previous line.
                                active_doc.state.curs_x = active_doc.content[new_idx].len();
                            }
                        } else {
                            // Delete the character on the left of the cursor.
                            if let Some(line) = active_doc.content.get_mut(idx) {
                                let delete_index = active_doc.state.curs_x;
                                if delete_index > 0 && delete_index <= line.len() {
                                    let removed_char = line.remove(delete_index - 1);
                                    active_doc.state.undo_stack.push(EditOp::DeleteChar {
                                        line: idx,
                                        col: delete_index - 1,
                                        ch: removed_char,
                                        applied: false,
                                    });
                                    active_doc.update_content();
                                    self.move_curs(CursorDirection::Left);
                                }
                            }
                        }
                    }
                }
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) =>{
                self.documents[self.active].save_file().unwrap_or_else(
                    |e| self.throw_error(e)
                )
            }
            (KeyCode::Char('z') | KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                match key_event.code{
                    KeyCode::Char('z') =>{
                        self.documents[self.active].undo();
                    }
                    KeyCode::Char('y') => {
                        self.documents[self.active].redo();
                    }
                    _ => {}
                }
            }
            // Handle regular character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                match self.focus {
                    Windows::Command => {
                        match key_event.modifiers{
                            KeyModifiers::SHIFT => self.input_buffer.push(c.to_ascii_uppercase()),
                            KeyModifiers::NONE => self.input_buffer.push(c),
                            _=>{}
                            
                        }
                        
                    }
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                        active_doc.unhighlight();
                        let offset = active_doc.state.scroll_offset;
                        // Ensure the content vector has enough lines
                        while active_doc.state.curs_y >= active_doc.content.len() {
                            active_doc.content.push(String::new());
                        }

                        // Insert the character at the cursor position
                        active_doc.content[offset + active_doc.state.curs_y].insert(
                            active_doc.state.curs_x,
                            if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                                active_doc.state.undo_stack.push(EditOp::InsertChar 
                                    { line: offset + active_doc.state.curs_y, col: active_doc.state.curs_x, ch: c.to_ascii_uppercase(), applied: false });
                                c.to_ascii_uppercase()
                            } else {
                                active_doc.state.undo_stack.push(EditOp::InsertChar 
                                    { line: offset + active_doc.state.curs_y, col: active_doc.state.curs_x, ch: c, applied:false });
                                c
                            },
                        );
                        

                        // Update the document content and move the cursor
                        active_doc.update_content();
                        self.move_curs(CursorDirection::Right);
                    }
                }
            }
            _ => {}
        }
       
    }
    
    

    pub fn move_curs(&mut self, direction: CursorDirection) {
        // Get the currently active document
        if let Some(doc) = self.documents.get_mut(self.active) {
            match direction {
                CursorDirection::Left => {
                    if doc.state.curs_x > 0 {
                        doc.state.curs_x -= 1;
                    }
                }
                CursorDirection::Right => {
                    if doc.content[doc.state.scroll_offset+ doc.state.curs_y].len()+1 > doc.state.curs_x + 1{
                    doc.state.curs_x += 1;
                    }
                }
                CursorDirection::Up => {
                    
                    if doc.state.curs_y > 0 {
              
                        doc.state.curs_y -= 1;
                        if doc.state.curs_x > doc.content[doc.state.scroll_offset+ doc.state.curs_y].len() {
                            doc.state.curs_x = doc.content[ doc.state.scroll_offset+doc.state.curs_y].len();
                        }
                    } 
                     else{
                        doc.state.scroll_offset = doc.state.scroll_offset.saturating_sub(1);
                     }
                }
                CursorDirection::Down => {
                    if doc.state.curs_y < doc.state.window_height - 2 && 
                       doc.state.curs_y + doc.state.scroll_offset < doc.content.len()-1 {
                        doc.state.curs_y += 1;
                        if doc.state.curs_x > doc.content[doc.state.scroll_offset + doc.state.curs_y].len() {
                            doc.state.curs_x = doc.content[doc.state.scroll_offset + doc.state.curs_y].len();
                        }
                    } else if doc.state.curs_y + doc.state.scroll_offset < doc.content.len()-1 {
                        doc.state.scroll_offset += 1;
                    }
                   
                }
            }
        }
    }

    pub fn open(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let file_path = file_path.to_string();
        let doc = Document::new(&file_path)?;
        self.documents.push(doc);
        self.active = self.documents.len() - 1;
        Ok(())
    }
    pub fn list_docs(&mut self) -> Vec<String> {
        self.documents.iter().map(|doc| doc.file_path.clone()).collect()
    }
    
    
    pub fn close(&mut self){
        if self.documents.len() == 0 {
            return;
        }
        self.documents.remove(self.active);
        self.active = self.active.saturating_sub(1);
        return;
    }


    pub fn view(&self) -> Vec<String> {
    if self.documents.is_empty() {
        return vec![];
    }
    let active_doc = &self.documents[self.active];
    active_doc
        .content
        .iter()
        .enumerate()
        .map(|(index, line)| format!("{}: {}", index + 1, line))
        .collect()
    }
    pub fn change(&mut self, index: usize){
        self.active = index;
    }
    pub fn command_parse(&self, cmd: &str) -> Result<Option<Operations>, Box<dyn Error>> {
        let mut parts = cmd.trim().split_whitespace();
        let command = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        if args.len() > 1 {
            return Err("Too many args".into());
        }
        if command.trim() == "open" {
            Ok(Some(Operations::Open(String::from(args[0]))))
        } else if command.trim() == "find" {
            Ok(Some(Operations::Find(String::from(args[0]))))
        } else if command.trim() == "count" {
            Ok(Some(Operations::WordCount(String::from(args[0]))))
        }else if command.trim() == "list" {
                Ok(Some(Operations::List))
        } else if command.trim() == "exit" {
            Ok(Some(Operations::Exit))
        }else if command.trim() == "close" {
            Ok(Some(Operations::Close))
        } else if command.trim() == "change" {
            let index = args[0].parse::<usize>()?;
            Ok(Some(Operations::Change(index)))
        }
         else {
            Err("Invalid command ".into())
        }
    }

    pub fn command_run(&mut self, cmd: &str) {
        match self.command_parse(cmd) {
            Ok(cmd) => match cmd {
                Some(Operations::Open(file_path)) => {
                    let result = self.open(&file_path);
                    if let Err(e) = result {
                        self.throw_error(e); // Fixed method name
                    }
                }
                Some(Operations::Find(word)) => {
                    let doc =&mut self.documents[self.active];
                    let matches = doc.find(&word);
                    doc.highlight(matches);

                 
                }
                Some(Operations::WordCount(word)) => {
                    let msg = self.documents[self.active].word_count(&word);
                    println!("{}", msg[0]);
                }
                Some(Operations::Exit) => {
                    self.exit().unwrap_or_else(|e| self.throw_error(e)); // Fixed method name
                }
                Some(Operations::List) => {
                    let msg = self.list_docs();
                    println!("{}", msg.join(" "));
                }
                Some(Operations::Close) => {
                    self.close();
                }
                Some(Operations::Change(index)) => {
                    self.change(index);
                }
                None => (),
            },
            Err(e) => {
                self.throw_error(e); // Fixed method name
            }
        }
    }
    pub fn exit(&mut self) -> Result<(), Box<dyn Error>>{
        session::save_session(self)?;
        self.running=false;
        Ok(())
    }
}
pub struct Document {
    pub file_path: String,
    pub permissions: String,
    pub size: u64,
    pub content: Vec<String>, // Changed from String to Vec<String>
    pub state: EditorState
}

pub struct EditorState {
    curs_x: usize,
    curs_y: usize,
    is_dirty: bool,
    window_height: usize,
    scroll_offset: usize,
    highlights: Vec<(usize, usize, usize)>,
    undo_stack: UndoStack
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct UndoStack {
    stack: Vec<EditOp>,
    cursor: usize,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
enum EditOp {
    InsertChar { line: usize, col: usize, ch: char, applied: bool },
    DeleteChar { line: usize, col: usize, ch: char, applied: bool },
    SplitLine { first_line: usize, second_line: usize, applied: bool }, // ← Enter key
    MergeLines { merged_line: usize, merge_point: usize,  applied: bool }, // ← Undo of SplitLine

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

impl EditorState {
    pub fn new(past_state: Option<EditorState>) -> EditorState {
        match past_state {
            Some(state) => state,
            None => EditorState { curs_x: 0, curs_y: 0, is_dirty: false, scroll_offset: 0, window_height: 0, highlights: Vec::new(), undo_stack:UndoStack::new(None)  }
        }
    }
}
impl UndoStack {
    pub fn new(past_stack: Option<UndoStack>) -> UndoStack {
        match past_stack {
            Some(state) => state,
            None => UndoStack { stack:Vec::new(), cursor: 0}
        }
    }
    pub fn push(&mut self, op: EditOp) {
        if self.cursor < self.stack.len() {
            self.stack.truncate(self.cursor);
        }
        self.stack.push(op);
        self.cursor = self.stack.len();
    }
    
}

impl Document {
    pub fn new(file_path: &String) -> Result<Document, Box<dyn Error>> {
        let contents = fs::read_to_string(file_path)?;
        let metadata = fs::metadata(file_path)?;
        let size = metadata.size();
        let lines = contents.lines().map(|line| line.to_string()).
        collect();
        let permissions = permission_string(metadata.mode(), metadata.is_dir());
        Ok(Document {
            file_path: file_path.clone(),
            permissions,
            size:size,
            content: lines,
            state: EditorState::new(None)

        })
    }
    pub fn update_content(&mut self) {
        // Ensure that empty lines are preserved
        self.state.is_dirty = true;
        let new_content: Vec<String> = self
            .content
            .iter()
            .map(|line| {
            if line.is_empty() {
                String::new() // Preserve empty lines
            } else {
                line.clone()
            }
            })
            .collect();
        self.content = new_content;
        // Update the file size based on the new content
        self.size = self.content.join("\n").len() as u64;
    }
    pub fn save_file(&mut self) -> Result<(), Box<dyn Error>>{
        if self.state.is_dirty {
            let content_sring = self.content.join("\n");
            let to = self.file_path.clone();
            fs::write(to, content_sring)?;
            self.state.is_dirty = false;
        }

        Ok(())
    }
    pub fn word_count(&self, word: &str) -> Vec<String> {
        println!("\ncount word: {word}");
        let mut findings: u32 = 0;
        for line in &self.content {
            for item in line.split_whitespace() {
                if item.contains(word) {
                    findings += 1;
                }
            }
        }
        let msg: String = if findings != 0 {
            format!("Found {} matches", findings)
        } else {
            "Didn't find any matches".to_string()
        };
        vec![msg]
    }

    pub fn find(&self, word: &str) -> Vec<(usize, usize, usize)>  {
        let mut  results = Vec::new();
        let word_size = word.len();

        for (line_index, line) in self.content.iter().enumerate() {
            let current_matches :Vec<(usize, usize, usize)>= 
            line.match_indices(word).map(|(start, _)| (line_index, start, start+word_size)).collect();
            results.extend(current_matches);
        }
        results
    }
    pub fn highlight(&mut self, v: Vec<(usize,usize,usize)>){
        self.state.highlights = v;
    }
    pub fn unhighlight(&mut self){
        self.state.highlights.clear();
    }
    pub fn undo(&mut self){
        // Scope the mutable borrow of the undo stack and extract the line, col, and add_offset flag
        let adjust = {
            let op_stack = &mut self.state.undo_stack;
            if op_stack.stack.is_empty(){
                return;
            }
            let index = op_stack.cursor.saturating_sub(1);
            match &mut op_stack.stack[index] {
                EditOp::InsertChar { line, col, ch: _, applied } => {
                    if !*applied {
                        self.content[*line].remove(*col);
                        *applied = true;
                        // For undoing an insertion, place the cursor at the removed char position
                        Some((*line, *col, false))
                    } else {
                        None
                    }
                }
                EditOp::DeleteChar { line, col, ch, applied } => {
                    if !*applied {
                        self.content[*line].insert(*col, *ch);
                        *applied = true;
                        // For undoing a deletion, move the cursor after the inserted char
                        Some((*line, *col, true))
                    } else {
                        None
                    }
                }
                EditOp::SplitLine { first_line, second_line, applied } => {
                    if !*applied {
                        let mut add_offset = false;
                        if self.content[*second_line].len() != 0 {
                            let to_move = self.content[*second_line].clone();
                            self.content[*first_line].push_str(&to_move);
                            add_offset = true;
                        } else {
                            if self.content[*first_line].len() != 0 {
                                add_offset = true;
                            }
                        }
                        self.content.remove(*second_line);
                        let aux = *first_line;
                        *applied = true;
                        Some((aux, self.content[aux].len().saturating_sub(1), add_offset))
                    } else {
                        None
                    }
                }
                EditOp::MergeLines { merged_line, merge_point, applied } => {
                    if !*applied {
                        // Undo a merge by splitting the merged line at merge_point.
                        let m_line = *merged_line;
                        let split_point = *merge_point;
                        let new_line = self.content[m_line].split_off(split_point);
                        self.content.insert(m_line + 1, new_line);
                        *applied = true;
                        Some((m_line, split_point, false))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };
        if let Some((line, col, add_offset)) = adjust {
            self.adjust_cursor(line, col, add_offset);
        }
        self.state.undo_stack.cursor = self.state.undo_stack.cursor.saturating_sub(1);
        self.update_content();
    }
 
    pub fn redo(&mut self) {
        // Scope the mutable borrow of the undo stack and extract the line, col, and add_offset flag
        let adjust = {
            let op_stack = &mut self.state.undo_stack;
            if op_stack.cursor >= op_stack.stack.len() {
                return;
            }
            match &mut op_stack.stack[op_stack.cursor] {
                EditOp::InsertChar { line, col, ch, applied } => {
                    if *applied {
                        self.content[*line].insert(*col, *ch);
                        *applied = false;
                        // For redoing an insertion, move the cursor after the inserted char
                        Some((*line, *col, true))
                    } else {
                        None
                    }
                }
                EditOp::DeleteChar { line, col, ch, applied } => {
                    if *applied {
                        self.content[*line].remove(*col);
                        *applied = false;
                        // For redoing a deletion, place the cursor at the removed char position
                        Some((*line, *col, false))
                    } else {
                        None
                    }
                }
                EditOp::MergeLines { merged_line, merge_point, applied } => {
                    if *applied {
                        // Redo a merge by removing the line after merged_line and appending its content.
                        let m_line = *merged_line;
                        if m_line + 1 < self.content.len() {
                            let second_line = self.content.remove(m_line + 1);
                            self.content[m_line].push_str(&second_line);
                        }
                        *applied = false;
                        Some((m_line, self.content[m_line].len().saturating_sub(1), false))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };
        if let Some((line, col, add_offset)) = adjust {
            self.adjust_cursor(line, col, add_offset);
        }
        self.state.undo_stack.cursor += 1;
        self.update_content();
    }

    // Helper function to adjust the cursor position based on an absolute line and column.
    fn adjust_cursor(&mut self, op_line: usize, op_col: usize, add_offset: bool) {
        // Adjust scroll offset if necessary to ensure the op_line is visible.
        if op_line < self.state.scroll_offset {
            self.state.scroll_offset = op_line;
        } else if op_line >= self.state.scroll_offset + self.state.window_height {
            self.state.scroll_offset = op_line.saturating_sub(self.state.window_height - 1);
        }
        // Set the cursor relative to the current viewport.
        self.state.curs_y = op_line.saturating_sub(self.state.scroll_offset);
        // When redoing an insertion (or undoing a deletion), we need the cursor to end up after the affected character.
        self.state.curs_x = op_col + if add_offset { 1 } else { 0 };
    }
}

pub fn permission_string(mode: u32, is_dir: bool) -> String {
    let file_type = if is_dir { 'd' } else { '-' };

    let rwx = |bit, r, w, x| {
        format!(
            "{}{}{}",
            if mode & r != 0 { 'r' } else { '-' },
            if mode & w != 0 { 'w' } else { '-' },
            if mode & x != 0 { 'x' } else { '-' },
        )
    };

    format!(
        "{}{}{}{}",
        file_type,
        rwx(mode, 0o400, 0o200, 0o100), // Owner
        rwx(mode, 0o040, 0o020, 0o010), // Group
        rwx(mode, 0o004, 0o002, 0o001), // Others
    )
}



/*
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn word_found_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test.txt"),
            content: vec![String::from("This is a test document, made for testing purposes only")],
        });
        let result = app.documents[0].find("test");
        assert_eq!(vec!["Word found"], result);
    }
    
    #[test]
    fn word_not_found_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test.txt"),
            content: vec![String::from("This is a test document, made for testing purposes only")],
        });
        let result = app.documents[0].find("mama");
        assert_eq!(vec!["Word not found"], result);
    }
    
    #[test]
    fn open_failed_test() {
        let mut app = App::new(vec![]);
        let result = app.open("nonexistent.txt");
        assert!(result.is_err(), "Expected an error when opening a non-existent file");
    }
    
    #[test]
    fn open_success_test() {
        let mut app = App::new(vec![]);
        // Replace "document.txt" with an actual file path that exists for testing
        let result = app.open("document.txt");
        assert!(result.is_ok(), "File opened successfully");
    }
    
    #[test]
    fn positive_word_count() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test.txt"),
            content: vec![String::from("This is a test document, made for testing purposes only")],
        });
        let result = app.documents[0].word_count("test");
        assert_eq!(vec!["Found 2 matches"], result);
    }
    
    
    #[test]
    fn list_docs_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test1.txt"),
            content: vec![String::from("Content of test1")],
        });
        app.documents.push(Document {
            file_path: String::from("test2.txt"),
            content: vec![String::from("Content of test2")],
        });
        let result = app.list_docs();
        assert_eq!(vec!["test1.txt", "test2.txt"], result);
    }
    
    #[test]
    fn close_document_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test1.txt"),
            content: vec![String::from("Content of test1")],
        });
        app.documents.push(Document {
            file_path: String::from("test2.txt"),
            content: vec![String::from("Content of test2")],
        });
        app.close();
        assert_eq!(app.documents.len(), 1);
        assert_eq!(app.documents[0].file_path, "test1.txt");
    }
    
    #[test]
    fn change_active_document_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test1.txt"),
            content: vec![String::from("Content of test1")],
        });
        app.documents.push(Document {
            file_path: String::from("test2.txt"),
            content: vec![String::from("Content of test2")],
        });
        app.change(1);
        assert_eq!(app.active, 1);
    }
}
    */