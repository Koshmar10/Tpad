use std::{error::Error, fs, os::unix::fs::MetadataExt};

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
        use std::io::ErrorKind;
        let contents = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                fs::File::create(file_path)?;
                
                String::new()
            }
            Err(e) => return Err(Box::new(e)),
        };
        let metadata = fs::metadata(file_path)?;
        
        let size = metadata.size();
        let lines = contents.lines().map(|line| line.to_string()).collect::<Vec<_>>();
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
        let new_content: Vec<String> = {
            let mut new = Vec::new();
            for line in &self.content {
                if line.is_empty(){
                    new.push(String::new());
                }
                else{
                    if line.contains('\n'){
                        for spl in line.split('\n'){
                            new.push(spl.to_string());
                        }
                    }
                    else{
                        new.push(line.to_owned());
                    }
                }
                
            }
            new
        };

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
    pub fn word_count(&self, word: &str) -> u32 {
        let mut findings: u32 = 0;
        for line in &self.content {
            for item in line.split_whitespace() {
                if item.contains(word) {
                    findings += 1;
                }
            }
        }
        return findings
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
            InsertSelection{start: (usize, usize), stop: (usize,usize), selection: String},
            DeleteSelection{start: (usize, usize), stop:(usize, usize)}
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
                EditOp::SplitLine { first_line, second_line, applied, .. } => {
                    if *applied {
                        return Ok(());
                    }
                    let l1 = *first_line;
                    let l2 = *second_line;
                    *applied = true;
                    // Undo a split by merging the two split lines.
                    OpAction::MergeLines { first_line: l1, second_line: l2 }
                }
                EditOp::MergeLines { merged_line, merge_point, applied, .. } => {
                    if *applied {
                        return Ok(());
                    }
                    let ml = *merged_line;
                    let mp = *merge_point;
                    *applied = true;
                    // Undo a merge by splitting the merged line at merge_point.
                    OpAction::SplitLine { merged_line: ml, merge_point: mp }
                }
                EditOp::DeleteSelection { start, stop, selection, applied,  } => {
                    if *applied {
                        return Ok(());
                        
                    }
                    *applied = true;
                    if *start > *stop {
                        let temp = *start;
                        *start = *stop;
                        *stop = temp;
                    }
                    OpAction::InsertSelection{start: *start, stop: *stop, selection: selection.to_owned()}
                }
                EditOp::InsetSelection { applied, start, stop, .. } => {
                    if *applied {
                        return Ok(());
                    }
                    *applied = true;
                    let mut s = *start;
                    let mut e = *stop;
                    if s > e {
                        std::mem::swap(&mut s, &mut e);
                    }
                    OpAction::DeleteSelection { start: s, stop: e }
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
            OpAction::InsertSelection { start, stop, selection } =>{
                self.insert_selection(start, selection);
                self.adjust_cursor(stop.0, stop.1, false);
            }
            OpAction::DeleteSelection { start, stop } => {
                self.delete_selection(start, stop);
                self.adjust_cursor(start.0,start.1, false);
                if start.0 < self.content.len(){
                    move_curs(self, CursorDirection::Up);
                }
            }
        }

        Ok(())
    }

        // Update the undo stack cursor and refresh the document content.
        
    
    
    pub fn redo(&mut self) {
        // First, extract operation details and update the applied flag.
      
            let op_stack = &mut self.state.undo_stack;
            if op_stack.cursor >= op_stack.stack.len() {
            return;
            }
            let op = &mut op_stack.stack[op_stack.cursor];
            let action : Option<EditOp> = {
                match op {
                    EditOp::InsertChar { applied, .. } => {
                        if *applied {
                            *applied = false;
                            Some(op.clone())
                //self.insert_char(*line, *col, *ch);
                } else {
                    None
                }
            }
            EditOp::DeleteChar { applied, .. } => {
                if *applied {
                    *applied = false;
                    // self.delete_char(*line, *col);
                    Some(op.clone())
                } else {
                    None
                }
            }
            EditOp::MergeLines { applied, .. } => {
                if *applied {
                *applied = false;
                Some(op.clone())
                /*
                if *merged_line + 1 < self.content.len() {
                    self.merge_lines(*merged_line, *merged_line + 1);
                }
                */
                } else {
                    None
                }
            }
            EditOp::SplitLine { applied, .. } => {
                if *applied {
                *applied = false;
                    Some(op.clone())
                }else {
                    None
                }
            }
            EditOp::DeleteSelection { applied, .. } => {
                if *applied {
                *applied = false;
                Some(op.clone())
                } else {
                    None
            }
            }
            EditOp::InsetSelection { applied, .. } => {
                if *applied {
                    *applied = false;
                    Some(op.clone())
                }else {
                    None
                }
            }
            
        }
    };
        match action {
            Some(op) =>{
                match op {
                    EditOp::DeleteChar { line, col, .. } => {
                        self.delete_char(line, col);
                        self.adjust_cursor(line, col-1, false);
                    }
                    EditOp::DeleteSelection { start, stop, .. } =>{
                        self.delete_selection(start, stop);
                    }
                    EditOp::InsertChar { line, col, ch, .. } =>{
                        self.insert_char(line, col, ch);
                        self.adjust_cursor(line, col+1, false);
                    }
                    EditOp::MergeLines { merged_line, .. } =>{
                        self.merge_lines(merged_line, merged_line+1);
                    }
                    EditOp::SplitLine { first_line, split_index, .. } =>{
                        self.split_lines(first_line, split_index);
                    }
                    EditOp::InsetSelection { start, stop, selection, .. } =>{
                        self.insert_selection(start, selection);
                        self.adjust_cursor(stop.0, stop.1, false);
                    }
                    
                }
            }
            None => {}
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
    pub fn insert_selection(&mut self, start: (usize, usize), st: String) {
        // Ensure enough lines exist
        while start.0 >= self.content.len() {
            self.content.push(String::new());
        }
        let line_idx = start.0;
        let col_idx = start.1;
        // Extract the original line to split into prefix and suffix
        let orig_line = self.content.remove(line_idx);
        let prefix: String = orig_line.chars().take(col_idx).collect();
        let suffix: String = orig_line.chars().skip(col_idx).collect();
        let parts: Vec<&str> = st.split('\n').collect();
        if parts.len() == 1 {
            // Single-line insertion
            self.content.insert(line_idx, format!("{}{}{}", prefix, parts[0], suffix));
        } else {
            // Multi-line insertion
            let mut idx = line_idx;
            for (i, part) in parts.iter().enumerate() {
                let new_line = if i == 0 {
                    format!("{}{}", prefix, part)
                } else if i == parts.len() - 1 {
                    format!("{}{}", part, suffix)
                } else {
                    part.to_string()
                };
                self.content.insert(idx, new_line);
                idx += 1;
            }
        }
        self.update_content();
        // Remove any active highlight
        self.unhighlight();
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
    pub fn delete_selection(&mut self, start: (usize, usize), stop: (usize, usize)) -> String {
        //order start and stoop
        let ((start_line, start_col), (stop_line, stop_col)) =
        if start.0 > stop.0 || (start.0 == stop.0 && start.1 > stop.1) {
            (stop, start)
        } else {
            (start, stop)
        };
        
        if start_line == stop_line {
            let new_start_col = start_col.min(stop_col);
            let new_stop_col = stop_col.max(start_col);
            if new_stop_col - new_start_col == 
            self.content[start_line].len() 
            {
                let mut  deleted: String;
                deleted = self.content[start_line].clone(); 
                self.content.remove(start_line);
                match self.content.get(start_line){
                    Some(s) => {
                        self.state.curs_x = s.len();
                    }
                    None =>{
                        self.state.curs_x =0;
                    }
                }
                if start_line == self.content.len(){move_curs(self, CursorDirection::Up)}

                self.update_content();
                self.state.selection = None;
                deleted.push('\n');
                deleted
            }
            else{
                let mut deleted = Vec::new();
                self.content[start_line]={
                    self.content[start_line]
                    .chars().enumerate()
                    .filter_map(
                        |(i, c)| {
                            if !(new_start_col<=i && new_stop_col>i) {
                                Some(c)
                            }else{
                                deleted.push(c);
                                None
                            }
                        }
                    ).collect()
                };
                self.state.selection = None;
                self.state.curs_x = new_start_col;
                deleted.iter().collect()
            }
        } 
        else {
            let mut deleted = Vec::new();
            let add_final_endl = if stop_col == self.content[stop_line].len() {true} else {false};

            let last_elem = self.content[stop_line][..stop_col].to_owned();
            self.content[stop_line] = self.content[stop_line][stop_col..].to_string();
            if self.content[stop_line].len() == 0{
                self.content.remove(stop_line);
            }
            let frist_elem = self.content[start_line][start_col..].to_string();
            
            self.content[start_line]=self.content[start_line][..start_col].to_string();
            deleted.push(frist_elem);
            self.content = {
                
                self.content.iter().enumerate().filter_map(
                    |(i,line)|
                    {
                        if i>start_line && i<stop_line {
                            deleted.push(line.to_owned());
                            None
                        }else {Some(line.to_owned())}
                    }
                ).collect()
            };
                if start_line+1 < self.content.len(){
                    self.merge_lines(start_line, start_line+1);
                }
                self.state.selection = None;
                self.state.curs_x = start_col; 
                self.state.curs_y=start_line;  
                deleted.push(last_elem);
                let deleted = deleted.join("\n");
                if add_final_endl {
                    deleted +"\n"
                }
                else{
                    deleted
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
