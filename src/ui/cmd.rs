use ratatui::{
    layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style, Stylize}, text::{Line, Span, Text}, widgets::{Block, Borders, Paragraph}, Frame
};

use crate::{data_models::*, theme::hex_to_color};

pub fn render_cmd(frame: &mut Frame<'_>, area: Rect, ctx: &RenderContext){
    let cmd_bg = hex_to_color(ctx.theme.command.background.clone());

    let cmd_fg = hex_to_color(ctx.theme.command.foreground.clone());
    let cmd_text = vec![String::from(": "), ctx.input_buffer.clone()].join("");
    let cmd = Paragraph::new(Text::from(cmd_text).style(cmd_fg)).block(Block::default().borders(Borders::ALL).style(cmd_bg));
    frame.render_widget(cmd, area);
    if let Windows::Command= ctx.focus{
        frame.set_cursor_position((area.x + 3 + *ctx.curs_x as u16, area.y+1));
    }
}