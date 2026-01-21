use std::io::Result;

use ratatui::Frame;

use crate::ui::{app::App, screens::ScreenLayout};

pub struct SettingsScreen;

impl SettingsScreen {
  pub fn draw(app: &mut App, frame: &mut Frame) -> Result<()> {
    ScreenLayout::draw(app, frame, Some("Settings"));

    Ok(())
  }
}
