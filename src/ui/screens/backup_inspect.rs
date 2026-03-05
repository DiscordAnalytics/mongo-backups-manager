use crate::datastores::Datastore;
use crate::ui::{app::App, screens::ScreenLayout};
use crate::utils::backup_manager::BackupJob;
use chrono::DateTime;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Borders, List, ListDirection, Paragraph, Wrap};
use ratatui::{
  Frame,
  widgets::{Block, BorderType},
};
use std::io::Result;

pub struct BackupInspectScreen;

impl BackupInspectScreen {
  pub fn draw(app: &mut App, backup: BackupJob, frame: &mut Frame) -> Result<()> {
    ScreenLayout::draw(app, frame, Some(&backup.display_name));

    let area = frame.area();
    let layout = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![
        Constraint::Min(25),
        Constraint::Length(1),
        Constraint::Fill(1),
      ])
      .split(area.centered(
        Constraint::Length(area.width - 2),
        Constraint::Length(area.height - 2),
      ));

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
        .border_type(BorderType::Rounded),
    );

    frame.render_widget(paragraph, layout[0]);

    let mut backups_labels = backup
      .datastore
      .get_backups(&backup.identifier)
      .iter()
      .map(|(dir_name, datastore)| {
        let health_state = datastore.check_backup_integrity().is_ok_and(|res| res);
        let timestamp = dir_name
          .rsplit("_")
          .next()
          .and_then(|s| s.parse::<i64>().ok())
          .unwrap_or(0);
        let date = match DateTime::from_timestamp_secs(timestamp) {
          Some(value) => value.to_rfc3339(),
          None => "Unknown date".to_string(),
        };
        format!(
          "\t{} - {} - {}",
          dir_name,
          date,
          if health_state { "Healthy" } else { "Unhealthy" }
        )
      })
      .collect::<Vec<String>>();
    backups_labels.push("Trigger manual backup".to_string());

    let list_block = Block::bordered().border_type(BorderType::Rounded);
    let list = List::new(backups_labels)
      .block(list_block)
      .highlight_style(Style::new().reversed())
      .highlight_symbol("▶")
      .repeat_highlight_symbol(true)
      .direction(ListDirection::TopToBottom);

    frame.render_stateful_widget(list, layout[2], &mut app.list_state);

    Ok(())
  }
}
