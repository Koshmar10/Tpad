use ratatui::{DefaultTerminal, Frame};
use std::error::Error;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io::{self};

use crate::data_models::*;

use crate::session;

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
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            let ctx = RenderContext {
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
                        self.input_buffer.clear();
                    }
                    Windows::Editor => {
                        let active_doc = &mut self.documents[self.active];
                        let offset = active_doc.state.scroll_offset;
                        active_doc.unhighlight();
                        if offset + active_doc.state.curs_y >= active_doc.content.len() {
                            // Add a new empty line if the cursor is beyond the current content
                            active_doc.content.push(String::new());
                            active_doc.state.undo_stack.push(EditOp::SplitLine {
                                first_line: offset + active_doc.state.curs_y,
                                second_line: offset + active_doc.state.curs_y + 1,
                                applied: false,
                            });
                            active_doc.state.curs_y = active_doc.content.len() - offset - 1;
                        } else {
                            let current_line =
                                &mut active_doc.content[offset + active_doc.state.curs_y];
                            if active_doc.state.curs_x >= current_line.len() {
                                // If pressing enter at the end of the line, insert an empty line after
                                active_doc
                                    .content
                                    .insert(offset + active_doc.state.curs_y + 1, String::new());
                                if active_doc.state.curs_y < self.window_height as usize - 2
                                    && offset + active_doc.state.curs_y
                                        < active_doc.content.len() - 1
                                {
                                    active_doc.state.curs_y += 1;
                                } else if active_doc.state.curs_y == self.window_height as usize - 2
                                {
                                    active_doc.state.scroll_offset += 1;
                                }
                            } else {
                                // Split the current line at the cursor position
                                let new_line = current_line.split_off(active_doc.state.curs_x);
                                active_doc
                                    .content
                                    .insert(offset + active_doc.state.curs_y + 1, new_line);
                                if active_doc.state.curs_y < self.window_height as usize - 2
                                    && offset + active_doc.state.curs_y
                                        < active_doc.content.len() - 1
                                {
                                    active_doc.state.curs_y += 1;
                                } else if active_doc.state.curs_y == self.window_height as usize - 2
                                {
                                    active_doc.state.scroll_offset += 1;
                                }
                            }
                            active_doc.state.undo_stack.push(EditOp::SplitLine {
                                first_line: offset + active_doc.state.curs_y.saturating_sub(1),
                                second_line: offset + active_doc.state.curs_y,
                                applied: false,
                            });
                        }
                        active_doc.state.curs_x = 0;

                        self.documents[self.active].update_content();
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

            (KeyCode::Right | KeyCode::Left | KeyCode::Up | KeyCode::Down, KeyModifiers::NONE) => {
                match key_event.code {
                    KeyCode::Right => self.move_curs(CursorDirection::Right),
                    KeyCode::Left => self.move_curs(CursorDirection::Left),
                    KeyCode::Down => self.move_curs(CursorDirection::Down),
                    KeyCode::Up => self.move_curs(CursorDirection::Up),

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
                                let new_idx =
                                    active_doc.state.scroll_offset + active_doc.state.curs_y;
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
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.documents[self.active]
                .save_file()
                .unwrap_or_else(|e| self.throw_error(e)),
            (KeyCode::Char('z') | KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                match key_event.code {
                    KeyCode::Char('z') => {
                        self.documents[self.active].undo();
                    }
                    KeyCode::Char('y') => {
                        self.documents[self.active].redo();
                    }
                    _ => {}
                }
            }
            (KeyCode::Char('n') | KeyCode::Char('m'), KeyModifiers::ALT) => {
                let active_doc = &mut self.documents[self.active];
                if active_doc.state.find_active {
                    let h = active_doc.state.highlights[active_doc.state.current_match];
                    if key_event.code == KeyCode::Char('m')
                        && key_event.modifiers == KeyModifiers::ALT
                    {
                        active_doc.adjust_cursor(h.0, h.2, false);
                        active_doc.state.current_match = active_doc
                            .state
                            .current_match
                            .checked_sub(1)
                            .unwrap_or_else(|| {
                                active_doc.unhighlight();
                                0
                            });
                    } else if key_event.code == KeyCode::Char('n')
                        && key_event.modifiers == KeyModifiers::ALT
                    {
                        if active_doc.state.current_match < active_doc.state.highlights.len() - 1 {
                            active_doc.adjust_cursor(h.0, h.2, false);
                            active_doc.state.current_match += 1;
                        } else {
                            active_doc.unhighlight();
                        }
                    }
                }
            }
            // Handle regular character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                match self.focus {
                    Windows::Command => match key_event.modifiers {
                        KeyModifiers::SHIFT => self.input_buffer.push(c.to_ascii_uppercase()),
                        KeyModifiers::NONE => self.input_buffer.push(c),
                        _ => {}
                    },
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
                                active_doc.state.undo_stack.push(EditOp::InsertChar {
                                    line: offset + active_doc.state.curs_y,
                                    col: active_doc.state.curs_x,
                                    ch: c.to_ascii_uppercase(),
                                    applied: false,
                                });
                                c.to_ascii_uppercase()
                            } else {
                                active_doc.state.undo_stack.push(EditOp::InsertChar {
                                    line: offset + active_doc.state.curs_y,
                                    col: active_doc.state.curs_x,
                                    ch: c,
                                    applied: false,
                                });
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
                    if doc.content[doc.state.scroll_offset + doc.state.curs_y].len() + 1
                        > doc.state.curs_x + 1
                    {
                        doc.state.curs_x += 1;
                    }
                }
                CursorDirection::Up => {
                    if doc.state.curs_y > 0 {
                        doc.state.curs_y -= 1;
                        if doc.state.curs_x
                            > doc.content[doc.state.scroll_offset + doc.state.curs_y].len()
                        {
                            doc.state.curs_x =
                                doc.content[doc.state.scroll_offset + doc.state.curs_y].len();
                        }
                    } else {
                        doc.state.scroll_offset = doc.state.scroll_offset.saturating_sub(1);
                        doc.state.curs_x = doc.content[doc.state.curs_y + doc.state.scroll_offset]
                            .len()
                            .min(doc.state.curs_x);
                    }
                }
                CursorDirection::Down => {
                    if doc.state.curs_y < self.window_height as usize - 2
                        && doc.state.curs_y + doc.state.scroll_offset < doc.content.len() - 1
                    {
                        doc.state.curs_y += 1;
                        if doc.state.curs_x
                            > doc.content[doc.state.scroll_offset + doc.state.curs_y].len()
                        {
                            doc.state.curs_x =
                                doc.content[doc.state.scroll_offset + doc.state.curs_y].len();
                        }
                    } else if doc.state.curs_y + doc.state.scroll_offset < doc.content.len() - 1 {
                        doc.state.scroll_offset += 1;

                        doc.state.curs_x = doc.content[doc.state.curs_y + doc.state.scroll_offset]
                            .len()
                            .min(doc.state.curs_x);
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
        } else if command.trim() == "list" {
            Ok(Some(Operations::List))
        } else if command.trim() == "exit" {
            Ok(Some(Operations::Exit))
        } else if command.trim() == "close" {
            Ok(Some(Operations::Close))
        } else if command.trim() == "change" {
            let index = args[0].parse::<usize>()?;
            Ok(Some(Operations::Change(index)))
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
                None => (),
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
