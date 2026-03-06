use ratatui::{
  Frame,
  layout::HorizontalAlignment,
  style::Stylize,
  text::Line,
  widgets::{Block, BorderType},
};

use crate::ui::app::{App, CurrentScreen};

pub struct ScreenLayout;

impl ScreenLayout {
  pub fn draw(app: &mut App, frame: &mut Frame, title: Option<&str>) {
    let area = frame.area();

    let display_title = format!(
      "MongoDB Backup Manager{}",
      title.map_or("".to_string(), |t| format!(" - {}", t))
    );
    let app_title = Block::bordered()
      .border_type(BorderType::Rounded)
      .title(display_title.bold())
      .title_alignment(HorizontalAlignment::Left);
    let hint_text = format!(
      "Esc or q to exit{}",
      if app.current_screen != CurrentScreen::Main {
        ", Backspace to go back"
      } else {
        ""
      }
    );
    let action_hint = Block::new().title_bottom(Line::from(hint_text).centered());

    frame.render_widget(app_title, area);
    frame.render_widget(action_hint, area);
  }
}
