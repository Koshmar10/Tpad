use crate::data_models::*;
use crate::theme::hex_to_color;

use crate::{
    LayoutSnapshot,
    ui::{doc_view::render_doc_view, status::render_status_bar, tab::render_tab_bar},
};
use crossterm::terminal::WindowSize;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};

use super::cmd::render_cmd;
use super::popup::render_popup;

pub fn render_ui(f: &mut Frame<'_>, ctx: &RenderContext) -> LayoutSnapshot {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Status
            Constraint::Length(2), // Tabs
            Constraint::Min(1),    // Editor
            Constraint::Length(3), // Command
        ])
        .spacing(0)
        .split(area);

    render_status_bar(f, chunks[0], ctx);
    // Render the tab bar

    render_tab_bar(f, chunks[1], ctx);

    // Render the document view
    render_doc_view(f, chunks[2], ctx);

    // Render the command line
    render_cmd(f, chunks[3], ctx);
    // Render the error popup if `render_error` is true
    render_popup(f, ctx);
    

    LayoutSnapshot {
        status_area: chunks[0],
        tab_area: chunks[1],
        editor_area: chunks[2],
        command_area: chunks[3],
    }
}
