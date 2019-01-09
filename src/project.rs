use chrono::prelude::DateTime;
use chrono::prelude::Local;
use chrono::prelude::Utc;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use termion::color::Rgb;
use uuid::Uuid;

use crate::formatted_string::FormattedString;

#[derive(PartialEq, Serialize, Deserialize, Copy, Clone, Debug)]
pub enum State {
    TODO,
    ONGOING,
    DONE,
}

static RED: Rgb = Rgb(192, 57, 43);
static YELLOW: Rgb = Rgb(241, 196, 15);
static GREEN: Rgb = Rgb(46, 204, 113);
static PINK: Rgb = Rgb(200, 0, 150);
static BLUE: Rgb = Rgb(52, 152, 219);

fn div() -> FormattedString {
    FormattedString::from("|").fg(BLUE)
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = match self {
            State::TODO => FormattedString::from("TODO").right(7).fg(RED),
            State::ONGOING => FormattedString::from("ONGOING").fg(YELLOW),
            State::DONE => FormattedString::from("DONE").right(7).fg(GREEN),
        };

        write!(f, "{}", name)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
    Description {
        data: String,
        date_time: DateTime<Utc>,
    },
    State {
        data: State,
        date_time: DateTime<Utc>,
    },
    Comment {
        data: String,
        date_time: DateTime<Utc>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub id: String,
    pub description: String,
    pub notes: String,
    pub tasks: Vec<Task>,
}

impl Project {
    pub fn new(description: String) -> Project {
        Project {
            id: Uuid::new_v4().to_string(),
            description: description,
            notes: String::from(""),
            tasks: vec![],
        }
    }

    pub fn header() -> String {
        format!(
            "{div_left}{tasks}{div}{desc}",
            tasks = "Tasks",
            desc = FormattedString::from("Description").left(79),
            div_left = div().left(2),
            div = div().center(3)
        )
    }
}

impl Task {
    pub fn new(description: String) -> Task {
        Task {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            events: vec![
                Event::State {
                    data: State::TODO,
                    date_time: Utc::now(),
                },
                Event::Description {
                    data: description,
                    date_time: Utc::now(),
                },
            ],
        }
    }

    pub fn state(&self) -> State {
        let mut state = State::TODO;
        for event in self.events.iter() {
            if let Event::State { data, date_time: _ } = event {
                state = data.clone()
            }
        }
        state
    }

    fn description(&self) -> String {
        let mut description = String::from("");
        for event in self.events.iter() {
            if let Event::Description { data, date_time: _ } = event {
                description = data.clone()
            }
        }
        description
    }

    fn created_at(&self) -> FormattedString {
        let date = self
            .created_at
            .with_timezone(&Local::now().timezone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        FormattedString::from(&date).fg(PINK)
    }

    pub fn header() -> String {
        let desc_width = termion::terminal_size().unwrap().0 as usize - 37;
        format!(
            "{div_left}{state}{div}{desc}{div}{date}",
            state = FormattedString::from("State").center(7),
            desc = FormattedString::from("Description").left(desc_width),
            date = "Created At",
            div_left = div().left(2),
            div = div().center(3),
        )
    }
}

pub trait Listable {
    fn view(&self) -> String;
}

impl Listable for String {
    fn view(&self) -> String {
        self.clone()
    }
}

impl Listable for &str {
    fn view(&self) -> String {
        self.to_string()
    }
}

impl Listable for Task {
    fn view(&self) -> String {
        let desc_width = termion::terminal_size().unwrap().0 as usize - 37;
        format!(
            "{div_left}{state}{div}{desc}{div}{date}",
            state = self.state(),
            desc = FormattedString::from(&self.description()).left(desc_width),
            date = self.created_at(),
            div_left = div().left(2),
            div = div().center(3),
        )
    }
}

impl Listable for Project {
    fn view(&self) -> String {
        let desc_width = termion::terminal_size().unwrap().0 as usize - 13;
        format!(
            "{div_left}{tasks}{div}{desc}",
            tasks = FormattedString::from(&self.tasks.len().to_string()).center(5),
            desc = FormattedString::from(&self.description).left(desc_width),
            div_left = div().left(2),
            div = div().center(3)
        )
    }
}
