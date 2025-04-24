use crate::data_models::RenderContext;
use crate::{
    LayoutSnapshot, tpad_error,
    ui::{doc_view::render_doc_view, status::render_status_bar, tab::render_tab_bar},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_ui(f: &mut Frame<'_>, ctx: &RenderContext) -> LayoutSnapshot {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2), // Tabs
            Constraint::Min(1),    // Editor
            Constraint::Length(3), // Command
        ])
        .spacing(0)
        .split(area);

    // Render the tab bar
    render_status_bar(f, chunks[0], ctx);

    render_tab_bar(f, chunks[1], ctx);

    // Render the document view
    render_doc_view(f, chunks[2], ctx);

    // Render the command line
    let cmd_text = vec![String::from(": "), ctx.input_buffer.clone()].join(" ");
    let cmd = Paragraph::new(Text::from(cmd_text)).block(Block::default().borders(Borders::ALL));
    f.render_widget(cmd, chunks[3]);

    // Render the error popup if `render_error` is true
    let popup_width = area.width / 2;
    let popup_height = 5;
    let popup_x = area.x + (area.width - popup_width) / 2;
    let popup_y = area.y + (area.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
    if *ctx.render_error {
        tpad_error::render_error_popup(f, popup_area, &ctx.error_msg);
    }
    if *ctx.show_popup {
        let popup = Paragraph::new(Text::from(vec![
            Line::from(" "),
            Line::from(ctx.popup_message.clone()),
            Line::from(" "),
        ]))
        .centered()
        .block(Block::default().borders(Borders::ALL).title("Popup"));
        f.render_widget(popup, popup_area);
    }

    LayoutSnapshot {
        status_area: chunks[0],
        tab_area: chunks[1],
        editor_area: chunks[2],
        command_area: chunks[3],
    }
}
