use color_eyre::owo_colors::OwoColorize;
use copypasta::{ClipboardContext, ClipboardProvider};
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
use crate::ui::render;

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
            clipboard: ClipboardContext::new().unwrap(),
            theme:Theme::load(),
            window_height: 0,
            documents,
            active: 0,
            render_error,
            error_msg,
            running: true,
            input_buffer: String::new(),
            show_popup: false,
            exit_requested: false,
            popup_message: String::new(),
            focus: Windows::Editor,
            curs_x: 0,
    
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            let ctx = RenderContext {
                theme: &self.theme,
                documents: &self.documents,
                input_buffer: &self.input_buffer,
                show_popup: &self.show_popup,
                active: &self.active,
                running: &self.running,
                render_error: &self.render_error,
                error_msg: &self.error_msg,
                exit_requested: &self.exit_requested,
                popup_message: &self.popup_message,
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
            if self.exit_requested && !self.show_popup {
                self.exit().unwrap();
            }
            self.handle_events()?;
        }
        Ok(())
    }
    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.clear_error();
                self.handle_key_event(key_event);
            }
            _ => {}
        }
        Ok(())
    }
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.show_popup {
            match key_event.code {
                KeyCode::Char('y') => {
                    self.documents[self.active]
                        .save_file()
                        .unwrap_or_else(|e| self.throw_error(e));
                    self.show_popup = false;
                    self.exit_requested = false;
                    self.exit().unwrap_or_else(|e| self.throw_error(e));
                }
                KeyCode::Char('n') => {
                    self.show_popup = false;
                    self.exit_requested = false;
                    self.exit().unwrap_or_else(|e| self.throw_error(e));
                }
                _ => return,
            }
            return;
        }
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
                        let split_index = active_doc.state.curs_x;
                        //if a split is initialized and the cursor is on the last line of the doc
                        //a new line is created so that he split does not create panic
                        if offset + active_doc.state.curs_y >= active_doc.content.len(){
                            active_doc.content.push(String::new());
                        }
                        if active_doc.state.curs_y == active_doc.state.window_height {
                            active_doc.state.scroll_offset+=1;
                        }
                        active_doc.state.undo_stack.push(
                            EditOp::SplitLine { first_line: offset+active_doc.state.curs_y, split_index, second_line: offset+active_doc.state.curs_y+1, applied: false }
                        );
                       
                        active_doc.split_lines(
                            offset+active_doc.state.curs_y,
                            split_index);
                        
                    }
                }
            }

            // Handle Ctrl + Q to quit the application
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.try_exit();
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
                                merged_line: offset + active_doc.state.curs_y.saturating_sub(1), 
                                merge_point: active_doc.content[offset + active_doc.state.curs_y.saturating_sub(1)].len(), 
                                applied: false });
                               
                            active_doc.merge_lines(
                                offset + active_doc.state.curs_y .saturating_sub(1), 
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
                        let offset =  active_doc.state.scroll_offset;
                        let insert_line = active_doc.state.curs_y+offset;
                        match active_doc.content.get_mut(insert_line){
                            Some(line) =>{
                                line.insert_str(active_doc.state.curs_x, &self.clipboard.get_contents().unwrap_or(String::new()));
                                active_doc.content ={
                                    let mut updated:Vec<String>= Vec::new();
                                    for line in &active_doc.content{
                                        for new_line in  line.split("\n"){
                                            updated.push(new_line.to_string());
                                        }
                                    }
                                    updated

                                } 
                            },
                            None => return,
                        };
                    }
                    _=>{}
                }
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.documents[self.active]
                .save_file()
                .unwrap_or_else(|e| self.throw_error(e)),
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
        else if command.trim().starts_with('/'){
            Ok(Some(Operations::Find(String::from(command.trim().trim_start_matches("/")))))
        } else if command.trim() == "count" {
            Ok(Some(Operations::WordCount(String::from(args[0]))))
        } else if command.trim() == "list" {
            Ok(Some(Operations::List))
        }else if command.trim() == "clundo" {
            self.documents[self.active].state.undo_stack.stack.clear();
            self.documents[self.active].state.undo_stack.cursor = 0;
            Ok(Some(Operations::None))
        } else if command.trim() == "q" {
            Ok(Some(Operations::Exit))
        }else if command.trim() == "wq" {
            self.documents[self.active].save_file()?;
            Ok(Some(Operations::Exit))
        } else if command.trim() == "w" {
            self.documents[self.active].save_file()?;
            Ok(Some(Operations::None))
        } 
        else if command.trim() == "cl" {
            Ok(Some(Operations::Close))
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
                        self.throw_error(e); // Fixed method name
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
                    self.try_exit();
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
                self.throw_error(e); // Fixed method name
            }
        }
    }
    pub fn try_exit(&mut self) {
        let doc = &self.documents[self.active];
        if doc.state.is_dirty {
            self.show_popup = true;
            self.exit_requested = true;
            self.popup_message = String::from("Save befor exiting?(y/n)");
        } else {
            self.exit().unwrap();
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
        match direction {
            CursorDirection::Left => {
                if active_doc.state.curs_x > 0 {
                    active_doc.state.curs_x -= 1;
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
