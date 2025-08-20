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

// Add this helper to render the empty-state help screen
fn render_empty_state(f: &mut Frame<'_>, area: Rect, ctx: &RenderContext) {
    let fg = hex_to_color(ctx.theme.editor.foreground.clone());
    let bg = hex_to_color(ctx.theme.editor.background.clone());

    // Pretty-print default dir with ~
    let dir_display = {
        use std::path::PathBuf;
        let p: PathBuf = ctx.default_dir.clone();
        let home_dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        if let Some(home) = home_dir {
            if let Ok(stripped) = p.strip_prefix(&home) {
                format!("~/{}", stripped.to_string_lossy())
            } else {
                p.to_string_lossy().to_string()
            }
        } else {
            p.to_string_lossy().to_string()
        }
    };

    let lines = [
        "Welcome to tpad",
        "",
        "No file is open. Try these commands:",
        "",
        "o <file>      - open file (bare name saved under default dir)",
        "setdir <path> - set default directory for new files",
        "theme         - open theme file",
        "set           - choose a theme",
        "/<pattern>    - search for a pattern",
        "count <word>  - word count",
        "list          - list commands",
        "cl            - exit editor",
        "",
        "Tip: Press ':' to enter Command mode",
        "",
        &format!("Default directory: {}", dir_display),
    ];

    let text = Text::from(lines.join("\n"));
    let widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" tpad ").style(bg))
        .style(fg);
    f.render_widget(widget, area);
}

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

    // If there are no documents, render a help/empty state instead of indexing documents.
    if ctx.documents.is_empty() {
        render_empty_state(f, chunks[2], ctx);
        super::cmd::render_cmd(f, chunks[3], ctx);
        super::popup::render_popup(f, ctx);

        return LayoutSnapshot {
            status_area: chunks[0],
            tab_area: chunks[1],
            editor_area: chunks[2],
            command_area: chunks[3],
        };
    }

    render_status_bar(f, chunks[0], ctx);
    render_tab_bar(f, chunks[1], ctx);
    render_doc_view(f, chunks[2], ctx);
    render_cmd(f, chunks[3], ctx);
    render_popup(f, ctx);

    LayoutSnapshot {
        status_area: chunks[0],
        tab_area: chunks[1],
        editor_area: chunks[2],
        command_area: chunks[3],
    }
}
