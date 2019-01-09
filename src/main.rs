#![feature(box_syntax, box_patterns)]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod formatted_string;
mod project;

use crate::formatted_string::FormattedString;
use crate::project::Listable;
use crate::project::{Event, Project, State, Task};

use chrono::prelude::Utc;
use std::cmp::max;
use std::cmp::min;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::io::{Error, ErrorKind, Result};
use termion::color;
use termion::color::Bg;
use termion::color::Fg;
use termion::color::Rgb;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;

const HEADER_OFFSET: u16 = 2;
const DIV_COLOR: Fg<Rgb> = Fg(Rgb(0, 150, 230));
const YELLOW: Rgb = Rgb(241, 196, 15);
const PINK: Rgb = Rgb(200, 0, 150);

fn main() -> Result<()> {
    let json_data = load_database()?;
    let mut projects: Vec<Project> = serde_json::from_str(json_data.as_str())?;
    let mut stdin = stdin();
    let stdout = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    handle_user_input(&mut stdin, stdout, &mut projects)
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

fn write_line(
    index: u16,
    current_line: u16,
    output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>,
    content: &Listable,
) -> Result<()> {
    let line_number = if index >= HEADER_OFFSET {
        (index - HEADER_OFFSET + 1).to_string()
    } else {
        " ".to_string()
    };

    let (cursor, formatted_content) = if index == current_line - 1 {
        (
            FormattedString::from(&line_number)
                .right(3)
                .fg(YELLOW)
                .focused(),
            FormattedString::from(&content.view()).focused(),
        )
    } else {
        (
            FormattedString::from(&line_number).right(3),
            FormattedString::from(&content.view()),
        )
    };

    write!(
        output,
        "{}{}{}{}",
        if index == 0 { "" } else { "\n" },
        termion::cursor::Goto(1, index + HEADER_OFFSET as u16),
        cursor,
        formatted_content,
    )
}

fn write_input_char(
    line: u16,
    column: u16,
    output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>,
    content: char,
) -> Result<()> {
    write!(output, "{}{}", termion::cursor::Goto(column, line), content)?;
    output.flush()
}

fn backspace(
    line: u16,
    column: u16,
    output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>,
) -> Result<()> {
    write!(
        output,
        "{}{}{}",
        termion::cursor::Goto(column, line),
        ' ',
        termion::cursor::Goto(column, line)
    )?;
    output.flush()
}

fn write_input_prompt(
    line: u16,
    output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>,
) -> Result<()> {
    write!(
        output,
        "{}{}",
        termion::cursor::Goto(1, line),
        FormattedString::from("-> ").fg(PINK)
    )?;
    output.flush()
}

fn clear_screen(output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>) -> Result<()> {
    write!(
        output,
        "{}{}{}",
        termion::cursor::Goto(1, 1),
        Bg(color::Reset),
        termion::clear::All
    )
}

fn handle_user_input(
    input: &mut std::io::Stdin,
    mut output: AlternateScreen<RawTerminal<std::io::Stdout>>,
    projects: &mut Vec<Project>,
) -> Result<()> {
    write!(output, "{}{}", termion::clear::All, termion::cursor::Hide)?;
    let mut context = Context::Project(HEADER_OFFSET + 1, projects.len() as u16);
    let mut project_context = Context::Project(HEADER_OFFSET + 1, projects.len() as u16);
    let (terminal_width, terminal_height) = termion::terminal_size()?;

    loop {
        clear_screen(&mut output)?;

        match context {
            Context::Project(current_line, _) => {
                write_line(0, 3, &mut output, &Project::header())?;
                write_line(1, 4, &mut output, &pane_div(terminal_width))?;
                for (i, project) in projects.iter().enumerate() {
                    write_line(i as u16 + HEADER_OFFSET, current_line, &mut output, project)?
                }
            }
            Context::Task(current_line, _) => {
                let project_idx = project_context.idx();

                write_line(0, 3, &mut output, &Task::header())?;
                write_line(1, 4, &mut output, &pane_div(terminal_width))?;
                for (i, task) in projects[project_idx].tasks.iter().enumerate() {
                    write_line(i as u16 + HEADER_OFFSET, current_line, &mut output, task)?
                }
            }
        }

        output.flush()?;

        match input.keys().next().unwrap()? {
            Key::Char('q') => break,
            Key::Char('j') | Key::Down => context = next_line(context),
            Key::Char('k') | Key::Up => context = previous_line(context),
            Key::Char(c @ 'J') | Key::Char(c @ 'K') => {
                context = swap_rows(context, project_context, c, projects)?;
            }
            Key::Char('\n') => enter_context(&mut context, &mut project_context, &projects),
            Key::Esc => leave_context(&mut context, &mut project_context),
            Key::Char(change @ '>') | Key::Char(change @ '<') => {
                change_status(context, project_context, projects, change)?;
            }
            Key::Char('-') => context = delete_current_line(context, project_context, projects)?,
            Key::Char('+') => {
                context = add_line(
                    context,
                    project_context,
                    terminal_height,
                    projects,
                    input,
                    &mut output,
                )?
            }
            _ => {}
        }
    }

    write!(output, "{}{}", termion::clear::All, termion::cursor::Show)?;
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

fn get_input_line(
    input: &mut std::io::Stdin,
    output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>,
    line: u16,
) -> Result<Option<String>> {
    let mut description = String::from("");
    let mut result = Ok(None);

    write!(output, "{}", termion::cursor::Show)?;

    for c in input.keys() {
        match c? {
            Key::Esc => break,
            Key::Char('\n') => {
                result = Ok(Some(description));
                break;
            }
            Key::Backspace => {
                if let Some(_) = description.pop() {
                    backspace(line, 4 + description.chars().count() as u16, output)?
                }
            }
            Key::Char(c) => {
                description.push(c);
                write_input_char(line, 3 + description.chars().count() as u16, output, c)?
            }
            _ => {}
        }
    }

    write!(output, "{}", termion::cursor::Hide)?;
    result
}

fn pane_div(width: u16) -> String {
    format!(
        "{}{}{}",
        DIV_COLOR,
        (0..width - 2).map(|_| "-").collect::<String>(),
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

fn add_line(
    context: Context,
    project_context: Context,
    terminal_height: u16,
    projects: &mut Vec<Project>,
    input: &mut std::io::Stdin,
    output: &mut AlternateScreen<RawTerminal<std::io::Stdout>>,
) -> Result<Context> {
    write_input_prompt(terminal_height, output)?;
    let description = get_input_line(input, output, terminal_height)?;

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
) -> Result<()> {
    if let Context::Task(_, _) = context {
        let task = &mut projects[project_context.idx()].tasks[context.idx()];
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
            _ => State::TODO, // Change
        };

        if next_state != current_state {
            task.events.push(Event::State {
                data: next_state,
                date_time: Utc::now(),
            });

            save_database(projects)
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}
