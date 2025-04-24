use std::error::Error;

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::Rect,
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use crate::data_models::*;

pub fn render_error_popup(frame: &mut Frame<'_>, area: Rect, error: &String) {
    let error_widget = Paragraph::new(Text::from(error.clone()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .centered();

    frame.render_widget(error_widget, area);
}
pub fn parse_error(error: Box<dyn Error>) -> String {
    error.to_string()
}

impl App {
    pub fn throw_error(&mut self, error: impl ToString) {
        self.render_error = true; // Set the flag to true
        self.error_msg = error.to_string(); // Update the error message
    }
    pub fn clear_error(&mut self) {
        self.render_error = false;
        self.error_msg.clear();
    }
}
