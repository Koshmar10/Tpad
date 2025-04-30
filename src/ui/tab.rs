use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use crate::{theme::hex_to_color, *};

pub fn render_tab_bar(f: &mut Frame<'_>, area: Rect, ctx: &data_models::RenderContext) {
    let pad = 6;
    let active_bg = hex_to_color(ctx.theme.tabs.active_bg.clone());
    let active_fg = hex_to_color(ctx.theme.tabs.active_fg.clone());
    let inactive_bg = hex_to_color(ctx.theme.tabs.inactive_bg.clone());
    let inactive_fg = hex_to_color(ctx.theme.tabs.inactive_fg.clone());
    // Create a vector with tabs, each as a tuple containing the global index and tab details.
    let opend_tabs: Vec<(usize, (String, usize))> = ctx
        .documents
        .iter()
        .enumerate()
        .map(|(i, d)| {

            let mut file_name = get_file_name(d.file_path.clone());
            if d.state.is_dirty {
                file_name += "*";
            }
            (i, (file_name.clone(), file_name.len() + pad))
        })
        .collect();

    // Partition tabs into packs based on the available area width.
    let mut tab_packs: Vec<Vec<(usize, (String, usize))>> = vec![vec![]];
    let mut current_pack_index: usize = 0;
    let mut pack_size: usize = 0;
    for tab in opend_tabs.into_iter() {
        if pack_size + tab.1.1 <= area.width as usize {
            tab_packs[current_pack_index].push(tab.clone());
            pack_size += tab.1.1;
        } else {
            current_pack_index += 1;
            tab_packs.push(vec![]);
            tab_packs[current_pack_index].push(tab.clone());
            pack_size = tab.1.1;
        }
    }

    // Calculate which pack the active tab belongs to and its relative index.
    let mut active_pack_index = None;
    let mut active_relative_index = None;
    for (pack_idx, pack) in tab_packs.iter().enumerate() {
        for (rel_idx, (global_idx, _)) in pack.iter().enumerate() {
            if *global_idx == *ctx.active {
                active_pack_index = Some(pack_idx);
                active_relative_index = Some(rel_idx);
                break;
            }
        }
        if active_pack_index.is_some() {
            break;
        }
    }

    // Retrieve the tabs for the current pack.
    let mut tabs_to_render: Vec<(usize, (String, usize))> = tab_packs
        .get(active_pack_index.unwrap_or(0))
        .cloned()
        .unwrap_or_default();

    // If this is not the last pack then add an extra overflow tab that fills the remaining space.
    if active_pack_index.unwrap_or(0) < tab_packs.len() - 1 {
        // Sum the widths used in this pack.
        let used_width: usize = tabs_to_render.iter().map(|tab| tab.1.1).sum();
        let mut remaining = if used_width < area.width as usize {
            area.width as usize - used_width
        } else {
            0
        };
        remaining = remaining + 2;
        // Use at least a few columns for the marker.
        tabs_to_render.push((usize::MAX, ("...".to_string(), remaining)));
    }

    let constraints: Vec<Constraint> = tabs_to_render
        .iter()
        .map(|tab| Constraint::Length(tab.1.1 as u16))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    // Render each visible tab.
    for (i, (&chunk, tab)) in chunks.iter().zip(tabs_to_render.iter()).enumerate() {
        // If tab's global index is usize::MAX, it's our overflow marker.
        let (name, _) = if tab.0 == usize::MAX {
            (tab.1.0.clone(), 0)
        } else {
            (tab.1.0.clone(), tab.1.1)
        };
        let style = if active_relative_index.is_some() && i == active_relative_index.unwrap() {
            (active_bg, active_fg)
        } else {
            (inactive_bg, inactive_fg)
        };
        let tab_widget = Paragraph::new(Text::from(name).style(style.1))
            .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT))
            .centered()
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(style.0));
        f.render_widget(tab_widget, chunk);
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
