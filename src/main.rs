use dotenvy::dotenv;
use std::error::Error;
use clap::Parser;

use crate::{
  utils::{
    config::Config,
  },
  cli::{Cli, Commands},
  ui::app::App
};

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

mod cli;
mod db;
mod ui;
mod utils;