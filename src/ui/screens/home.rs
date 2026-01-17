use std::io::Result;

use ratatui::{
    Frame,
    layout::HorizontalAlignment,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, List, ListDirection, ListItem, Paragraph},
};

use crate::ui::{
    app::App,
    screens::{ScreenLayout, centered_area, to_list_items},
};

pub enum HomeItem {
    Backups,
    Exit,
}

impl From<&HomeItem> for ListItem<'_> {
    fn from(value: &HomeItem) -> Self {
        let line = Line::from(match value {
            HomeItem::Backups => "Backups",
            HomeItem::Exit => "Exit",
        })
        .alignment(HorizontalAlignment::Center);
        ListItem::new(line)
    }
}

pub struct HomeScreen;

impl HomeScreen {
    pub fn draw(app: &mut App, frame: &mut Frame) -> Result<()> {
        ScreenLayout::draw(frame);

        let area = frame.area();

        let list_block = Block::bordered().border_type(BorderType::Rounded);
        let list = List::new(to_list_items(Self::list_items()))
            .block(list_block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol("▶")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);
        frame.render_stateful_widget(list, centered_area(area), &mut app.list_state);

        let desc_block =
            Block::new().title_bottom(Line::from("↑ or ↓ to navigate, Enter to select").centered());
        frame.render_widget(desc_block, centered_area(area));

        Ok(())
    }

    pub fn list_items() -> &'static [HomeItem] {
        static ITEMS: [HomeItem; 2] = [HomeItem::Backups, HomeItem::Exit];
        &ITEMS
    }
}
