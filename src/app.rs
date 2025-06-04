use color_eyre::owo_colors::OwoColorize;
use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::layout::Direction;
use ratatui::symbols::line;
use ratatui::{DefaultTerminal, Frame};
use std::error::Error;
use std::fs;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io::{self};

use crate::data_models::*;

use crate::session;

use crate::theme::Theme;
use crate::ui::{popup, render};

impl App {
    pub fn new(file_args: Vec<String>) -> App {
        let files: Vec<String> = if !file_args.is_empty() {
            file_args
        } else {
            Vec::new()
        };

        let mut error_msg = String::new();
        let mut render_error = false;
        let mut undo_history: Vec<UndoStack> = Vec::new();

        let mut old_docs: Vec<Document> = match session::load_session() {
            Some(session) => {
                undo_history = session.undo_bufs;
                session
                    .saved_files
                    .iter()
                    .filter_map(|file_path| match Document::new(file_path) {
                        Ok(doc) => Some(doc),
                        Err(err) => {
                            error_msg
                                .push_str(&format!("Error loading '{}': {}\n", file_path, err));
                            render_error = true;
                            None
                        }
                    })
                    .collect()
            }
            None => Vec::new(),
        };

        for (doc, stack) in old_docs.iter_mut().zip(undo_history) {
            doc.state.undo_stack = stack;
        }

        let new_docs: Vec<Document> = files
            .iter()
            .filter_map(|file_path| match Document::new(file_path) {
                Ok(doc) => Some(doc),
                Err(err) => {
                    error_msg.push_str(&format!("Error loading '{}': {}\n", file_path, err));
                    render_error = true;
                    None
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
            theme:Theme::load(),
            selected_theme : 0,
            popup: None,
            clipboard: ClipboardContext::new().unwrap(),
            window_height: 0,
            documents,
            active: 0,
            running: true,
            input_buffer: String::new(),
            focus: Windows::Editor,
            curs_x: 0,
    
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            let ctx = RenderContext {
                theme: &self.theme,
                selected_theme: &self.selected_theme,
                documents: &self.documents,
                input_buffer: &self.input_buffer,
                popup: &self.popup,
                active: &self.active,
                running: &self.running,
                focus: &self.focus,
                curs_x: &self.curs_x,
            };

            terminal
                .draw(|f: &mut Frame<'_>| {
                    let layout = render::render_ui(f, &ctx);
                    self.window_height = layout.editor_area.height;
                })
                .unwrap();
            for doc in &mut self.documents {
                doc.state.window_height = self.window_height as usize;
            }

            self.handle_events()?;
        }
        Ok(())
    }
    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                if self.popup.is_some() {
                    let popup = self.popup.take().unwrap();
                    self.popup = self.handle_popup(popup, key_event);
                } else {
                    self.handle_key_event(key_event);
                }
            }
            _ => {}
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
                        self.curs_x =0;
                        self.input_buffer.clear();
                    }
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                       
                        let offset = active_doc.state.scroll_offset;
                        let split_index = active_doc.state.curs_x
                        ;
                        //if a split is initialized and the cursor is on the last line of the doc
                        //a new line is created so that he split does not create panic
                        if offset + active_doc.state.curs_y >= active_doc.content.len(){
                            active_doc.content.push(String::new());
                        }
                        if active_doc.state.curs_y >= active_doc.state.window_height-2 {
                            active_doc.state.scroll_offset+=1;
                        }
                        active_doc.state.undo_stack.push(
                            EditOp::SplitLine { first_line: offset+active_doc.state.curs_y, split_index, second_line: offset+active_doc.state.curs_y+1, applied: false }
                        );
                       
                        active_doc.split_lines(
                            offset+active_doc.state.curs_y,
                            split_index);
                        if active_doc.state.curs_y < active_doc.state.window_height-2{    
                            move_curs(active_doc, CursorDirection::Down);
                        }
                        active_doc.state.curs_x=0;
                        
                    }
                }
            }

            // Handle Ctrl + Q to quit the application
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                if self.documents[self.active].state.is_dirty {
                        self.show_popup(String::from("Save before quitting?"), PopupTypes::SaveOnClosePopup);
                    }   
                    else{
                        self.exit().unwrap_or_else(
                            |e| self.show_popup(e.to_string(), PopupTypes::ErrorPopup)
                        );
                    }
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
                    self.change(index - 1);
                }
            }

            // Handle Alt + Right Arrow to switch tabs right
            (KeyCode::Right, KeyModifiers::ALT) => {
                if self.active < self.documents.len() - 1 {
                    self.active += 1;
                }
            }

            (KeyCode::Right | KeyCode::Left | KeyCode::Up | KeyCode::Down, KeyModifiers::SHIFT | KeyModifiers::NONE) => {
                match  self.focus {
                    Windows::Command =>{
                        match key_event.code {
                            KeyCode::Right => {
                                if self.curs_x+1 <= self.input_buffer.len() {
                                    self.curs_x+=1
                                }
                            },
                            KeyCode::Left => {
                                if self.curs_x-1 > 0 {
                                    self.curs_x-=1
                                }
                            },

                            _ => {}
                        }
                    }
                    Windows::Editor =>{
                        
                        let active_doc = &mut self.documents[self.active];
                        match (key_event.code, key_event.modifiers)  {
                            (KeyCode::Right, KeyModifiers::NONE) => {
                                active_doc.state.selection = None;
                                move_curs(active_doc, CursorDirection::Right);
                            },
                            (KeyCode::Left, KeyModifiers::NONE) => {
                                active_doc.state.selection = None;
                                move_curs(active_doc, CursorDirection::Left);
                            },
                            (KeyCode::Down, KeyModifiers::NONE) => {
                                active_doc.state.selection = None;
                                move_curs(active_doc, CursorDirection::Down);
                            },
                            (KeyCode::Up, KeyModifiers::NONE) => {
                                active_doc.state.selection = None;
                                move_curs(active_doc, CursorDirection::Up);
                            },
                            (KeyCode::Right, KeyModifiers::SHIFT) => 
                            {
                                
                                active_doc.state.start_selection(active_doc.state.curs_y, active_doc.state.curs_x);
                                move_curs(active_doc, CursorDirection::Right);
                                active_doc.state.update_selection_end(active_doc.state.curs_y, active_doc.state.curs_x);
                            },
                            (KeyCode::Left, KeyModifiers::SHIFT) => {
                                active_doc.state.start_selection(active_doc.state.curs_y, active_doc.state.curs_x);
                                move_curs(active_doc, CursorDirection::Left);
                                active_doc.state.update_selection_end(active_doc.state.curs_y, active_doc.state.curs_x);
                            },
                            (KeyCode::Down, KeyModifiers::SHIFT) => {
                                active_doc.state.start_selection(active_doc.state.curs_y, active_doc.state.curs_x);
                                move_curs(active_doc, CursorDirection::Down);
                                active_doc.state.update_selection_end(active_doc.state.curs_y, active_doc.state.curs_x);
                            },
                            (KeyCode::Up, KeyModifiers::SHIFT)  => {
                                active_doc.state.start_selection(active_doc.state.curs_y, active_doc.state.curs_x);
                                move_curs(active_doc, CursorDirection::Up);
                                active_doc.state.update_selection_end(active_doc.state.curs_y, active_doc.state.curs_x);
                            },
                            _ => {}
                        }
                    }
                }
                
            }

            // Handle Backspace to remove the last character from input buffer
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                match self.focus {
                    Windows::Command => {
                        if self.input_buffer.is_empty() {
                            return;
                        }
                        self.input_buffer.remove(self.curs_x.saturating_sub(1));
                        self.curs_x = self.curs_x.saturating_sub(1);
                    }
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                        let offset = active_doc.state.scroll_offset;
                        // Check if the cursor is at the beginning of the line.

                        match  active_doc.state.selection {

                            Some((start, stop)) => {
                                //in order for something to be selected it mys texist 
                               
                                let deleted = active_doc.delete_selection(start, stop);
                                active_doc.state.undo_stack.push(
                                    EditOp::DeleteSelection { start:start, stop:stop, selection: deleted, applied: false }
                                );

                                active_doc.adjust_cursor(start.min(stop).0, start.min(stop).1, false);
                                //If all selection is a complete line
                                
                            }
                            None =>{
                                if active_doc.state.curs_x == 0 {
                                    if active_doc.state.curs_y ==0 && active_doc.state.scroll_offset == 0{
                                        return;
                                    }
                                    if active_doc.state.curs_y == 0 && active_doc.state.scroll_offset > 0{
                                        //its on the first visible line && there are lines above
                                        //scrolling is needed 
                                        active_doc.state.scroll_offset-=1;
                                    }
                                    active_doc.state.undo_stack.push(EditOp::MergeLines { 
                                        merged_line: (offset + active_doc.state.curs_y).saturating_sub(1), 
                                        merge_point: active_doc.content[(offset + active_doc.state.curs_y).saturating_sub(1)].len(), 
                                        applied: false });
                                       
                                    active_doc.merge_lines(
                                        (offset + active_doc.state.curs_y).saturating_sub(1), 
                                        offset + active_doc.state.curs_y  );
                                    
                                } else {
                                    // Delete the character on the left of the cursor.
                                    
                                    
                                   let ch = active_doc.content[offset+active_doc.state.curs_y].chars().nth(active_doc.state.curs_x - 1).unwrap();
                                    active_doc.state.undo_stack.push(EditOp::DeleteChar {
                                        line: offset + active_doc.state.curs_y,
                                        col: active_doc.state.curs_x-1,
                                        ch: ch,
                                        applied: false,
                                    });
                    
                                    active_doc.delete_char(
                                        offset + active_doc.state.curs_y, 
                                        active_doc.state.curs_x);
                                    
                                }
        
                            }
                            
                        }
                    }
                }
            }
            (KeyCode::Char('c') | KeyCode::Char('v'), KeyModifiers::CONTROL) =>{
                let active_doc = &mut self.documents[self.active];
                match key_event.code {
                    KeyCode::Char('c') =>{
                        match active_doc.state.selection {
                            Some(val) =>{
                                let(line1, col1, line2, col2) = {
                                    let (y1, x1) = val.0;
                                    let (y2, x2) = val.1;

                                    if y1 < y2 {
                                        (y1, x1, y2, x2)
                                    }
                                    else{
                                        (y2, x2, y1, x1)
                                    }

                                };
                                
                                let mut copy_content: Vec<String> = Vec::new();
                                let mut copy_buffer = String::new();
                                if line1 == line2 {
                                    copy_buffer = active_doc.content[line1].as_str()[col2.min(col1)..col1.max(col2)].to_string();
                                }else{
                                    copy_content.push(active_doc.content[line1].as_str()[col1..].to_string());
                                    if line2 - line1 != 0 {
                                        for line in &active_doc.content[line1+1..line2]{
                                            copy_content.push(line.clone());
                                        }
                                    }
                                    copy_content.push(active_doc.content[line2].as_str()[..col2].to_string());
                                    copy_buffer = copy_content.join("\n");
                                        
                                }
                                self.clipboard = ClipboardContext::new().unwrap();
                                self.clipboard.set_contents(copy_buffer.to_owned()).unwrap();
                                fs::write("log.txt", format!("{},{} {},{} \n {}", line1,col1,line2,col2, self.clipboard.get_contents().unwrap())).ok();
                            },
                            None => {},
                        }
                    }
                    KeyCode::Char('v') =>{
                        let active_doc = &mut self.documents[self.active];
                        let offset = active_doc.state.scroll_offset;

                        // If there is a selection, delete it first
                        let (insert_line, insert_col) = if let Some(sel) = active_doc.state.selection.take() {
                            // normalize selection bounds
                            let ((y1, x1), (y2, x2)) = (sel.0, sel.1);
                            let (start, stop) = if (y1, x1) <= (y2, x2) {
                                ((y1, x1), (y2, x2))
                            } else {
                                ((y2, x2), (y1, x1))
                            };
                            // delete the selected range
                            let deleted = active_doc.delete_selection(start, stop);
                            active_doc.state.undo_stack.push(
                                EditOp::DeleteSelection {
                                    start,
                                    stop,
                                    selection: deleted,
                                    applied: false,
                                }
                            );
                            // move cursor to start of deleted region
                            active_doc.adjust_cursor(start.0, start.1, false);
                            // start paste at that position
                            start
                        } else {
                            // no selection: paste at current cursor
                            (offset + active_doc.state.curs_y, active_doc.state.curs_x)
                        };

                        // Now perform the paste
                        if let Some(line) = active_doc.content.get_mut(insert_line) {
                            let clipboard_text = self.clipboard.get_contents().unwrap_or_default();
                            let lines: Vec<&str> = clipboard_text.split('\n').collect();
                            // compute stop position
                            let stop = if lines.len() > 1 {
                                (insert_line + lines.len() - 1, lines.last().unwrap().len())
                            } else {
                                (insert_line, insert_col + lines[0].len())
                            };
                            // insert the clipboard text
                            active_doc.insert_selection((insert_line, insert_col), clipboard_text.clone());
                            // record for undo
                            active_doc.state.undo_stack.push(
                                EditOp::InsetSelection {
                                    applied: false,
                                    start: (insert_line, insert_col),
                                    stop,
                                    selection: clipboard_text,
                                }
                            );
                            // move cursor to end of inserted text
                            active_doc.adjust_cursor(stop.0, stop.1, false);
                        }
                        
                    }
                    _=>{}
                }
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => 
            {
                
                match self.documents[self.active].save_file() {
                    Ok(_ ) => {
                        self.theme = Theme::load();
                    }
                    Err(e) =>{
                        self.show_popup(e.to_string(), PopupTypes::ErrorPopup)
                    }
                }
                
                
            },
            (KeyCode::Char('z') | KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                let active_doc = &mut self.documents[self.active];
                match key_event.code {
                    KeyCode::Char('z') => {
                        active_doc.undo();
                    }
                    KeyCode::Char('y') => {
                        active_doc.redo();
                    }
                    _ =>{}
                }
            }
            (KeyCode::Char('n') | KeyCode::Char('m'), KeyModifiers::ALT) => {
                let active_doc = &mut self.documents[self.active];
                if active_doc.state.find_active {
                    match key_event.code {
                        KeyCode::Char('n') => {
                            if !active_doc.state.highlights.is_empty() {
                                let len = active_doc.state.highlights.len();
                                active_doc.state.current_match = (active_doc.state.current_match + 1) % len;
                                let h = active_doc.state.highlights[active_doc.state.current_match];
                                active_doc.adjust_cursor(h.0, h.2, false);
                            }
                        }
                        KeyCode::Char('m') => {
                            if !active_doc.state.highlights.is_empty() {
                                let len = active_doc.state.highlights.len();
                                if active_doc.state.current_match == 0 {
                                    active_doc.state.current_match = len - 1;
                                } else {
                                    active_doc.state.current_match -= 1;
                                }
                                let h = active_doc.state.highlights[active_doc.state.current_match];
                                active_doc.adjust_cursor(h.0, h.2, false);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Handle regular character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                match self.focus {
                    Windows::Command => match key_event.modifiers {
                        KeyModifiers::SHIFT => {
                            self.input_buffer.insert(self.curs_x, c.to_ascii_uppercase());
                            self.curs_x+=1;
                        },
                        KeyModifiers::NONE => {
                            self.input_buffer.insert(self.curs_x, c);
                            self.curs_x+=1;
                        }
                        _ =>{}
                    },
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                        let offset = active_doc.state.scroll_offset;
                        match active_doc.state.selection{

                            Some(s)=>{
                                {
                                    // Normalize selection bounds
                                    let (y1, x1) = s.0;
                                    let (y2, x2) = s.1;
                                    let (start, stop) = if (y1, x1) <= (y2, x2) {
                                        ((y1, x1), (y2, x2))
                                    } else {
                                        ((y2, x2), (y1, x1))
                                    };

                                    // Delete the selection and record it for undo
                                    let deleted = active_doc.delete_selection(start, stop);
                                    active_doc.state.undo_stack.push(EditOp::DeleteSelection {
                                        start,
                                        stop,
                                        selection: deleted,
                                        applied: false,
                                    });

                                    // Move cursor to the start of the deleted region and clear selection
                                    active_doc.adjust_cursor(start.0, start.1, false);
                                    active_doc.state.selection = None;
                                }
                            }
                            None =>{
                            
                            }
                        }
                        active_doc.state.undo_stack.push(EditOp::InsertChar { 
                            line: offset + active_doc.state.curs_y,
                            col: active_doc.state.curs_x,
                            ch: if key_event.modifiers.contains(KeyModifiers::SHIFT) {c.to_ascii_uppercase()} else {c},
                            applied: false});

                        active_doc.insert_char(
                            offset + active_doc.state.curs_y,
                            active_doc.state.curs_x,
                            if key_event.modifiers.contains(KeyModifiers::SHIFT) {c.to_ascii_uppercase()} else {c}
                            );

                        
                        
                        
                        // Ensure the content vector has enough lines
                        
                    }
                }
            }
            _ => {}
        }
    }

    
    pub fn open(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let file_paths : Vec<&str>= file_path.split(" ").collect();
        for file in file_paths{
            let doc = Document::new(&String::from(file))?;
            self.documents.push(doc);
        }
        self.active = self.documents.len() - 1;
        Ok(())
    }
    pub fn list_docs(&mut self) -> Vec<String> {
        self.documents
            .iter()
            .map(|doc| doc.file_path.clone())
            .collect()
    }

    pub fn close(&mut self) {
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
    pub fn change(&mut self, index: usize) {
        self.active = index;
    }
    pub fn command_parse(&mut self, cmd: &str) -> Result<Option<Operations>, Box<dyn Error>> {
        let mut parts = cmd.trim().split_whitespace();
        let command = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
       
        if command.trim() == "o" {
            Ok(Some(Operations::Open(String::from(args.join(" ")))))
        } else if command.trim() == "theme" {
            Ok(Some(Operations::Open(String::from("/home/petru/.config/tpad/theme.toml"))))
        } 
        else if command.trim() == "set"{
            self.show_popup(String::new(), PopupTypes::ThemeSelectPopup);
            Ok(Some(Operations::None))
        }
        else if command.trim().starts_with('/'){
            Ok(Some(Operations::Find(String::from(command.trim().trim_start_matches("/")))))
        } else if command.trim() == "count" {
            if let Some(word) = args.get(0) {
                Ok(Some(Operations::WordCount((*word).to_string())))
            } else {
                Err("No word provided for count command".into())
            }
        } else if command.trim() == "list" {
            Ok(Some(Operations::List))
        }else if command.trim() == "clundo" {
            self.documents[self.active].state.undo_stack.stack.clear();
            self.documents[self.active].state.undo_stack.cursor = 0;
            Ok(Some(Operations::None))
        } else if command.trim() == "q" {
            Ok(Some(Operations::Close))
        }else if command.trim() == "wq" {
            self.documents[self.active].save_file()?;
            Ok(Some(Operations::Close))
        } else if command.trim() == "w" {
            self.documents[self.active].save_file()?;
            Ok(Some(Operations::None))
        } 
        else if command.trim() == "cl" {
            Ok(Some(Operations::Exit))
           
        } else {
            Err("Invalid command ".into())
        }
    }

    pub fn command_run(&mut self, cmd: &str) {
        match self.command_parse(cmd) {
            Ok(cmd) => match cmd {
                Some(Operations::Open(file_path)) => {
                    let result = self.open(&file_path);
                    if let Err(e) = result {
                        self.show_popup(e.to_string(), PopupTypes::ErrorPopup); // Fixed method name
                    }
                }
                Some(Operations::Find(word)) => {
                    let doc = &mut self.documents[self.active];
                    let matches = doc.find(&word);
                    doc.highlight(matches);
                    self.focus = Windows::Editor;
                }
                Some(Operations::WordCount(word)) => {
                    let msg = self.documents[self.active].word_count(&word);
                    println!("{}", msg[0]);
                }
                Some(Operations::Exit) => {
                    if self.documents[self.active].state.is_dirty {
                        self.show_popup(String::from("Save before quitting?"), PopupTypes::SaveOnClosePopup);
                    }   
                    else{
                        self.exit().unwrap_or_else(
                            |e| self.show_popup(e.to_string(), PopupTypes::ErrorPopup)
                        );
                    }
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
                Some(Operations::None) => {},
                None => {},
            },
            Err(e) => {
                self.show_popup(e.to_string(), PopupTypes::ErrorPopup); // Fixed method name
            }
        }
    }
    

    pub fn exit(&mut self) -> Result<(), Box<dyn Error>> {
        session::save_session(self)?;
        self.running = false;
        Ok(())
    }
}
pub fn move_curs(active_doc: &mut Document, direction: CursorDirection) {
    // Get the currently active document
    if active_doc.content.len() == 0 {return;}
        match direction {
            CursorDirection::Left => {
                if active_doc.state.curs_x > 0 {
                    active_doc.state.curs_x -= 1;
                }
                else {
                    if active_doc.state.curs_y !=0 {

                        active_doc.state.curs_y = (active_doc.state.scroll_offset+ active_doc.state.curs_y).saturating_sub(1);
                        active_doc.state.curs_x = active_doc.content[active_doc.state.curs_y].len();
                    }
                }
            }
            CursorDirection::Right => {
                if active_doc.content[active_doc.state.scroll_offset + active_doc.state.curs_y].len() + 1
                    > active_doc.state.curs_x + 1
                {
                    active_doc.state.curs_x += 1;
                }
            }
            CursorDirection::Up => {
                if active_doc.state.curs_y > 0 {
                    active_doc.state.curs_y -= 1;
                    if active_doc.state.curs_x
                        > active_doc.content[active_doc.state.scroll_offset + active_doc.state.curs_y].len()
                    {
                        active_doc.state.curs_x =
                            active_doc.content[active_doc.state.scroll_offset + active_doc.state.curs_y].len();
                    }
                } else {
                    active_doc.state.scroll_offset = active_doc.state.scroll_offset.saturating_sub(1);
                    active_doc.state.curs_x = active_doc.content[active_doc.state.curs_y + active_doc.state.scroll_offset]
                        .len()
                        .min(active_doc.state.curs_x);
                }
            }
            CursorDirection::Down => {
                if active_doc.state.curs_y < active_doc.state.window_height - 2
                    && active_doc.state.curs_y + active_doc.state.scroll_offset < active_doc.content.len() - 1
                {
                    active_doc.state.curs_y += 1;
                    if active_doc.state.curs_x
                        > active_doc.content[active_doc.state.scroll_offset + active_doc.state.curs_y].len()
                    {
                        active_doc.state.curs_x =
                            active_doc.content[active_doc.state.scroll_offset + active_doc.state.curs_y].len();
                    }
                } else if active_doc.state.curs_y + active_doc.state.scroll_offset < active_doc.content.len() - 1 {
                    active_doc.state.scroll_offset += 1;

                    active_doc.state.curs_x = active_doc.content[active_doc.state.curs_y + active_doc.state.scroll_offset]
                        .len()
                        .min(active_doc.state.curs_x);
                }
            }
        }
    
}
