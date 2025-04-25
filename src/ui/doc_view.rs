use std::fs;

use color_eyre::owo_colors::{colors::xterm::MatrixPink, OwoColorize};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style, Stylize}, text::{Line, Span, Text}, widgets::{Block, Borders, Paragraph}, Frame
};

use crate::{data_models::*, theme::hex_to_color};

pub fn render_doc_view(frame: &mut Frame<'_>, area: Rect, ctx: &RenderContext) {
    let selected_doc = &ctx.documents[*ctx.active];
    let highl = hex_to_color(ctx.theme.editor.highlights.clone());
    let fg_color = hex_to_color(ctx.theme.editor.foreground.clone());
    let bg_color = hex_to_color(ctx.theme.editor.background.clone());
    //fs::write("log.txt", format!("{:?} {:?} {:?}", highl, fg_color, bg_color)).unwrap();
    let mut doc_view = selected_doc
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
            

                for (i, ch) in line.char_indices() {
                    let style = if highlights.iter().any(|&(_, start, end)| i >= start && i < end) {
                        Style::default().fg(highl)
                    } else {
                        Style::default().fg(fg_color)
                    };
                    spans.push(Span::styled(ch.to_string(), style));
                }
                spans
            })
        })
        .collect::<Vec<Line>>();
        
        let offst = selected_doc.state.scroll_offset;
        let selection= selected_doc.state.selection;
        match selection {
            
            Some(val) => {
                let (start_y, start_x, stop_y, stop_x) = {
                    let (y1, x1) = val.0;
                    let (y2, x2) = val.1;
                    if y1 <= y2 {
                        
                        (y1, x1, y2, x2)
                    } else {
                        (y2, x2, y1, x1)
                    }
                };

                for (y, line )in doc_view.iter_mut().enumerate() {
                    for (x,span )in line.iter_mut().enumerate() {

                        if y == start_y && y == stop_y {
                            // Single line selection.
                            
                            if x >= start_x.min(stop_x) && x < stop_x.max(start_x)  {
                                *span = Span::styled(span.content.clone(), span.style.bg(highl));
                            }
                        } else if y == start_y {
                            // First line of a multi-line selection.
                            if x >= start_x {
                                *span = Span::styled(span.content.clone(), span.style.bg(highl));
                            }
                        } else if y > start_y && y < stop_y {
                            // Entire middle line is selected.
                            *span = Span::styled(span.content.clone(), span.style.bg(highl));
                        } else if y == stop_y {
                            // Last line of a multi-line selection.
                            if x < stop_x {
                                *span = Span::styled(span.content.clone(), span.style.bg(highl));
                            }
                        }
                        
                    
                            
                           
                            
                        
                        
                       
                            
                        
                    }
                }
            }
            None => {}
        }
        
    let doc_view_slice = &doc_view[selected_doc.state.scroll_offset..];
    let doc_view_paragraph = Paragraph::new(Text::from_iter(doc_view_slice.iter().cloned())).style(bg_color)
        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));

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
    let line_numbers_widget =
        Paragraph::new(line_numbers).style(fg_color).block(Block::default().borders(Borders::LEFT | Borders::TOP).style(bg_color));
    frame.render_widget(line_numbers_widget, chunks[0]);
    frame.render_widget(doc_view_paragraph, chunks[1]);

    let curs_x = chunks[1].x + 1 + selected_doc.state.curs_x as u16;
    let curs_y = chunks[1].y + 1 + selected_doc.state.curs_y as u16;

    if let Windows::Editor = ctx.focus {
        frame.set_cursor_position((curs_x, curs_y));
    }
}