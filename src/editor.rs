use crate::Document;
use crate::Row;
use crate::Terminal;
use std::env;
use std::time::Duration;
use std::time::Instant;
use crossterm::style::Color;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            time:Instant::now(),
            text: message,
        }
    }
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_message: StatusMessage,
    quit_times: u8,
}

impl Editor {
    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(error);
            }
        }
    }

    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = String::from("Help: Ctrl-S = save | Ctrl-Q = quit");
        let document = if args.len() > 1 {
            let file_name = &args[1];
            let doc = Document::open(&file_name);
            if doc.is_ok() {
                doc.unwrap()
            } else {
                initial_status = format!("ERR: Could not open file: {}", file_name);
                Document::default()
            }
        } else {
            Document::default()
        };
        Self { 
            should_quit: false,
            terminal: Terminal::default().expect("Failed to initialize terminal"),
            document,
            cursor_position: Position::default(),
            offset: Position::default(),
            status_message: StatusMessage::from(initial_status),
            quit_times: QUIT_TIMES
        }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::cursor_position(&Position::default())?;
        if self.should_quit {
            Terminal::clear_screen()?;
            println!("Goodbye. \r");
        } else {
            Terminal::clear_screen()?;
            self.draw_rows();  
            self.draw_status_bar();
            self.draw_message_bar();
            Terminal::cursor_position(&Position {
                 x: self.cursor_position.x.saturating_sub(self.offset.x), 
                 y: self.cursor_position.y.saturating_sub(self.offset.y),
            })?; 
        }
        Terminal::flush()?;
        Ok(())
    }

    fn save(&mut self) {
        if self.document.file_name.is_none() {
            let new_name = self.prompt("Save as: ").unwrap_or(None);
            if new_name.is_none() {
                self.status_message = StatusMessage::from("Save aborted.".to_string());
                return;
            }
            self.document.file_name = new_name;
        }

        if self.document.save().is_ok() {
            self.status_message = StatusMessage::from("File saved successfully.".to_string());
        } else {
            self.status_message = StatusMessage::from("Error writing file!".to_string());
        }
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let KeyEvent { code, modifiers, .. } = Terminal::read_key()?;
    
        match (code, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                if self.quit_times > 0 && self.document.is_dirty() {
                    self.status_message = StatusMessage::from(format!(
                        "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                        self.quit_times
                    ));
                    self.quit_times -= 1;
                    return Ok(());
                }
                self.should_quit = true
            },
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.save(),
            (KeyCode::Char(c), KeyModifiers::NONE) => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(KeyCode::Right);
            },
            (KeyCode::Delete, KeyModifiers::NONE) => self.document.delete(&self.cursor_position),
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(KeyCode::Left);
                    self.document.delete(&self.cursor_position);
                }
            },
            (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right, KeyModifiers::NONE) => self.move_cursor(code),
            _ => {}
        }
        self.scroll();
        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_message = StatusMessage::from(String::new());
        }
        Ok(())
    }


    fn prompt(&mut self, prompt: &str) -> Result<Option<String>, std::io::Error> {
        let mut result = String::new();
        loop {
            self.status_message = StatusMessage::from(format!("{}{}", prompt, result));
            self.refresh_screen()?;
            
            match Terminal::read_key() {
                Ok(KeyEvent{ code: KeyCode::Backspace, modifiers: KeyModifiers::NONE, .. }) => {
                    if !result.is_empty() {
                        result.truncate(result.len() - 1);
                    }
                },
                Ok(KeyEvent{ code: KeyCode::Char('\n'), modifiers: KeyModifiers::NONE, .. }) => break,
                Ok(KeyEvent{ code: KeyCode::Char(c), modifiers: KeyModifiers::NONE, .. }) => {
                    if !c.is_control() {
                        result.push(c);
                    }
                },
                Ok(KeyEvent{ code: KeyCode::Esc, modifiers: KeyModifiers::NONE, .. }) => {
                    result.truncate(0);
                    break;
                },
                _ => (),
            }            
        }
        self.status_message = StatusMessage::from(String::new());
        if result.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(result))
    }
    

    fn draw_welcome_message(&self) {
        let mut welcome_message = format!("RTT - Version: {}", VERSION);
        let width = self.terminal.size().width as usize;
        let len = welcome_message.len();
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{}{}", spaces, welcome_message);
        welcome_message.truncate(width);
        println!("{}\r", welcome_message);
    }    

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;
        let mut offset = &mut self.offset;
        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_sub(height).saturating_add(1);
        }
        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, key: KeyCode) {
        let terminal_height = self.terminal.size().height as usize;
        let Position { mut y, mut x } = self.cursor_position;
        let height = self.document.len();
        let mut width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        match key {
            KeyCode::Up => y = y.saturating_sub(1),
            KeyCode::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            },
            KeyCode::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            },
            KeyCode::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            },
            KeyCode::PageUp => {
              y = if y > terminal_height {
                y - terminal_height
              } else {
                0
              }
            },
            KeyCode::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y + terminal_height as usize
                } else {
                    height
                }
            },
            KeyCode::Home => x = 0,
            KeyCode::End => x = width,
            _ => (),
        }
        width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        if x > width {
            x = width;
        }

        self.cursor_position = Position { x, y }
    }

    pub fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x + width;
        let row = row.render(start, end);
        println!("{}\r", row)
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line().unwrap_or_default();
            if let Some(row) = self.document.row(terminal_row as usize + self.offset.y) {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcome_message();
            } else {
                println!("~\r");
            }
        }
    }

    fn draw_status_bar(&self) {
        let mut status;
        let width = self.terminal.size().width as usize;
        let modified_indicator = if self.document.is_dirty() {
            " (modified)"
        } else {
            ""
        };
        let mut file_name = "[No Name]".to_string();
        if let Some(name) = &self.document.file_name {
            file_name = name.clone();
            file_name.truncate(20);
        }
        status = format!("{} - {} lines", file_name, self.document.len());
        
        let line_indicator = format!(
            "{}/{}",
            self.cursor_position.y.saturating_add(1),
            self.document.len()
        );
        let len = status.len() + line_indicator.len();
        if width > len {
            status.push_str(&" ".repeat(width - len));
        }

        status = format!(
            "{} - {} lines{}",
            file_name,
            self.document.len(),
            modified_indicator
        );

        let line_indicator = format!(
            "{}/{}",
            self.cursor_position.y.saturating_add(1),
            self.document.len()
        );
        let len = status.len() + line_indicator.len();
        if width > len {
            status.push_str(&" ".repeat(width - len));
        }
        status = format!("{}{}", status, line_indicator);
        status.truncate(width);

        Terminal::set_background_color(Color::DarkBlue).unwrap_or_default();
        Terminal::set_foreground_color(Color::White).unwrap_or_default();
        println!("{}\r", status);
        Terminal::reset_color().unwrap_or_default();
    }

    fn draw_message_bar(&self) {
        Terminal::clear_current_line().unwrap_or_default();
        let message = &self.status_message;
        if Instant::now() - message.time < Duration::new(5, 0) {
            let mut text = message.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{}", text);
        }
    }
}

fn die(e: std::io::Error) {
    Terminal::clear_screen().unwrap_or_else(|_| {
        eprintln!("Failed to clear the terminal");
    });

    panic!("{}", e);
}

