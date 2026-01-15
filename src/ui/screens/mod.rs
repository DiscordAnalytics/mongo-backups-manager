mod home;
pub use home::HomeScreen;
use ratatui::layout::{Constraint, Rect};

pub fn centered_area(area: Rect) -> Rect {
    area.centered(Constraint::Percentage(50), Constraint::Percentage(50))
}
