#![feature(box_syntax, box_patterns)]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod formatted_string;
mod io;
mod project;

use crate::formatted_string::FormattedString;
use crate::io::IO;
use crate::project::Listable;
use crate::project::{Event, Project, State, Task};

use chrono::prelude::Utc;
use std::cmp::max;
use std::cmp::min;
use std::fs;
use std::io::{stdin, stdout};
use std::io::{Error, ErrorKind, Result};
use termion::color;
use termion::color::Fg;
use termion::color::Rgb;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

const HEADER_OFFSET: u16 = 2;
const DIV_COLOR: Fg<Rgb> = Fg(Rgb(0, 150, 230));
const YELLOW: Rgb = Rgb(241, 196, 15);
const PINK: Rgb = Rgb(200, 0, 150);
const BLUE: Rgb = Rgb(52, 152, 219);

fn main() -> Result<()> {
    let json_data = load_database()?;
    let mut projects: Vec<Project> = serde_json::from_str(json_data.as_str())?;

    let mut io = IO {
        input: &mut stdin(),
        output: &mut AlternateScreen::from(stdout().into_raw_mode().unwrap()),
    };

    handle_user_input(&mut io, &mut projects)
}

fn database_file() -> Result<String> {
    match dirs::home_dir() {
        Some(path) => Ok(format!(
            "{}{}",
            path.to_str().unwrap(),
            "/.tasks/projects.json"
        )),
        None => Err(Error::new(
            ErrorKind::Other,
            "Couldn't resolve your home directory",
        )),
    }
}

fn load_database() -> Result<String> {
    fs::read_to_string(database_file()?)
}

fn save_database(projects: &Vec<Project>) -> Result<()> {
    let content = serde_json::to_string(projects)?;
    fs::write(database_file()?, content)
}

#[derive(Copy, Clone)]
enum Context {
    Project(u16, u16),
    Task(u16, u16),
}

