use super::term::*;

use std::cell::RefCell;
use std::cmp::max;
use std::io::{stdout, Write};
use std::ops::{Add, Div, Mul};
use std::rc::Rc;

use termion::raw::IntoRawMode;
use termion::{clear, color, terminal_size};

use crate::HIGHLIGHT_PAIR;

type LayoutRef = Rc<RefCell<Box<Layout>>>;

pub enum LayoutKind {
    Vert,
    Horz,
}

pub enum Style {
    Dash,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Vec2 {
    x: u16,
    y: u16,
}

impl Vec2 {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Mul for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Div for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

struct Layout {
    kind: LayoutKind,
    pos: Vec2,
    size: Vec2,
    max_size: Vec2,
    children: Vec<LayoutRef>,
}

impl Layout {
    fn new(kind: LayoutKind, pos: Vec2, max_size: Vec2) -> Self {
        Self {
            kind,
            pos,
            max_size,
            size: Vec2::default(),
            children: Vec::new(),
        }
    }

    fn available_pos(&self) -> Vec2 {
        match self.kind {
            LayoutKind::Horz => self.pos + self.size * Vec2::new(1, 0),
            LayoutKind::Vert => self.pos + self.size * Vec2::new(0, 1),
        }
    }

    fn available_size(&self) -> Vec2 {
        let div = self.children.len() as u16 + 1;
        match self.kind {
            LayoutKind::Horz => self.max_size / Vec2::new(div, 1),
            LayoutKind::Vert => self.max_size / Vec2::new(1, div),
        }
    }

    fn add_widget(&mut self, size: Vec2) {
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

    fn resize_chidren(&mut self, size: Vec2) {
        self.size = Vec2::default();

        for child in self.children.iter() {
            child.borrow_mut().resize_chidren(size);
        }

        for _ in 0..self.children.len() {
            self.add_widget(size);
        }
    }

    fn add_child(&mut self, kind: LayoutKind) -> &LayoutRef {
        let size = self.available_size();
        self.resize_chidren(size);

        let pos = self.available_pos();
        let child = Box::new(Layout::new(kind, pos, size));

        self.add_widget(size);
        self.children.push(Rc::new(RefCell::new(child)));

        self.children.last().unwrap()
    }
}

pub struct UI<W: Write> {
    term: W,
    stack: Vec<LayoutRef>,
}

impl UI<termion::raw::RawTerminal<std::io::Stdout>> {
    pub fn new() -> Self {
        let mut term = stdout().into_raw_mode().unwrap();
        term_reset(&mut term);
        term_hide_cursor(&mut term);
        term.flush().unwrap();
        Self {
            term,
            stack: Vec::new(),
        }
    }

    pub fn begin(&mut self, pos: Vec2, kind: LayoutKind) {
        assert!(self.stack.is_empty());

        let (w, h) = terminal_size().unwrap();
        let root = Box::new(Layout::new(kind, pos, Vec2::new(w, h)));

        self.stack.push(Rc::new(RefCell::new(root)));

        term_goto(&mut self.term, (pos.y, pos.x));
        term_write(&mut self.term, &format!("{}", clear::AfterCursor));
    }

    pub fn begin_layout(&mut self, kind: LayoutKind) {
        let child = {
            let layout = self
                .stack
                .last()
                .expect("Can't create a layout outside of UI::begin() and UI::end()");
            let mut layout = layout.borrow_mut();
            let child = layout.add_child(kind);
            Rc::clone(child)
        };

        self.stack.push(child);
    }

    pub fn br(&mut self) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render break line outside of any layout");
        let mut layout = layout.borrow_mut();
        layout.add_widget(Vec2::new(0, 1));
    }

    pub fn hl(&mut self, style: Style) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render horizontal line outside of any layout");

        let text = match style {
            Style::Dash => "─".repeat(layout.borrow().max_size.x as usize),
        };

        self.label(&text);
    }

    pub fn label(&mut self, text: &str) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render label outside of any layout");
        let pos = layout.borrow().available_pos();

        term_goto(&mut self.term, (pos.y, pos.x));
        term_write(&mut self.term, text);

        layout
            .borrow_mut()
            .add_widget(Vec2::new(text.len() as u16, 1));
    }

    pub fn label_styled(&mut self, text: &str, pair: (&dyn color::Color, &dyn color::Color)) {
        term_set_style(&mut self.term, pair);
        self.label(text);
        term_style_reset(&mut self.term);
    }

    pub fn edit_label(&mut self, text: &String, cur: usize, prefix: String) {
        let layout = self
            .stack
            .last_mut()
            .expect("Tried to render edit mode outside of any layout");
        let pos = layout.borrow().available_pos();

        // Buffer
        {
            term_goto(&mut self.term, (pos.y, pos.x));
            term_write(&mut self.term, &format!("{}{}", prefix, text));
            layout
                .borrow_mut()
                .add_widget(Vec2::new(text.len() as u16, 1));
        }

        // Cursor
        {
            term_goto(
                &mut self.term,
                (pos.y, pos.x + cur as u16 + prefix.len() as u16),
            );
            term_set_style(&mut self.term, HIGHLIGHT_PAIR);
            term_write(&mut self.term, text.get(cur..=cur).unwrap_or(" "));
            term_style_reset(&mut self.term);
        }
    }

    pub fn end_layout(&mut self) {
        self.stack
            .pop()
            .expect("Can't end a non-existing layout. Was there UI::begin_layout()?");
        self.stack.last_mut().expect(
            "Can't end a non-existing layout. Was there UI::begin_layout() or UI::begin()?",
        );
    }

    pub fn end(&mut self) {
        self.stack
            .pop()
            .expect("Can't end a non-existing UI. Was there UI::begin()?");
        self.term.flush().unwrap();
    }

    pub fn clear(&mut self) {
        term_show_cursor(&mut self.term);
        term_reset(&mut self.term);
        self.term.flush().unwrap();
    }
}
