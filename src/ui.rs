use std::io;
use color_eyre::owo_colors::OwoColorize;
use crossterm::{event::{self, Event, KeyEventKind}, style::Stylize};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::data_models::*;
use crate::{App, Windows};

impl App {
    pub fn render_status_bar(&mut self, frame: &mut Frame<'_>, area: Rect) {
        
        let cursor_info = (
            self.documents[self.active].state.curs_x,
            self.documents[self.active].state.curs_y + self.documents[self.active].state.scroll_offset,
        );
        let permissions = &self.documents[self.active].permissions;
        let saved: &str = if !self.documents[self.active].state.is_dirty {
            "Saved"
        } else {
            "Unsaved"
        };
        let status_text = format!(
            "Tpad | Line: {} Col: {} | {} | {}| Size: {} | open tabs: {} | op cusor: {}",
            cursor_info.1,
            cursor_info.0,
            saved,
            permissions,
            self.documents[self.active].size,
            self.documents.len(),
            self.documents[self.active].state.undo_stack.cursor
        );
        let status_bar = Paragraph::new(Line::from(status_text).left_aligned());
        frame.render_widget(status_bar, area);
    }

    pub fn render_tab_bar(&mut self, f: &mut Frame<'_>, area: Rect) {
        let pad = 6;
        // Create a vector with tabs, each as a tuple containing the global index and tab details.
        let opend_tabs: Vec<(usize, (String, usize))> = self
            .documents
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let file_name = get_file_name(d.file_path.clone());
                (i, (file_name.clone(), file_name.len() + pad))
            })
            .collect();

        // Partition tabs into packs based on the available area width.
        let mut tab_packs: Vec<Vec<(usize, (String, usize))>> = vec![vec![]];
        let mut current_pack_index: usize = 0;
        let mut pack_size: usize = 0;
        for tab in opend_tabs.into_iter() {
            if pack_size + tab.1.1 <= area.width as usize {
                tab_packs[current_pack_index].push(tab.clone());
                pack_size += tab.1.1;
            } else {
                current_pack_index += 1;
                tab_packs.push(vec![]);
                tab_packs[current_pack_index].push(tab.clone());
                pack_size = tab.1.1;
            }
        }

        // Calculate which pack the active tab belongs to and its relative index.
        let mut active_pack_index = None;
        let mut active_relative_index = None;
        for (pack_idx, pack) in tab_packs.iter().enumerate() {
            for (rel_idx, (global_idx, _)) in pack.iter().enumerate() {
                if *global_idx == self.active {
                    active_pack_index = Some(pack_idx);
                    active_relative_index = Some(rel_idx);
                    break;
                }
            }
            if active_pack_index.is_some() {
                break;
            }
        }

        // Retrieve the tabs for the current pack.
        let mut tabs_to_render: Vec<(usize, (String, usize))> = tab_packs
            .get(active_pack_index.unwrap_or(0))
            .cloned()
            .unwrap_or_default();

        // If this is not the last pack then add an extra overflow tab that fills the remaining space.
        if active_pack_index.unwrap_or(0) < tab_packs.len() - 1 {
            // Sum the widths used in this pack.
            let used_width: usize = tabs_to_render.iter().map(|tab| tab.1.1).sum();
            let mut remaining = if used_width < area.width as usize {
                area.width as usize - used_width
            } else {
                0
            };
            remaining = remaining+2;
                // Use at least a few columns for the marker.
                tabs_to_render.push((usize::MAX, ("...".to_string(), remaining)));
            
        }

        let constraints: Vec<Constraint> = tabs_to_render
            .iter()
            .map(|tab| Constraint::Length(tab.1.1 as u16))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        // Render each visible tab.
        for (i, (&chunk, tab)) in chunks.iter().zip(tabs_to_render.iter()).enumerate() {
            // If tab's global index is usize::MAX, it's our overflow marker.
            let (name, _) = if tab.0 == usize::MAX {
                (tab.1.0.clone(), 0)
            } else {
                (tab.1.0.clone(), tab.1.1)
            };
            let style = if active_relative_index.is_some() && i == active_relative_index.unwrap() {
                Color::Yellow
            } else {
                Color::White
            };
            let tab_widget = Paragraph::new(Text::from(name))
                .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT))
                .centered()
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(style));
            f.render_widget(tab_widget, chunk);
        }
    }

    pub fn render_doc_view(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let selected_doc = &mut self.documents[self.active];
        let doc_view_height = area.height as usize;
        if selected_doc.state.window_height == 0 {
            selected_doc.state.window_height = doc_view_height;
        }

        let doc_view = selected_doc
            .content
            .iter()
            .enumerate()
            .map(|(index, line)| {
                Line::from({
                    // Collect all highlights for the current line and sort them by start index.
                    let mut highlights: Vec<(usize, usize, usize)> = selected_doc
                        .state
                        .highlights
                        .iter()
                        .filter(|(line_index, _, _)| *line_index == index)
                        .cloned()
                        .collect();
                    highlights.sort_by_key(|&(_, start, _)| start);

                    let mut spans = Vec::new();
                    let mut last_end = 0;

                    for &(_, start, end) in highlights.iter() {
                        // Add non-highlighted part before the match.
                        if start > last_end {
                            spans.push(Span::raw(&line[last_end..start]));
                        }
                        // Add highlighted segment.
                        spans.push(Span::styled(
                            &line[start..end],
                            Style::default().fg(Color::Yellow),
                        ));
                        last_end = end;
                    }
                    // Add remaining part of the line.
                    if last_end < line.len() {
                        spans.push(Span::raw(&line[last_end..]));
                    }
                    spans
                })
            })
            .collect::<Vec<Line>>();

        let doc_view_slice = &doc_view[selected_doc.state.scroll_offset..];
        let doc_view_paragraph = Paragraph::new(Text::from_iter(doc_view_slice.iter().cloned()))
            .block(
                Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT),
            );

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(4), Constraint::Min(3)])
            .split(area);

        let lines: Vec<Line> = (selected_doc.state.scroll_offset..selected_doc.content.len())
            .map(|num| {
                // For each line, draw a line number centered.
                Line::from(num.to_string()).centered()
            })
            .collect();
        let line_numbers = Text::from(lines);
        let line_numbers_widget = Paragraph::new(line_numbers)
            .block(Block::default().borders(Borders::LEFT | Borders::TOP));
        frame.render_widget(line_numbers_widget, chunks[0]);
        frame.render_widget(doc_view_paragraph, chunks[1]);

        let curs_x = chunks[1].x + 1 + selected_doc.state.curs_x as u16;
        let curs_y = chunks[1].y + 1 + selected_doc.state.curs_y as u16;

        if let Windows::Editor = self.focus {
            frame.set_cursor_position((curs_x, curs_y));
        }
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
}

fn get_file_name(path: String) -> String {
    let mut name = String::new();
    let path: String = path.chars().rev().collect();
    for c in path.chars() {
        if c == '/' {
            break;
        }
        name.push(c);
    }
    name.chars().rev().collect()
}