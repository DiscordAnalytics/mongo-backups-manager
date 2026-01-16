use std::io::Result;

use ratatui::{
    Frame,
    layout::HorizontalAlignment,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, List, ListDirection, Paragraph},
};

use crate::ui::{app::App, screens::centered_area};

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
        let items = [
            Line::from("Backups").alignment(HorizontalAlignment::Center),
            Line::from("Exit").alignment(HorizontalAlignment::Center),
        ];
        let list = List::new(items)
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
}
