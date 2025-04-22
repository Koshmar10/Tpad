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
            "Tpad | Line: {} Col: {} | {} | {}| Size: {} | op_cursor {}",
            cursor_info.1,
            cursor_info.0,
            saved,
            permissions,
            self.documents[self.active].size,
            self.documents[self.active].state.undo_stack.cursor
        );
        let status_bar = Paragraph::new(Line::from(status_text).left_aligned());
        frame.render_widget(status_bar, area);
    }

    pub fn render_tab_bar(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
    ) {
        let tabs: Vec<String> = self.documents.iter().map(
            |doc| get_file_name(doc.file_path.clone())
        ).collect();
        let tab_size = tabs.iter().map(|s| s.len()).max().unwrap_or(0) as u16;
        let tab_bar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                tabs.iter()
                    .map(|_| Constraint::Length(tab_size + 4))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        for (i, (tab, doc)) in tab_bar.iter().zip(tabs).enumerate() {
            let tab_item = Paragraph::new(Text::from(doc))
                .block(
                    Block::default()
                        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT),
                )
                .centered()
                .style(Style::default().fg({
                    if i == self.active {
                        Color::Yellow
                    } else {
                        Color::White
                    }
                }));
            frame.render_widget(tab_item, *tab);
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