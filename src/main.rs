mod app;
mod terminal;
mod tui;

use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut app = app::App::new()?;
    let mut terminal = tui::init();
    let result = app.run(&mut terminal);
    tui::restore();

    result
}
