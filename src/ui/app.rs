use std::io;

use ratatui::{
  Terminal,
  crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
  },
  prelude::CrosstermBackend,
  widgets::ListState,
};

use crate::{
  db::DatabaseConnection,
  ui::screens::{BackupsScreen, HomeItem, HomeScreen, SettingsScreen},
  utils::config::Config,
};

#[derive(PartialEq)]
pub enum CurrentScreen {
  Main,
  Backups,
  Settings,
}

pub struct App {
  should_quit: bool,
  pub current_screen: CurrentScreen,
  pub list_state: ListState,
  pub config: Config,
  pub database_connection: DatabaseConnection,
}

impl App {
  pub fn new(config: Config) -> Self {
    let mut list_state = ListState::default();
    list_state.select_first();
    Self {
      should_quit: false,
      current_screen: CurrentScreen::Main,
      list_state,
      config,
      database_connection: DatabaseConnection::new(),
    }
  }

  pub async fn run(&mut self) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while !self.should_quit {
      terminal.draw(|frame| match self.current_screen {
        CurrentScreen::Main => {
          if let Err(e) = HomeScreen::draw(self, frame) {
            eprintln!("Draw error: {}", e);
          }
        }
        CurrentScreen::Backups => {
          if let Err(e) = BackupsScreen::draw(self, frame) {
            eprintln!("Draw error: {}", e);
          }
        }
        CurrentScreen::Settings => {
          if let Err(e) = SettingsScreen::draw(self, frame) {
            eprintln!("Draw error: {}", e);
          }
        }
      })?;

      if let Event::Key(key) = event::read()? {
        self.handle_key_event(key);
      }
    }

    disable_raw_mode()?;
    execute!(
      terminal.backend_mut(),
      LeaveAlternateScreen,
      DisableMouseCapture
    )?;
    terminal.show_cursor()?;

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
      (CurrentScreen::Main | CurrentScreen::Backups, KeyCode::Down) => {
        self.list_state.select_next()
      }
      (CurrentScreen::Main | CurrentScreen::Backups, KeyCode::Up) => {
        self.list_state.select_previous()
      }
      (CurrentScreen::Main, KeyCode::Enter) => {
        let items = HomeScreen::list_items();
        if let Some(idx) = self.list_state.selected() {
          match items[idx] {
            HomeItem::Backups => self.set_screen(CurrentScreen::Backups),
            HomeItem::Settings => self.set_screen(CurrentScreen::Settings),
            HomeItem::Exit => self.should_quit = true,
          }
        }
      }
      (CurrentScreen::Backups | CurrentScreen::Settings, KeyCode::Backspace) => {
        self.set_screen(CurrentScreen::Main)
      }
      (_, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
      _ => {}
    }
  }
}
