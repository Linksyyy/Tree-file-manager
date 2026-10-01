mod app;
mod event;
mod filesystem;
mod layout;
mod tui;
mod ui;

use std::time::Duration;

use color_eyre::Result;

use app::App;
use tui::Tui;

fn main() -> Result<()> {
    color_eyre::install()?;

    let root_path = std::env::current_dir()?;
    let mut app = App::new(root_path)?;
    let mut terminal = Tui::new()?;
    terminal.enter()?;

    let result = run(&mut app, &mut terminal);
    let exit_result = terminal.exit();

    result?;
    exit_result?;

    if let Some(path_file) = std::env::var_os("KD_CWD_FILE") {
        std::fs::write(
            path_file,
            app.exit_path().as_os_str().to_string_lossy().as_bytes(),
        )?;
    }
    Ok(())
}

fn run(app: &mut App, terminal: &mut Tui) -> Result<()> {
    while !app.should_quit {
        terminal.draw(app)?;

        if let Some(key) = event::read_key(Duration::from_millis(250))? {
            app.handle_key(key);
        }
    }

    Ok(())
}
