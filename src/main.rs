use std::error::Error;

use clap::Parser;
use dotenvy::dotenv;

use crate::{
  cli::{Cli, Commands},
  ui::app::App,
  utils::config::Config,
};

mod cli;
mod datastores;
mod db;
mod ui;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  dotenv().ok();
  let cli = Cli::parse();
  let config = Config::new();

  match cli.command {
    None | Some(Commands::Tui) => App::new().run().await?,
  };

  Ok(())
}

#[cfg(test)]
pub mod tests {
  use std::fs::remove_dir_all;
  use std::path::PathBuf;

  pub fn get_test_dir_path(test_name: &str) -> String {
    PathBuf::from(format!("/tmp/mbm_tests_{test_name}").as_str())
      .display()
      .to_string()
  }

  pub fn clean_test_dir(path: String) {
    let _ = remove_dir_all(path).unwrap_or(());
  }
}
