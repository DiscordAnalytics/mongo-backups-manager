use std::io::Result;

use crate::datastores::Datastore;
use crate::ui::{app::App, screens::ScreenLayout};
use crate::utils::backup_manager::BackupJob;
use ratatui::prelude::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Borders, List, ListDirection, Paragraph, Wrap};
use ratatui::{
  Frame,
  widgets::{Block, BorderType},
};

pub struct BackupInspectScreen;

impl BackupInspectScreen {
  pub fn draw(app: &mut App, backup: BackupJob, frame: &mut Frame) -> Result<()> {
    ScreenLayout::draw(app, frame, Some(&backup.display_name));

    let area = frame.area();

    let text = format!(
      "Datastore:\n\tType: {:?}\n\tPath: {}\n\tSchedule: {:?}\n{:?}Encryption: {}",
      backup.datastore.as_str(),
      backup.datastore.get_base_path(),
      if let Some(schedule) = backup.clone().raw_schedule {
        schedule
      } else {
        "disabled".to_string()
      },
      if let Some(next) = backup.get_next_run() {
        format!("Next run: {:?}", next)
      } else {
        String::new()
      },
      if backup.is_encryption_enabled() {
        "enabled"
      } else {
        "disabled"
      }
    );
    let mut lines = vec![
      Line::raw("Datastore:"),
      Line::raw(format!("    Type: {}", backup.datastore.as_str())),
      Line::raw(format!("    Path: {}", backup.datastore.get_base_path())),
      Line::raw(format!(
        "Schedule: {}",
        if let Some(schedule) = backup.clone().raw_schedule {
          schedule
        } else {
          "disabled".to_string()
        }
      )),
      Line::raw(format!(
        "Encryption: {}",
        if backup.is_encryption_enabled() {
          "enabled"
        } else {
          "disabled"
        }
      )),
    ];

    if let Some(next) = backup.clone().get_next_run() {
      lines.insert(4, Line::raw(format!("Next run: {:?}", next)))
    }

    let paragraph = Paragraph::new(lines).block(
      Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(backup.display_name),
    );

    frame.render_widget(paragraph, area);

    Ok(())
  }
}
