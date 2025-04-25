use ratatui::{layout::Rect, text::{Line, Text}, widgets::{Block, Borders, Paragraph}, Frame};

use crate::{tpad_error, RenderContext};



pub fn render_popup(frame: &mut Frame<'_>,  ctx: &RenderContext){
    //asiging popup space
    let area = frame.area();
    let popup_width = area.width / 2;
    let popup_height = 5;
    let popup_x = area.x + (area.width - popup_width) / 2;
    let popup_y = area.y + (area.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    if *ctx.render_error {
        tpad_error::render_error_popup(frame, popup_area, &ctx.error_msg);
    }
    if *ctx.show_popup {
        let popup = Paragraph::new(Text::from(vec![
            Line::from(" "),
            Line::from(ctx.popup_message.clone()),
            Line::from(" "),
        ]))
        .centered()
        .block(Block::default().borders(Borders::ALL).title("Popup"));
        frame.render_widget(popup, popup_area);
    }


}