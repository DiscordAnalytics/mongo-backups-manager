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
    screens::{centered_area, to_list_items},
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
        let area = frame.area();

        let title = Block::bordered()
            .border_type(BorderType::Rounded)
            .title("MongoDB Backup Manager".bold())
            .title_alignment(HorizontalAlignment::Center);
        let greeting = Paragraph::new("Welcome to MongoDB Backup Manager!")
            .block(title)
            .centered();
        frame.render_widget(greeting, area);

        let quit_action = Block::new().title_bottom(Line::from("Esc or q to quit").centered());
        frame.render_widget(quit_action, area);

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

    pub fn list_items() -> Vec<HomeItem> {
        vec![HomeItem::Backups, HomeItem::Exit]
    }
}
