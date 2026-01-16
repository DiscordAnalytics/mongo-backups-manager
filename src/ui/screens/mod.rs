mod home;
pub use home::HomeScreen;
use ratatui::{
    layout::{Constraint, Rect},
    text::Line,
};

pub fn centered_area(area: Rect) -> Rect {
    area.centered(Constraint::Percentage(50), Constraint::Percentage(50))
}

pub fn line_to_string(line: &Line) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}
