use std::{error::Error, fs, os::unix::fs::MetadataExt};

use ratatui::symbols::line;

use crate::{app::move_curs, data_models::*};

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
        let ofs = self.scroll_offset;
        if let Some((start, _)) = self.selection {
            self.selection = Some((start, (ofs + y, x)));
        }
    }
    pub fn start_selection(&mut self, y: usize, x:usize) {
        let ofs = self.scroll_offset;
        if self.selection == None {
            self.selection = Some(((ofs+ y,x),(ofs +y,x)));
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
        if self.state.undo_stack.stack.is_empty() || self.state.undo_stack.cursor == 0 {
            return Err("No operations to undo".into());
        }

        // Decrement the cursor to get the index of the op to undo.
        self.state.undo_stack.cursor -= 1;
        let index = self.state.undo_stack.cursor;

        enum OpAction {
            DeleteChar { line: usize, col: usize },
            InsertChar { line: usize, col: usize, ch: char },
            MergeLines { first_line: usize, second_line: usize },
            SplitLine { merged_line: usize, merge_point: usize },
        }

        let action = {
            let op = &mut self.state.undo_stack.stack[index];
            match op {
                EditOp::InsertChar { line, col, applied, .. } => {
                    if *applied {
                        return Ok(()); // already undone
                    }
                    let l = *line;
                    let c = *col;
                    *applied = true;
                    // For an insertion, undo by deleting the char at col+1
                    OpAction::DeleteChar { line: l, col: c + 1 }
                }
                EditOp::DeleteChar { line, col, ch, applied } => {
                    if *applied {
                        return Ok(()); // already undone
                    }
                    let l = *line;
                    let c = *col;
                    let ch = *ch;
                    *applied = true;
                    // For a deletion, undo by inserting the missing char.
                    OpAction::InsertChar { line: l, col: c, ch }
                }
                EditOp::SplitLine { first_line, second_line, split_index, applied } => {
                    if *applied {
                        return Ok(());
                    }
                    let l1 = *first_line;
                    let l2 = *second_line;
                    *applied = true;
                    // Undo a split by merging the two split lines.
                    OpAction::MergeLines { first_line: l1, second_line: l2 }
                }
                EditOp::MergeLines { merged_line, merge_point, applied } => {
                    if *applied {
                        return Ok(());
                    }
                    let m_line = *merged_line;
                    let m_point = *merge_point;
                    *applied = true;
                    // Undo a merge by splitting the merged line at merge_point.
                    OpAction::SplitLine { merged_line: m_line, merge_point: m_point }
                }
            }
        };

        match action {
            OpAction::DeleteChar { line, col } => {
                self.delete_char(line, col);
                self.adjust_cursor(line, col.saturating_sub(1), false);
            }
            OpAction::InsertChar { line, col, ch } => {
                self.insert_char(line, col, ch);
                self.adjust_cursor(line, col, false);
            }
            OpAction::MergeLines { first_line, second_line } => {
                self.merge_lines(first_line, second_line);
                self.adjust_cursor(first_line, self.content[first_line].len(), false);
            }
            OpAction::SplitLine { merged_line, merge_point } => {
                self.split_lines(merged_line, merge_point);
                self.adjust_cursor(merged_line, merge_point.saturating_sub(1), false);
            }
        }

        Ok(())
    }

        // Update the undo stack cursor and refresh the document content.
        
    
    
    pub fn redo(&mut self) {
        // First, extract operation details and update the applied flag.
        let (action, line, col, ch, add_offset) = {
            let op_stack = &mut self.state.undo_stack;
            if op_stack.cursor >= op_stack.stack.len() {
                return;
            }
            match &mut op_stack.stack[op_stack.cursor] {
                EditOp::InsertChar { line, col, ch, applied } => {
                    if *applied {
                        *applied = false;
                        ("insert", *line, *col, Some(*ch), true)
                    } else {
                        return;
                    }
                }
                EditOp::DeleteChar { line, col, ch: _, applied } => {
                    if *applied {
                        *applied = false;
                        ("delete", *line, *col, None, false)
                    } else {
                        return;
                    }
                }
                EditOp::MergeLines { merged_line, merge_point: _, applied } => {
                    if *applied {
                        *applied = false;
                        ("merge", *merged_line, 0, None, false)
                    } else {
                        return;
                    }
                }
                EditOp::SplitLine { first_line, second_line: _, split_index, applied } => {
                    if *applied {
                        *applied = false;
                        ("split", *first_line, *split_index, None, false)
                    } else {
                        return;
                    }
                }
            }
        };

        // Now perform the redo action outside the mutable borrow of the undo stack.
        match action {
            "insert" => {
                self.insert_char(line, col, ch.unwrap());
            }
            "delete" => {
                self.delete_char(line, col);
            }
            "merge" => {
                // Make sure there is a next line to merge.
                if line + 1 < self.content.len() {
                    self.merge_lines(line, line + 1);
                }
            }
            "split" => {
                self.split_lines(line, col);
            }
            _ => {}
        }

        self.adjust_cursor(line, col, add_offset);
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

    pub fn insert_char(&mut self, line: usize, col:usize, c:char){

        while self.state.curs_y >= self.content.len() {
            self.content.push(String::new());
        }

        // Insert the character at the cursor position
        self.content[line].insert(col, c);

        // Update the document content and move the cursor
        self.update_content();
         let active_doc = self;
        move_curs(active_doc, CursorDirection::Right);
        active_doc.unhighlight();
    }
    pub fn delete_char(&mut self, line: usize, col:usize){
        if let Some(target) = self.content.get_mut(line) {
             
            let delete_index = col;
            if delete_index > 0 && delete_index <= target.len() {
                target.remove(delete_index - 1);
                self.update_content();
                move_curs(self, CursorDirection::Left);
            }
        }
    }
    pub fn merge_lines(&mut self, line1:usize, line2:usize){
        //remove highlight styiling
        self.unhighlight();

        let to_merge  = self.content[line2].to_owned(); 
        let merge_point = self.content[line1].len();
        self.content.remove(line2);
  
        self.content[line1].insert_str(merge_point, &to_merge);
       
        self.update_content();
       
        self.adjust_cursor(line1, merge_point, false);
    }
    pub fn split_lines(&mut self, line1:usize, split_index:usize){
        self.unhighlight();
        
        let split = &self.content[line1][split_index..].to_owned();
        
       
        if split_index == self.content[line1].len(){
            self.content.insert(line1+1, String::new());
            
        }else if split_index == 0 {
            self.content[line1].clear();
            self.content.insert(line1+1, split.to_owned());
        }
        else{
            self.content[line1] = self.content[line1][..split_index].to_string();
            self.content.insert(line1+1, split.to_owned());
        }
        self.update_content();
        self.state.curs_y+=1;
        self.state.curs_x=0;


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
