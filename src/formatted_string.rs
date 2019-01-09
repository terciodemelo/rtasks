use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use termion::color;
use termion::color::AnsiValue;
use termion::color::Bg;
use termion::color::Fg;
use termion::color::Rgb;

#[derive(Clone, Debug)]
pub enum FormattedString {
    Raw(String),
    ColoredFg(Box<FormattedString>, Rgb),
    ColoredBg(Box<FormattedString>, AnsiValue),
    LeftAligned(Box<FormattedString>, usize),
    RightAligned(Box<FormattedString>, usize),
    CenterAligned(Box<FormattedString>, usize),
}

impl FormattedString {
    pub fn from(content: &str) -> FormattedString {
        FormattedString::Raw(content.to_string())
    }

    pub fn content(&self) -> &str {
        match self {
            FormattedString::Raw(content) => content,
            FormattedString::ColoredFg(box content, _) => content.content(),
            FormattedString::ColoredBg(box content, _) => content.content(),
            FormattedString::LeftAligned(box content, _) => content.content(),
            FormattedString::RightAligned(box content, _) => content.content(),
            FormattedString::CenterAligned(box content, _) => content.content(),
        }
    }

    pub fn fg(&self, color: Rgb) -> FormattedString {
        match self {
            FormattedString::Raw(_) => FormattedString::ColoredFg(box self.clone(), color),
            FormattedString::ColoredFg(box content, _) => {
                FormattedString::ColoredFg(box content.clone(), color)
            }
            s @ _ => FormattedString::ColoredFg(box s.clone(), color),
        }
    }

    pub fn bg(&self, color: AnsiValue) -> FormattedString {
        match self {
            FormattedString::Raw(_) => FormattedString::ColoredBg(box self.clone(), color),
            FormattedString::ColoredBg(box content, _) => {
                FormattedString::ColoredBg(box content.clone(), color)
            }
            s @ _ => FormattedString::ColoredBg(box s.clone(), color),
        }
    }

    pub fn focused(&self) -> FormattedString {
        let color = AnsiValue::grayscale(6);
        match self {
            FormattedString::Raw(_) => FormattedString::ColoredBg(box self.clone(), color),
            FormattedString::ColoredBg(box content, _) => {
                FormattedString::ColoredBg(box content.clone(), color)
            }
            s @ _ => FormattedString::ColoredBg(box s.clone(), color),
        }
    }

    pub fn left(&self, width: usize) -> FormattedString {
        match self {
            FormattedString::LeftAligned(box boxed, _) => {
                FormattedString::LeftAligned(box boxed.clone(), width)
            }
            FormattedString::ColoredFg(box boxed, color) => boxed.left(width).fg(*color),
            FormattedString::ColoredBg(box boxed, color) => boxed.left(width).bg(*color),
            FormattedString::Raw(_) => FormattedString::LeftAligned(box self.clone(), width),
            _ => FormattedString::from(self.content()).left(width),
        }
    }

    pub fn right(&self, width: usize) -> FormattedString {
        match self {
            FormattedString::RightAligned(box boxed, _) => {
                FormattedString::RightAligned(box boxed.clone(), width)
            }
            FormattedString::ColoredFg(box boxed, color) => boxed.right(width).fg(*color),
            FormattedString::ColoredBg(box boxed, color) => boxed.right(width).bg(*color),
            FormattedString::Raw(_) => FormattedString::RightAligned(box self.clone(), width),
            _ => FormattedString::from(self.content()).right(width),
        }
    }

    pub fn center(&self, width: usize) -> FormattedString {
        match self {
            FormattedString::CenterAligned(box boxed, _) => {
                FormattedString::CenterAligned(box boxed.clone(), width)
            }
            FormattedString::ColoredFg(box boxed, color) => boxed.center(width).fg(*color),
            FormattedString::ColoredBg(box boxed, color) => boxed.center(width).bg(*color),
            FormattedString::Raw(_) => FormattedString::CenterAligned(box self.clone(), width),
            _ => FormattedString::from(self.content()).center(width),
        }
    }
}

impl Display for FormattedString {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FormattedString::Raw(content) => write!(f, "{}", content),
            FormattedString::LeftAligned(box content, width) => {
                write!(f, "{:<width$}", content.to_string(), width = width)
            }
            FormattedString::RightAligned(box content, width) => {
                write!(f, "{:>width$}", content.to_string(), width = width)
            }
            FormattedString::CenterAligned(box content, width) => {
                write!(f, "{:^width$}", content.to_string(), width = width)
            }
            FormattedString::ColoredFg(box content, color) => write!(
                f,
                "{}{}{}",
                Fg(*color),
                content.to_string(),
                Fg(color::Reset)
            ),
            FormattedString::ColoredBg(box content, color) => write!(
                f,
                "{}{}{}",
                Bg(*color),
                content.to_string(),
                Bg(color::Reset)
            ),
        }
    }
}