impl Context {
    fn idx(self) -> usize {
        match self {
            Context::Project(index, _) => (index - HEADER_OFFSET - 1) as usize,
            Context::Task(index, _) => (index - HEADER_OFFSET - 1) as usize,
        }
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

fn handle_user_input<'a>(io: &mut IO<'a>, projects: &mut Vec<Project>) -> Result<()> {
    io.clear_screen()?;
    io.hide_cursor()?;
    let mut context = Context::Project(HEADER_OFFSET + 1, projects.len() as u16);
    let mut project_context = Context::Project(HEADER_OFFSET + 1, projects.len() as u16);
    let (terminal_width, terminal_height) = termion::terminal_size()?;

    loop {
        io.clear_screen()?;

        match context {
            Context::Project(focused_row, _) => {
                io.write_in_pos(1, 1, numbered_row(0, 3, &Project::header()))?;
                io.write_in_pos(2, 1, numbered_row(1, 4, &pane_div(terminal_width)))?;
                for (i, project) in projects.iter().enumerate() {
                    let row = i as u16 + HEADER_OFFSET + 1;
                    io.write_in_pos(row, 1, numbered_row(row, focused_row, project))?
                }
            }
            Context::Task(focused_row, _) => {
                projects[project_context.idx()].sort_tasks();

                io.write_in_pos(1, 1, numbered_row(0, 3, &Task::header()))?;
                io.write_in_pos(2, 1, numbered_row(1, 4, &pane_div(terminal_width)))?;
                for (i, task) in projects[project_context.idx()].tasks.iter().enumerate() {
                    let row = i as u16 + HEADER_OFFSET + 1;
                    io.write_in_pos(row, 1, numbered_row(row, focused_row, task))?
                }
            }
        }

        match io.get_char()? {
            Key::Char('q') => break,
            Key::Char('j') | Key::Down => context = next_line(context),
            Key::Char('k') | Key::Up => context = previous_line(context),
            Key::Char('g') => context = first_line(context),
            Key::Char('G') => context = last_line(context),
            Key::Char(c @ 'J') | Key::Char(c @ 'K') => {
                context = swap_rows(context, project_context, c, projects)?;
            }
            Key::Char('\n') => enter_context(&mut context, &mut project_context, &projects),
            Key::Esc => leave_context(&mut context, &mut project_context),
            Key::Char(change @ '>') | Key::Char(change @ '<') => {
                context = change_status(context, project_context, projects, change)?;
            }
            Key::Char('-') => match confirm_deletion(terminal_height, io)? {
                true => context = delete_current_line(context, project_context, projects)?,
                _ => {}
            },
            Key::Char('+') => {
                context = add_line(context, project_context, terminal_height, projects, io)?
            }
            _ => {}
        }
    }
    io.clear_screen()?;
    io.show_cursor()?;
    Ok(())
}

fn swap_rows(
    context: Context,
    project_context: Context,
    command: char,
    projects: &mut Vec<Project>,
) -> Result<Context> {
    let neighbor: i16 = context.idx() as i16 + if command == 'J' { 1 } else { -1 };
    match context {
        Context::Project(_, len) => {
            if neighbor >= 0 && neighbor < len as i16 {
                projects.swap(context.idx(), neighbor as usize);
                save_database(projects)?;
                Ok(Context::Project(neighbor as u16 + HEADER_OFFSET + 1, len))
            } else {
                Ok(context)
            }
        }
        Context::Task(_, len) => {
            if neighbor >= 0 && neighbor < len as i16 {
                projects[project_context.idx()]
                    .tasks
                    .swap(context.idx(), neighbor as usize);
                save_database(projects)?;
                Ok(Context::Task(neighbor as u16 + HEADER_OFFSET + 1, len))
            } else {
                Ok(context)
            }
        }
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

fn pane_div(width: u16) -> String {
    format!(
        "{}{}{}",
        DIV_COLOR,
        (0..width - 3).map(|_| "-").collect::<String>(),
        Fg(color::Reset),
    )
}

fn next_line(context: Context) -> Context {
    match context {
        Context::Project(line, len) => Context::Project(min(len + HEADER_OFFSET, line + 1), len),
        Context::Task(line, len) => Context::Task(min(len + HEADER_OFFSET, line + 1), len),
    }
}

fn previous_line(context: Context) -> Context {
    match context {
        Context::Project(line, len) => Context::Project(max(HEADER_OFFSET + 1, line - 1), len),
        Context::Task(line, len) => Context::Task(max(HEADER_OFFSET + 1, line - 1), len),
    }
}

fn first_line(context: Context) -> Context {
    match context {
        Context::Project(_, len) => Context::Project(HEADER_OFFSET + 1, len),
        Context::Task(_, len) => Context::Task(HEADER_OFFSET + 1, len),
    }
}

fn last_line(context: Context) -> Context {
    match context {
        Context::Project(_, len) => Context::Project(HEADER_OFFSET + len, len),
        Context::Task(_, len) => Context::Task(HEADER_OFFSET + len, len),
    }
}

fn enter_context(context: &mut Context, project_context: &mut Context, projects: &Vec<Project>) {
    if let Context::Project(_, _) = context {
        *project_context = *context;
        *context = Context::Task(
            HEADER_OFFSET + 1,
            projects[context.idx()].tasks.len() as u16,
        );
    }
}

fn leave_context(context: &mut Context, project_context: &mut Context) {
    if let Context::Task(_, _) = context {
        *context = *project_context
    }
}

fn add_line<'a>(
    context: Context,
    project_context: Context,
    terminal_height: u16,
    projects: &mut Vec<Project>,
    io: &mut IO<'a>,
) -> Result<Context> {
    io.write_in_pos(terminal_height, 1, FormattedString::from("-> ").fg(PINK))?;
    let description = get_input_line(io, terminal_height)?;

    if let Some(description) = description {
        match context {
            Context::Task(_, size) => {
                projects[project_context.idx()]
                    .tasks
                    .push(Task::new(description));
                save_database(projects)?;
                Ok(Context::Task(size + HEADER_OFFSET + 1, size + 1))
            }
            Context::Project(_, size) => {
                projects.push(Project::new(description));
                save_database(projects)?;
                Ok(Context::Project(size + HEADER_OFFSET + 1, size + 1))
            }
        }
    } else {
        Ok(context)
    }
}

fn delete_current_line(
    context: Context,
    project_context: Context,
    projects: &mut Vec<Project>,
) -> Result<Context> {
    match context {
        Context::Project(line, length) => {
            if length > 0 {
                projects.remove(context.idx());
                save_database(projects)?;
                Ok(Context::Project(line - 1, length - 1))
            } else {
                Ok(context)
            }
        }
        Context::Task(line, length) => {
            if length > 0 {
                projects[project_context.idx()].tasks.remove(context.idx());
                save_database(projects)?;
                Ok(Context::Task(line - 1, length - 1))
            } else {
                Ok(context)
            }
        }
    }
}

fn change_status(
    context: Context,
    project_context: Context,
    projects: &mut Vec<Project>,
    change: char,
) -> Result<Context> {
    if let Context::Task(_, len) = context {
        let task = &mut projects[project_context.idx()].tasks[context.idx()];
        let task_id = task.id.clone();
        let current_state = task.state();
        let next_state = match change {
            '>' => match current_state {
                State::TODO => State::ONGOING,
                State::ONGOING => State::DONE,
                s @ _ => s,
            },
            '<' => match current_state {
                State::DONE => State::ONGOING,
                State::ONGOING => State::TODO,
                s @ _ => s,
            },
            _ => State::TODO, // Rust doesn't understand that we already have exhaustive check
        };

        if next_state != current_state {
            task.events.push(Event::State {
                data: next_state,
                date_time: Utc::now(),
            });
            projects[project_context.idx()].sort_tasks();
            save_database(projects)?;

            if let Some(index) = projects[project_context.idx()].find_task_index(task_id) {
                Ok(Context::Task(index as u16 + HEADER_OFFSET + 1, len))
            } else {
                Ok(context)
            }
        } else {
            Ok(context)
        }
    } else {
        Ok(context)
    }
}
