use ratatui::{
    Frame,
    layout::HorizontalAlignment,
    style::Stylize,
    widgets::{Block, Paragraph},
};

use crate::ui::app::App;

pub fn home(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let title = Block::bordered()
        .title("MongoDB Backup Manager".bold())
        .title_alignment(HorizontalAlignment::Center);
    let greeting = Paragraph::new("Welcome to MongoDB Backup Manager!")
        .block(title)
        .alignment(HorizontalAlignment::Center);
    frame.render_widget(greeting, area);
}
