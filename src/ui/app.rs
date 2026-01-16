use std::io::{Result, Stdout};

use ratatui::{
    Terminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    prelude::CrosstermBackend,
    widgets::ListState,
};

use crate::ui::screens::{HomeItem, HomeScreen, line_to_string};

pub enum CurrentScreen {
    Main,
}

pub struct App {
    should_quit: bool,
    current_screen: CurrentScreen,
    pub list_state: ListState,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select_first();
        Self {
            should_quit: false,
            current_screen: CurrentScreen::Main,
            list_state,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| match self.current_screen {
                CurrentScreen::Main => HomeScreen::draw(self, frame).unwrap(),
            })?;

            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key);
            }
        }

        Ok(())
    }

    pub fn set_screen(&mut self, screen: CurrentScreen) {
        self.list_state.select_first();
        self.current_screen = screen;
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match (&self.current_screen, key.code) {
            (CurrentScreen::Main, KeyCode::Down) => self.list_state.select_next(),
            (CurrentScreen::Main, KeyCode::Up) => self.list_state.select_previous(),
            (CurrentScreen::Main, KeyCode::Enter) => {
                let items = HomeScreen::list_items();
                if let Some(idx) = self.list_state.selected() {
                    match items[idx] {
                        HomeItem::Backups => {}
                        HomeItem::Exit => self.should_quit = true,
                    }
                }
            }
            (_, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
            _ => {}
        }
    }
}
