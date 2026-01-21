use std::io::Result;

use ratatui::{
  Frame,
  layout::{Constraint, Direction, Layout},
  style::Style,
  widgets::{Block, BorderType, List, ListDirection},
};

use crate::ui::{app::App, screens::ScreenLayout};

pub struct BackupsScreen;

impl BackupsScreen {
  pub fn draw(app: &mut App, frame: &mut Frame) -> Result<()> {
    ScreenLayout::draw(app, frame, Some("Backups"));

    let backups = &app.config.backups;
    let backup_names = backups
      .iter()
      .map(|(_, v)| v.display_name.clone())
      .collect::<Vec<_>>();

    let area = frame.area();

    let layout = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
      .split(area.centered(
        Constraint::Length(area.width - 2),
        Constraint::Length(area.height - 2),
      ));

    let list_block = Block::bordered().border_type(BorderType::Rounded);
    let list = List::new(backup_names)
      .block(list_block)
      .highlight_style(Style::new().reversed())
      .highlight_symbol("▶")
      .repeat_highlight_symbol(true)
      .direction(ListDirection::TopToBottom);

    frame.render_stateful_widget(list, layout[0], &mut app.list_state);

    Ok(())
  }
}
