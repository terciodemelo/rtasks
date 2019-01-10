use std::fmt::Display;
use std::io::{Result, Write};
use termion::cursor::Goto;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::RawTerminal;
use termion::screen::AlternateScreen;

pub struct IO<'a> {
    pub(crate) input: &'a mut std::io::Stdin,
    pub(crate) output: &'a mut AlternateScreen<RawTerminal<std::io::Stdout>>,
}

impl<'a> IO<'a> {
    pub fn get_char(&mut self) -> Result<Key> {
        match self.input.keys().next() {
            Some(result) => result,
            None => panic!("Couldn't get key from input"),
        }
    }

    pub fn write<D: Display>(&mut self, content: D) -> Result<()> {
        write!(self.output, "{}", content)?;
        self.output.flush()
    }

    pub fn write_in_pos<D: Display>(&mut self, row: u16, column: u16, content: D) -> Result<()> {
        self.write(Goto(column, row))?;
        self.write(content)
    }

    pub fn erase(&mut self, row: u16, column: u16) -> Result<()> {
        self.write_in_pos(row, column, ' ')?;
        self.write(Goto(column, row))
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        self.write(termion::cursor::Show)
    }

    pub fn hide_cursor(&mut self) -> Result<()> {
        self.write(termion::cursor::Hide)
    }

    pub fn clear_screen(&mut self) -> Result<()> {
        self.write(termion::clear::All)
    }
}
