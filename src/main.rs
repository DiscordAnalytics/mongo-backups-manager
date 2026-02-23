use std::error::Error;

use clap::Parser;
use dotenvy::dotenv;

use crate::{
  cli::{
    BackupCommand, Cli, Commands,
    commands::{self, Daemon},
  },
  ui::app::App,
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

  match cli.command {
    Some(Commands::Daemon) => Daemon::start().await,
    Some(Commands::Backup { action }) => match action {
      BackupCommand::Inspect { name } => commands::backup::inspect(name),
      BackupCommand::List => commands::backup::list(),
      BackupCommand::Restore { name, target } => commands::backup::restore(name, target).await,
      BackupCommand::Start { name } => commands::backup::start(name).await,
    },
    None | Some(Commands::Tui) => App::new().run().await?,
  };

  Ok(())
}

#[cfg(test)]
pub mod tests {
  use std::{fs::remove_dir_all, path::PathBuf};

  pub fn get_test_dir_path(test_name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/mbm_tests_{test_name}").as_str())
  }

  pub fn clean_test_dir(path: PathBuf) {
    remove_dir_all(path).unwrap_or(());
  }
}
