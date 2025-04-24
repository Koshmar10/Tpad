use std::{error::Error, fs, os::unix::fs::MetadataExt};

use crate::data_models::*;

impl EditorState {
    pub fn new(past_state: Option<EditorState>) -> EditorState {
        match past_state {
            Some(state) => state,
            None => EditorState {
                curs_x: 0,
                curs_y: 0,
                is_dirty: false,
                scroll_offset: 0,
                window_height: 0,
                find_active: false,
                current_match: 0,
                highlights: Vec::new(),
                undo_stack: UndoStack::new(None),
                selection: None,
            },
        }
    }
    pub fn update_selection_end(&mut self, y: usize, x: usize) {
        if let Some((start, _)) = self.selection {
            self.selection = Some((start, (y, x)));
        }
    }
    pub fn start_selection(&mut self, y: usize, x:usize) {
        if self.selection == None {
            self.selection = Some(((y,x),(y,x)));
        }
    }
}
impl UndoStack {
    pub fn new(past_stack: Option<UndoStack>) -> UndoStack {
        match past_stack {
            Some(state) => state,
            None => UndoStack {
                stack: Vec::new(),
                cursor: 0,
            },
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
        let lines = contents.lines().map(|line| line.to_string()).collect();
        let permissions = permission_string(metadata.mode(), metadata.is_dir());
        Ok(Document {
            file_path: file_path.clone(),
            permissions,
            size: size,
            content: lines,
            state: EditorState::new(None),
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
    pub fn save_file(&mut self) -> Result<(), Box<dyn Error>> {
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

    pub fn find(&self, word: &str) -> Vec<(usize, usize, usize)> {
        let mut results = Vec::new();
        let word_size = word.len();

        for (line_index, line) in self.content.iter().enumerate() {
            let current_matches: Vec<(usize, usize, usize)> = line
                .match_indices(word)
                .map(|(start, _)| (line_index, start, start + word_size))
                .collect();
            results.extend(current_matches);
        }
        results
    }
    pub fn highlight(&mut self, v: Vec<(usize, usize, usize)>) {
        self.state.highlights = v;
        self.state.find_active = true;
    }
    pub fn unhighlight(&mut self) {
        self.state.highlights.clear();
        self.state.find_active = false;
    }
    pub fn undo(&mut self) -> Result<(), Box<dyn Error>> {
        let adjust = {
            let op_stack = &mut self.state.undo_stack;
            if op_stack.stack.is_empty() {
                return Err("No operations to undo".into());
            }
    
            let index = op_stack.cursor.saturating_sub(1);
    
            match &mut op_stack.stack[index] {
                EditOp::InsertChar { line, col, applied, .. } => {
                    if *applied {
                        return Ok(()); // already undone
                    }
    
                    let line_str = self.content.get_mut(*line).ok_or("Invalid line index")?;
                    if *col >= line_str.len() {
                        return Err("Invalid character index".into());
                    }
    
                    line_str.remove(*col);
                    *applied = true;
    
                    Some((*line, *col, false))
                }
    
                EditOp::DeleteChar { line, col, ch, applied } => {
                    if *applied {
                        return Ok(()); // already undone
                    }
    
                    let line_str = self.content.get_mut(*line).ok_or("Invalid line index")?;
                    if *col > line_str.len() {
                        return Err("Invalid insert position".into());
                    }
    
                    line_str.insert(*col, *ch);
                    *applied = true;
    
                    Some((*line, *col, true))
                }
    
                EditOp::SplitLine { first_line, second_line, split_index, applied } => {
                    if *applied {
                        return Ok(());
                    }
    
                    if *first_line >= self.content.len() || *second_line >= self.content.len() {
                        return Err("Invalid line indices in SplitLine".into());
                    }
    
                    // Merge the split lines back together using split_index.
                    // We assume that the original split was performed at split_index.
                    let first_part = self.content[*first_line].clone();
                    let second_part = self.content[*second_line].clone();
                    self.content[*first_line] = format!("{}{}", first_part, second_part);
                    self.content.remove(*second_line);
    
                    *applied = true;
                    Some((*first_line, *split_index, true))
                }
    
                EditOp::MergeLines { merged_line, merge_point, applied } => {
                    if *applied {
                        return Ok(());
                    }
    
                    let m_line = *merged_line;
                    if m_line >= self.content.len() {
                        return Err("Invalid line index in MergeLines".into());
                    }
    
                    let split_point = *merge_point;
                    let original = &mut self.content[m_line];
                    if split_point > original.len() {
                        return Err("Merge point beyond line length".into());
                    }
    
                    let new_line = original.split_off(split_point);
                    self.content.insert(m_line + 1, new_line);
                    *applied = true;
    
                    Some((m_line, split_point, false))
                }
            }
        };
    
        if let Some((line, col, offset)) = adjust {
            self.adjust_cursor(line, col, offset);
        }
    
        self.state.undo_stack.cursor = self.state.undo_stack.cursor.saturating_sub(1);
        self.update_content();
        Ok(())
    }
    
    pub fn redo(&mut self) {
        // Scope the mutable borrow of the undo stack and extract the line, col, and add_offset flag
        let adjust = {
            let op_stack = &mut self.state.undo_stack;
            if op_stack.cursor >= op_stack.stack.len() {
                return;
            }
            match &mut op_stack.stack[op_stack.cursor] {
                EditOp::InsertChar {
                    line,
                    col,
                    ch,
                    applied,
                } => {
                    if *applied {
                        self.content[*line].insert(*col, *ch);
                        *applied = false;
                        // For redoing an insertion, move the cursor after the inserted char
                        Some((*line, *col, true))
                    } else {
                        None
                    }
                }
                EditOp::DeleteChar {
                    line,
                    col,
                    ch: _,
                    applied,
                } => {
                    if *applied {
                        self.content[*line].remove(*col);
                        *applied = false;
                        // For redoing a deletion, place the cursor at the removed char position
                        Some((*line, *col, false))
                    } else {
                        None
                    }
                }
                EditOp::MergeLines {
                    merged_line,
                    merge_point: _,
                    applied,
                } => {
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
                EditOp::SplitLine { first_line, second_line, split_index, applied } => {
                    if *applied {
                        let line_index = *first_line;
                        if line_index >= self.content.len() {
                            return;
                        }
                        let actual_split_index = *split_index;
                        let second_part = self.content[line_index].split_off(actual_split_index);
                        self.content.insert(line_index + 1, second_part);
                        *applied = false;
                        Some((line_index, actual_split_index, false))
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
    pub fn adjust_cursor(&mut self, op_line: usize, op_col: usize, add_offset: bool) {
        // If the operation line is above the current viewport, scroll up.
        let window_height = self.state.window_height - 2;
        if op_line < self.state.scroll_offset {
            self.state.scroll_offset = op_line;
        // If the operation line is below the visible area, scroll down.
        } else if op_line > self.state.scroll_offset + window_height {
            self.state.scroll_offset = op_line - window_height + 1;
        }
        // Calculate the cursor's position within the visible window and adjust upward by one line.
        self.state.curs_y = op_line.saturating_sub(self.state.scroll_offset);
        // Adjust the cursor column, adding one if needed.
        self.state.curs_x = op_col + if add_offset { 1 } else { 0 };
    }
}

pub fn permission_string(mode: u32, is_dir: bool) -> String {
    let file_type = if is_dir { 'd' } else { '-' };

    let rwx = |_bit, r, w, x| {
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
