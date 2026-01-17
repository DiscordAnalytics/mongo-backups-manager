use ratatui::{
    Frame,
    layout::HorizontalAlignment,
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType},
};

pub struct ScreenLayout;

impl ScreenLayout {
    pub fn draw(frame: &mut Frame) {
        let area = frame.area();
        let title = Block::bordered()
            .border_type(BorderType::Rounded)
            .title("MongoDB Backup Manager".bold())
            .title_alignment(HorizontalAlignment::Center);
        let quit_action = Block::new().title_bottom(Line::from("Esc or q to quit").centered());

        frame.render_widget(title, area);
        frame.render_widget(quit_action, area);
    }
}
