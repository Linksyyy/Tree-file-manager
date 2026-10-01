use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

pub fn read_key(timeout: Duration) -> io::Result<Option<KeyEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }

    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(key)),
        _ => Ok(None),
    }
}
