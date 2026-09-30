use std::path::PathBuf;
use std::{fs, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

#[derive(Debug, Default)]
pub struct App {
    actual_dirs: Vec<PathBuf>,
    // nextDirs: Vec<PathBuf>,
    exit: bool,
    show_hidden: bool,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            self.handle_actual_dirs();
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_events(key_event)
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Char('r') => self.show_hidden = !self.show_hidden,
            _ => {}
        }
    }

    fn handle_actual_dirs(&mut self) {
        self.actual_dirs = fs::read_dir(".")
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap();
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" tree-view ".bold()).centered();
        let instructions = Line::from(vec![
            " Reload ".into(),
            "<R>".blue().bold(),
            " Quit ".into(),
            "<Q>".blue().bold(),
        ]);
        let block = Block::bordered()
            .border_set(border::THICK)
            .title(title)
            .title_bottom(instructions);

        let dirs = Text::from(
            self.actual_dirs
                .iter()
                .filter_map(|path| {
                    let file_name = path.file_name().unwrap().to_string_lossy();
                    let is_hidden = file_name.to_lowercase().starts_with('.');

                    if is_hidden && !self.show_hidden {
                        return None;
                    }

                    Some(Line::from(file_name.into_owned()))
                })
                .collect::<Vec<_>>(),
        );
        Paragraph::new(dirs)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
