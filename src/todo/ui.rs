use super::term::*;

use std::cmp::max;
use std::io::{stdout, Write};
use std::ops::{Add, Mul};

use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::{clear, color};

pub const HIGHLIGHT_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::White);

pub enum LayoutKind {
    Vert,
    Horz,
}

#[derive(Default, Clone, Copy)]
pub struct Point {
    x: u16,
    y: u16,
}

impl Point {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Mul for Point {
    type Output = Point;
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

struct Layout {
    kind: LayoutKind,
    pos: Point,
    size: Point,
}

impl Layout {
    fn available_pos(&mut self) -> Point {
        match self.kind {
            LayoutKind::Horz => self.pos + self.size * Point::new(1, 0),
            LayoutKind::Vert => self.pos + self.size * Point::new(0, 1),
        }
    }

    fn add_widget(&mut self, size: Point) {
        match self.kind {
            LayoutKind::Horz => {
                self.size.x += size.x;
                self.size.y = max(self.size.y, size.y);
            }
            LayoutKind::Vert => {
                self.size.x = max(self.size.x, size.x);
                self.size.y += size.y;
            }
        }
    }
}

pub struct UI<W: Write> {
    term: W,
    layouts: Vec<Layout>,
    pub key: Option<Key>,
}

impl UI<termion::raw::RawTerminal<std::io::Stdout>> {
    pub fn new() -> UI<termion::raw::RawTerminal<std::io::Stdout>> {
        let mut term = stdout().into_raw_mode().unwrap();
        term_reset(&mut term);
        term_hide_cursor(&mut term);
        term.flush().unwrap();
        Self {
            term,
            layouts: Vec::<Layout>::new(),
            key: None,
        }
    }

    pub fn begin(&mut self, pos: Point, kind: LayoutKind) {
        assert!(self.layouts.is_empty());

        self.layouts.push(Layout {
            kind,
            pos,
            size: Point::new(0, 0),
        });

        term_goto(&mut self.term, (0, 0));
        term_write(&mut self.term, &format!("{}", clear::AfterCursor));
    }

    pub fn begin_layout(&mut self, kind: LayoutKind) {
        let layout = self
            .layouts
            .last_mut()
            .expect("Can't create a layout outside of UI::begin() and UI::end()");
        let pos = layout.available_pos();

        self.layouts.push(Layout {
            kind,
            pos,
            size: Point::new(0, 0),
        });
    }

    pub fn label(&mut self, text: &str) {
        let layout = self
            .layouts
            .last_mut()
            .expect("Tried to render label outside of any layout");
        let pos = layout.available_pos();

        term_goto(&mut self.term, (pos.y, pos.x));
        term_write(&mut self.term, text);

        layout.add_widget(Point::new(text.len() as u16, 1));
    }

    pub fn label_styled(&mut self, text: &str, pair: (&dyn color::Color, &dyn color::Color)) {
        term_set_style(&mut self.term, pair);
        self.label(text);
        term_style_reset(&mut self.term);
    }

    pub fn edit_field(&mut self, text: &mut String, cur: &mut usize, prefix: String) {
        let layout = self
            .layouts
            .last_mut()
            .expect("Tried to render outide of any layout");
        let pos = layout.available_pos();

        if *cur > text.len() {
            *cur = text.len();
        }

        if let Some(key) = self.key.take() {
            match key {
                Key::Left => {
                    if *cur > 0 {
                        *cur -= 1;
                    }
                },
                Key::Right => {
                    if *cur < text.len() {
                        *cur += 1;
                    }
                },
                Key::Backspace => {
                    if *cur > 0 {
                        *cur -= 1;
                        if *cur < text.len() {
                            text.remove(*cur);
                        }
                    }
                },
                Key::Delete => {
                    if *cur < text.len() {
                        text.remove(*cur);
                    }
                },
                Key::Home => *cur = 0,
                Key::End => *cur = text.len(),
                Key::Char(c) => {
                    let c = c as u8;
                    if c.is_ascii() && (32..127).contains(&c) {
                        if *cur > text.len() {
                            text.push(c as char);
                        } else {
                            text.insert(*cur, c as char);
                        }
                        *cur += 1;
                    } else {
                        self.key = Some(key)
                    }
                },
                _ => self.key = Some(key),
            }
        }
        
        // Buffer
        {
            term_goto(&mut self.term, (pos.y, pos.x));
            term_write(&mut self.term, &format!("{}{}", prefix, text));
            layout.add_widget(Point::new(text.len() as u16, 1));
        }

        // Cursor
        {
            term_goto(&mut self.term, (pos.y, pos.x + *cur as u16 + prefix.len() as u16));
            term_set_style(&mut self.term, HIGHLIGHT_PAIR);
            term_write(&mut self.term, text.get(*cur..=*cur).unwrap_or(" "));
            term_style_reset(&mut self.term);
        }
    }

    pub fn end_layout(&mut self) {
        let layout = self
            .layouts
            .pop()
            .expect("Can't end a non-existing layout. Was there UI:begin_layout()?");
        self.layouts
            .last_mut()
            .expect("Can't end a non-existing layout. Was there UI:begin_layout()?")
            .add_widget(layout.size);
    }

    pub fn end(&mut self) {
        self.layouts
            .pop()
            .expect("Can't end a non-existing UI. Was there UI:begin()?");
        self.term.flush().unwrap();
    }

    pub fn clear(&mut self) {
        term_show_cursor(&mut self.term);
        term_reset(&mut self.term);
        self.term.flush().unwrap();
    }
}
