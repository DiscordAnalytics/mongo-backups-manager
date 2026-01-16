mod home;
pub use home::{HomeItem, HomeScreen};
use ratatui::{
    layout::{Constraint, Rect},
    widgets::ListItem,
};

pub fn centered_area(area: Rect) -> Rect {
    area.centered(Constraint::Percentage(50), Constraint::Percentage(50))
}

pub fn to_list_items<T>(items: Vec<T>) -> Vec<ListItem<'static>>
where
    for<'a> ListItem<'static>: From<&'a T>,
{
    items.iter().map(ListItem::from).collect()
}
