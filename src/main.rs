#![feature(box_syntax, box_patterns)]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod database;
mod formatted_string;
mod io;
mod project;

use crate::database::*;
use crate::formatted_string::*;
use crate::io::*;
use crate::project::*;

use std::io::Result;
use std::io::{stdin, stdout};
use termion::color::Rgb;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

const HEADER_OFFSET: u16 = 2;
const DIV_COLOR: Rgb = Rgb(0, 150, 230);
const YELLOW: Rgb = Rgb(241, 196, 15);
const PINK: Rgb = Rgb(200, 0, 150);
const BLUE: Rgb = Rgb(52, 152, 219);

fn main() -> Result<()> {
    let mut database = Database::load()?;

    let mut io = IO {
        input: &mut stdin(),
        output: &mut AlternateScreen::from(stdout().into_raw_mode().unwrap()),
    };

    handle_user_input(&mut io, &mut database)
}

#[derive(Copy, Clone)]
enum Context {
    Project(u16, u16),
    Task(u16, u16),
}

impl Context {
    fn idx(self) -> usize {
        match self {
            Context::Project(row, _) => (row - HEADER_OFFSET - 1) as usize,
            Context::Task(row, _) => (row - HEADER_OFFSET - 1) as usize,
        }
    }

    fn drop(self) -> Option<Context> {
        match self {
            Context::Project(row, len) => {
                if len > 0 {
                    Some(Context::Project(row - 1, len - 1))
                } else {
                    None
                }
            }
            Context::Task(row, len) => {
                if len > 0 {
                    Some(Context::Task(row - 1, len - 1))
                } else {
                    None
                }
            }
        }
    }

    fn length(self) -> usize {
        match self {
            Context::Project(_, length) => length as usize,
            Context::Task(_, length) => length as usize,
        }
    }

    fn jump_to(self, index: usize) -> Option<Context> {
        self.jump(index as i16 - self.idx() as i16)
    }

    fn jump(self, distance: i16) -> Option<Context> {
        let index = self.idx() as i16;
        match self {
            Context::Project(row, len) => {
                if index + distance >= 0 && index + distance < len as i16 {
                    Some(Context::Project((row as i16 + distance) as u16, len))
                } else {
                    None
                }
            }
            Context::Task(row, len) => {
                if index + distance >= 0 && index + distance < len as i16 {
                    Some(Context::Task((row as i16 + distance) as u16, len))
                } else {
                    None
                }
            }
        }
    }

    fn pane_div(self, terminal_width: u16) -> String {
        let columns = match self {
            Context::Project(_, _) => vec![0, 8, 16, 26, 33],
            Context::Task(_, _) => vec![0, 10, terminal_width - 24],
        };
        let raw_div = (0..terminal_width - 3)
            .map(|i| if columns.contains(&i) { "╋" } else { "━" })
            .collect::<String>();

        FormattedString::from(&raw_div).fg(DIV_COLOR).to_string()
    }
}

fn numbered_row<'a>(row: u16, focused_row: u16, content: &Listable) -> String {
    let row_number = if row > HEADER_OFFSET {
        (row - HEADER_OFFSET).to_string()
    } else {
        " ".to_string()
    };

    let cursor = FormattedString::from(&row_number).right(3);
    let formatted_content = FormattedString::from(&content.view());

    if row == focused_row {
        cursor.fg(YELLOW).concat(&formatted_content.focused())
    } else {
        cursor.concat(&formatted_content)
    }
}

fn confirm_deletion<'a>(row: u16, io: &mut IO<'a>) -> Result<bool> {
    let question = FormattedString::from("Are you sure you want to delete this row?").fg(YELLOW);
    io.write_in_pos(row, 1, question)?;
    io.write(FormattedString::from(" [y/N]").fg(BLUE))?;

    match io.get_char()? {
        Key::Char('y') | Key::Char('Y') => Ok(true),
        _ => Ok(false),
    }
}

