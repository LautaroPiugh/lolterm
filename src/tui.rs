use ratatui::DefaultTerminal;

pub fn init() -> DefaultTerminal {
    ratatui::init()
}

pub fn restore() {
    ratatui::restore();
}
