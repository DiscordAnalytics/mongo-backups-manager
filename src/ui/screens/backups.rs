use std::io::Result;

use ratatui::{
  Frame,
  layout::{Constraint, Direction, Layout},
  widgets::{Block, Borders, Paragraph},
};

use crate::ui::{app::App, screens::ScreenLayout};

pub struct BackupsScreen;

impl BackupsScreen {
  pub fn draw(app: &mut App, frame: &mut Frame) -> Result<()> {
    ScreenLayout::draw(frame, Some("Backups"));

    let backups = &app.config.backups;
    let backup_names = backups
      .iter()
      .map(|(_, v)| &v.display_name)
      .collect::<Vec<_>>()
      .as_slice();

    let area = frame.area();

    let layout = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
      .split(area.centered(
        Constraint::Length(area.width - 2),
        Constraint::Length(area.height - 2),
      ));

    frame.render_widget(
      Paragraph::new("outer 0").block(Block::new().borders(Borders::ALL)),
      layout[0],
    );

    Ok(())
  }
}
