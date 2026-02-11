pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
  Backup {
    #[command(subcommand)]
    action: BackupCommand,
  },
  Daemon,
  Tui,
}

#[derive(Subcommand)]
pub enum BackupCommand {
  Inspect,
  List,
  Start { name: String },
}
