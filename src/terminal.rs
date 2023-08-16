use crate::Position;
use std::io::{self, Write};
use crossterm::event::{Event, KeyEvent, DisableMouseCapture};
use crossterm::terminal::ClearType;
use crossterm::style::{Print, SetForegroundColor, SetBackgroundColor, ResetColor, Color, Attribute};
use crossterm::{event, execute, terminal, cursor};

pub struct Size {
    pub width: u16,
    pub height: u16,
}

pub struct Terminal {
    size: Size,
}

impl Terminal {
    pub fn default() -> Result<Self, std::io::Error> {
        terminal::enable_raw_mode().expect("Could not turn on Raw mode");

        let size = crossterm::terminal::size().unwrap();
        Ok(Self {
            size: Size {
                width: size.0,
                height: size.1.saturating_sub(2),
            },
        })
    }

    pub fn size(&self) -> &Size {
        &self.size
    }
    
    pub fn cursor_position(position: &Position) -> Result<(), std::io::Error> {
        let Position{mut x, mut y} = position;
        let x = x as u16;
        let y = y as u16;
        execute!(io::stdout(), cursor::MoveTo(x, y))
    }

    pub fn clear_screen() -> Result<(), std::io::Error> {
        execute!(io::stdout(), terminal::Clear(ClearType::All))
    }

    pub fn clear_current_line() -> Result<(), std::io::Error> {
        execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine))
    }

    pub fn flush() -> Result<(), std::io::Error> {
        io::stdout().flush()
    }
    
    pub fn read_key() -> Result<KeyEvent, std::io::Error> {
        loop {
            if let Ok(Event::Key(key_event)) = event::read() {
                return Ok(key_event);
            }
        }
    }
    pub fn set_background_color(color: Color) -> io::Result<()> {
        execute!(io::stdout(), SetBackgroundColor(color))
    }
    pub fn set_foreground_color(color: Color) -> io::Result<()> {
        execute!(io::stdout(), SetForegroundColor(color)) 
    }
    pub fn reset_color() -> io::Result<()> {
        execute!(io::stdout(), ResetColor) 
    }
}