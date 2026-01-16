use std::error::Error;

use clap::Parser;

use crate::{
    cli::{Cli, Commands},
    ui::app::App,
};

mod cli;
mod ui;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Tui) => ratatui::run(|terminal| {
            let mut app = App::new();
            if let Err(err) = app.run(terminal) {
                eprintln!("{err:?}");
            }
        }),
    };

    Ok(())
}
