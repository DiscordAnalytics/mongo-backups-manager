use ratatui::{
  Frame,
  layout::HorizontalAlignment,
  style::Stylize,
  text::Line,
  widgets::{Block, BorderType},
};

pub struct ScreenLayout;

impl ScreenLayout {
  pub fn draw(frame: &mut Frame, title: Option<&str>) {
    let area = frame.area();
    let display_title = format!(
      "MongoDB Backup Manager{}",
      title.map_or("".to_string(), |t| format!(" - {}", t))
    );
    let app_title = Block::bordered()
      .border_type(BorderType::Rounded)
      .title(display_title.bold())
      .title_alignment(HorizontalAlignment::Left);
    let quit_action = Block::new().title_bottom(Line::from("Esc or q to exit").centered());

    frame.render_widget(app_title, area);
    frame.render_widget(quit_action, area);
  }
}
