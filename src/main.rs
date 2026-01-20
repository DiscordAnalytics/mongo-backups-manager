use std::error::Error;

use clap::Parser;
use dotenvy::dotenv;

use crate::{
  cli::{Cli, Commands},
  ui::app::App,
  utils::config::Config,
};

mod cli;
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
