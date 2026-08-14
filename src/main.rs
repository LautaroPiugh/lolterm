mod app;
mod terminal;
mod tui;

use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = tui::init();
    let result = app::App::new(terminal.size()?).and_then(|mut app| app.run(&mut terminal));
    tui::restore();

    result
}
