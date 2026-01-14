use std::io::{Result, Stdout};

use ratatui::{
    Frame, Terminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    prelude::CrosstermBackend,
};

use crate::ui::screens;

pub enum CurrentScreen {
    Main,
}

pub struct App {
    pub current_screen: CurrentScreen,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: CurrentScreen::Main,
            should_quit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;

            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key);
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        match self.current_screen {
            CurrentScreen::Main => screens::home(frame, self),
        }
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }
}