fn handle_user_input<'a>(io: &mut IO<'a>, db: &mut Database) -> Result<()> {
    io.clear_screen()?;
    io.hide_cursor()?;
    let mut context = Context::Project(HEADER_OFFSET + 1, db.project_count());
    let mut project_context = Context::Project(HEADER_OFFSET + 1, db.project_count());
    let (terminal_width, terminal_height) = termion::terminal_size()?;

    loop {
        io.clear_screen()?;

        match context {
            Context::Project(focused_row, _) => {
                io.write_in_pos(1, 1, numbered_row(0, 3, &Project::header()))?;
                io.write_in_pos(2, 1, numbered_row(1, 4, &context.pane_div(terminal_width)))?;
                for (i, project) in db.projects().enumerate() {
                    let row = i as u16 + HEADER_OFFSET + 1;
                    io.write_in_pos(row, 1, numbered_row(row, focused_row, project))?
                }
            }
            Context::Task(focused_row, _) => {
                io.write_in_pos(1, 1, numbered_row(0, 3, &Task::header()))?;
                io.write_in_pos(2, 1, numbered_row(1, 4, &context.pane_div(terminal_width)))?;
                for (i, task) in db.tasks(project_context.idx()).enumerate() {
                    let row = i as u16 + HEADER_OFFSET + 1;
                    io.write_in_pos(row, 1, numbered_row(row, focused_row, task))?
                }
            }
        }

        match io.get_char()? {
            Key::Char('q') => break,
            Key::Char('j') | Key::Down => context = context.jump(1).unwrap_or(context),
            Key::Char('k') | Key::Up => context = context.jump(-1).unwrap_or(context),
            Key::Char('g') => context = context.jump_to(0).unwrap_or(context),
            Key::Char('G') => context = context.jump_to(context.length() - 1).unwrap_or(context),
            Key::Char(c @ 'J') | Key::Char(c @ 'K') => {
                context = swap_rows(context, project_context.idx(), c, db)?;
            }
            Key::Char('\n') => enter_context(&mut context, &mut project_context, db),
            Key::Esc => leave_context(&mut context, &mut project_context),
            Key::Char(change @ '>') | Key::Char(change @ '<') => {
                context = change_status(context, project_context.idx(), db, change)?;
            }
            Key::Char('-') => match confirm_deletion(terminal_height, io)? {
                true => context = delete_row(context, project_context, db)?,
                _ => {}
            },
            Key::Char('+') => context = add_row(context, project_context, terminal_height, db, io)?,
            _ => {}
        }
    }
    io.clear_screen()?;
    io.show_cursor()?;
    Ok(())
}

fn swap_rows(context: Context, project: usize, cmd: char, db: &mut Database) -> Result<Context> {
    if let Some(next_context) = context.jump(if cmd == 'J' { 1 } else { -1 }) {
        match context {
            Context::Project(_, _) => db.swap_projects(context.idx(), next_context.idx())?,
            Context::Task(_, _) => db.swap_tasks(project, context.idx(), next_context.idx())?,
        }
        Ok(next_context)
    } else {
        Ok(context)
    }
}

fn get_input_line<'a>(io: &mut IO<'a>, row: u16) -> Result<Option<String>> {
    let mut description = String::from("");
    let mut result = Ok(None);

    io.show_cursor()?;

    loop {
        let c = io.get_char();

        match c? {
            Key::Esc => break,
            Key::Char('\n') => {
                result = Ok(Some(description));
                break;
            }
            Key::Backspace => {
                if let Some(_) = description.pop() {
                    io.erase(row, 4 + description.chars().count() as u16)?
                }
            }
            Key::Char(c) => {
                description.push(c);
                io.write_in_pos(row, 3 + description.chars().count() as u16, c)?
            }
            _ => {}
        }
    }

    io.hide_cursor()?;
    result
}

fn enter_context(context: &mut Context, project_context: &mut Context, db: &Database) {
    if let Context::Project(_, _) = context {
        *project_context = *context;
        *context = Context::Task(HEADER_OFFSET + 1, db.task_count(project_context.idx()));
    }
}

fn leave_context(context: &mut Context, project_context: &mut Context) {
    if let Context::Task(_, _) = context {
        *context = *project_context
    }
}

fn add_row<'a>(
    context: Context,
    project_context: Context,
    terminal_height: u16,
    db: &mut Database,
    io: &mut IO<'a>,
) -> Result<Context> {
    io.write_in_pos(terminal_height, 1, FormattedString::from("-> ").fg(PINK))?;
    let description = get_input_line(io, terminal_height)?;

    if let Some(description) = description {
        match context {
            Context::Task(_, size) => {
                let task = Task::new(description);
                let task_index = db.add_task(project_context.idx(), task)?.unwrap() as u16;
                Ok(Context::Task(task_index + HEADER_OFFSET + 1, size + 1))
            }
            Context::Project(_, size) => {
                db.add_project(Project::new(description))?;
                Ok(Context::Project(size + HEADER_OFFSET + 1, size + 1))
            }
        }
    } else {
        Ok(context)
    }
}

fn delete_row(context: Context, project_context: Context, db: &mut Database) -> Result<Context> {
    match context.drop() {
        Some(new_context @ Context::Project(_, _)) => {
            db.remove_project(context.idx())?;
            Ok(new_context)
        }
        Some(new_context @ Context::Task(_, _)) => {
            db.remove_task(project_context.idx(), context.idx())?;
            Ok(new_context)
        }
        None => Ok(context),
    }
}

fn change_status(context: Context, project: usize, db: &mut Database, c: char) -> Result<Context> {
    if let Context::Task(_, len) = context {
        let current_state = db.task_state(project, context.idx());
        let next_state = match c {
            '>' => current_state.next(),
            _ => current_state.previous(),
        };

        match db.set_task_state(project, context.idx(), next_state)? {
            Some(new_index) => Ok(Context::Task(new_index as u16 + HEADER_OFFSET + 1, len)),
            None => Ok(context),
        }
    } else {
        Ok(context)
    }
}
