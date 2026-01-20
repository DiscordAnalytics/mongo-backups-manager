mod databases;
pub use databases::DatabasesScreen;
mod home;
pub use home::{HomeItem, HomeScreen};
mod layout;
pub use layout::ScreenLayout;
mod settings;
pub use settings::SettingsScreen;

use ratatui::{
  layout::{Constraint, Rect},
  widgets::ListItem,
};

pub fn centered_area(area: Rect) -> Rect {
  area.centered(Constraint::Percentage(50), Constraint::Percentage(50))
}

pub fn to_list_items<T>(items: &[T]) -> Vec<ListItem<'static>>
where
  for<'a> ListItem<'static>: From<&'a T>,
{
  items.iter().map(ListItem::from).collect()
}
