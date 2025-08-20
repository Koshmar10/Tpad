use ratatui::{layout::{Rect, Alignment}, style::{Color, Style, Stylize}, text::{Line, Text}, widgets::{Block, Borders, Paragraph}, Frame};
use crate::{data_models::PopupTypes::{ErrorPopup, SaveOnClosePopup, ThemeSelectPopup, InfoPopup}, theme::hex_to_color};
use crate::{RenderContext};



pub fn render_popup(frame: &mut Frame<'_>,  ctx: &RenderContext){
    //asiging popup space
    let area = frame.area();
    let mut popup_width = area.width / 2;
    let mut popup_height = 5;
    

    

    
    match ctx.popup {
        Some(p) => {
            let height = p.msg.split('\n').collect::<Vec<&str>>().len();
            let popup:Paragraph =  match p.kind{
                ErrorPopup => {
                    let fg = hex_to_color(ctx.theme.popup.error_fg.to_owned());
                    let bg = hex_to_color(ctx.theme.popup.error_bg.to_owned());
                    let text = Text::from(vec![
                        Line::from(format!("{: ^1$}", " ", area.width as usize/2)),
                        Line::raw(format!("{: ^1$}", &p.msg, area.width as usize/2)),
                        Line::from(format!("{: ^1$}", " ", area.width as usize/2)),]
                        
                    );
                    Paragraph::new(text)
                        .block(Block::default().borders(Borders::ALL).style(bg))
                        .style(Style::default().fg(fg))
                },
                SaveOnClosePopup => {
                    let fg = hex_to_color(ctx.theme.popup.fg.to_owned());
                    let bg = hex_to_color(ctx.theme.popup.bg.to_owned());
                    let text = Text::from(vec![
                        Line::from(format!("{: ^1$}", " ", area.width as usize/2)),
                        Line::raw(format!("{: ^1$}", &p.msg, area.width as usize/2)),
                        Line::from(format!("{: ^1$}", " ", area.width as usize/2)),]
                    );
                    Paragraph::new(text)
                        .block(Block::default().borders(Borders::ALL).style(bg))
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(fg))
                },
                InfoPopup => {
                    let fg = hex_to_color(ctx.theme.popup.fg.to_owned());
                    let bg = hex_to_color(ctx.theme.popup.bg.to_owned());
                    let mut lines: Vec<Line> = Vec::new();
                    let indent = " ".repeat((area.width as usize) / 8);

                    // top padding line, indented
                    lines.push(Line::raw(indent.clone()));

                    // message lines, indented
                    for msg_line in p.msg.split('\n') {
                        lines.push(Line::raw(format!("{}{}", indent.clone(), msg_line)));
                    }

                    // bottom padding line, indented
                    lines.push(Line::raw(indent.clone()));

                    // Adjust popup_height based on the number of lines generated
                    popup_height = lines.len() as u16;
                    // Ensure a minimum height if necessary
                    if popup_height < 3 { 
                        popup_height = 3;
                    }

                    let text = Text::from(lines);
                    Paragraph::new(text)
                        .block(Block::default().borders(Borders::ALL).style(bg))
                        .alignment(Alignment::Left) // Align text to the left within the popup block
                        .style(Style::default().fg(fg))
                },
                
                // Set styles for the theme selection popup
                ThemeSelectPopup => {
                let fg = hex_to_color(ctx.theme.popup.fg.to_owned());
                let bg = hex_to_color(ctx.theme.popup.bg.to_owned());

                // Build the list of themes and adjust the popup height accordingly
                let files: Vec<Line> = match ctx.theme.list_themes() {
                    Ok(list) => {
                        popup_height = 2 + list.len() as u16;
                        list.into_iter()
                            .enumerate()
                            .filter_map(|(i, dir)| { // Use filter_map to handle potential None from file_name
                                dir.file_name().map(|entry_osstr| {
                                    let mut entry_str = entry_osstr.to_string_lossy().into_owned();
                                    entry_str = format!("{: ^1$}", entry_str, area.width as usize /2 -2);
                                    if i == *ctx.selected_theme {
                                        Line::from(entry_str).style(Style::default().bg(bg))
                                    } else {
                                        Line::from(entry_str).style(Style::default())
                                    }
                                })
                            })
                            .collect()
                    }
                    Err(_) => {
                        popup_height = 3;
                        vec![Line::from("failed loading theme").style(Style::default())]
                    }
                };
                    let themes = Text::from(files);
                    Paragraph::new(themes)
                        .block(Block::default().borders(Borders::ALL).style(bg))
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(fg))
                }
            };
            let  popup_x = area.x + (area.width - popup_width) / 2;
            let  popup_y = area.y + (area.height - popup_height) / 2;
            let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
            frame.render_widget(popup, popup_area);
        }
        None =>{}
    }
        
    


}