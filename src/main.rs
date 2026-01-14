use std::error::Error;

use crate::ui::app::App;

mod ui;

fn main() -> Result<(), Box<dyn Error>> {
    ratatui::run(|terminal| {
        let mut app = App::new();
        let res = app.run(terminal);
        if let Err(err) = res {
            println!("{err:?}");
        }
    });

    Ok(())
}
