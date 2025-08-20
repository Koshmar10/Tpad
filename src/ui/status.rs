use std::path::{self, PathBuf};

use ratatui::{Frame, layout::Rect, text::Line, widgets::Paragraph};

use crate::{data_models::*, theme::hex_to_color};

pub fn render_status_bar(frame: &mut Frame<'_>, area: Rect, ctx: &RenderContext) {
    let fg = hex_to_color(ctx.theme.status.foreground.clone());
    let cursor_info = (
        ctx.documents[*ctx.active].state.curs_x,
        ctx.documents[*ctx.active].state.curs_y + ctx.documents[*ctx.active].state.scroll_offset,
    );
    let permissions = &ctx.documents[*ctx.active].permissions;
    let saved: &str = if !ctx.documents[*ctx.active].state.is_dirty {
        "Saved"
    } else {
        "Unsaved"
    };
    let status_text = format!(
        "Tpad | Line: {} Col: {} |{}| Size: {} | tabs: {} | path: {}",
        cursor_info.1,
        cursor_info.0,
        permissions,
        ctx.documents[*ctx.active].size,
        ctx.documents.len(),
        strip_path(ctx.default_dir.clone())
    );
    let status_bar = Paragraph::new(Line::from(status_text).left_aligned().style(fg));
    frame.render_widget(status_bar, area);
}

fn strip_path(path: PathBuf) -> String {
    let home_dir = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    
    if let Some(home) = home_dir {
        if let Ok(stripped_path) = path.strip_prefix(&home) {
            return format!("~/{}", stripped_path.to_string_lossy());
        }
    }
    
    path.to_string_lossy().to_string()
}