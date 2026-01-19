use std::io::Result;

use ratatui::Frame;

use crate::ui::{app::App, screens::ScreenLayout};

pub struct DatabasesScreen;

impl DatabasesScreen {
  pub fn draw(app: &mut App, frame: &mut Frame) -> Result<()> {
    ScreenLayout::draw(frame, Some("Databases"));

    Ok(())
  }
}