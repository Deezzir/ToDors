use super::term::*;

use std::cell::RefCell;
use std::cmp::{max, min};
use std::io::Write;
use std::ops::{Add, Div, Mul};
use std::rc::Rc;

use termion::{clear, color, terminal_size};

use crate::HIGHLIGHT_PAIR;

type LayoutRef = Rc<RefCell<Box<Layout>>>;

pub enum LayoutKind {
    Vert,
    Horz,
}

#[allow(dead_code)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
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

    fn div_rem(self, rhs: Self) -> (Self, Self) {
        (
            Self {
                x: self.x / rhs.x,
                y: self.y / rhs.y,
            },
            Self {
                x: self.x % rhs.x,
                y: self.y % rhs.y,
            },
        )
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
        let child_size = self.available_size().0;

        match self.kind {
            LayoutKind::Horz => {
                let x = min(self.size.x, child_size.x);
                self.pos + Vec2::new(x, 0)
            }
            LayoutKind::Vert => {
                let y = min(self.size.y, child_size.y);
                self.pos + Vec2::new(0, y)
            }
        }
    }

    fn available_size(&self) -> (Vec2, Vec2) {
        let div = self.children.len() as u16 + 1;
        match self.kind {
            LayoutKind::Horz => self.max_size.div_rem(Vec2::new(div, 1)),
            LayoutKind::Vert => self.max_size.div_rem(Vec2::new(1, div)),
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

    fn resize(&mut self, size: Vec2) {
        let child_size = self.available_size().0;

        self.max_size = size;
        self.size.x = min(self.size.x, child_size.x);

        for child in &self.children {
            child.borrow_mut().resize(child_size);
        }
    }

    fn add_child(&mut self, child: LayoutRef) {
        let size = Vec2::new(child.borrow().max_size.x, child.borrow().size.y);

        self.resize(self.max_size);
        self.add_widget(size);
        self.children.push(child);
    }
}

pub struct UI<W: Write> {
    stdout: W,
    stack: Vec<LayoutRef>,
}

impl<W: Write> Drop for UI<W> {
    fn drop(&mut self) {
        term_show_cursor(&mut self.stdout);
        term_reset(&mut self.stdout);
        self.stdout.flush().unwrap();
    }
}

impl<W: Write> UI<W> {
    pub fn new(mut stdout: W) -> Self {
        term_reset(&mut stdout);
        term_hide_cursor(&mut stdout);
        stdout.flush().unwrap();
        Self {
            stdout,
            stack: Vec::new(),
        }
    }

    pub fn begin(&mut self, pos: Vec2, kind: LayoutKind) {
        assert!(self.stack.is_empty());

        let pos = Vec2::new(pos.x + 1, pos.y);

        let (w, h) = terminal_size().unwrap_or((80, 24));
        let root = Box::new(Layout::new(kind, pos, Vec2::new(w, h)));

        self.stack.push(Rc::new(RefCell::new(root)));

        term_goto(&mut self.stdout, (pos.y, pos.x));
        term_write(&mut self.stdout, &format!("{}", clear::AfterCursor));
    }

    pub fn begin_layout(&mut self, kind: LayoutKind) {
        let layout = self
            .stack
            .last()
            .expect("Can't create a layout outside of UI::begin() and UI::end()");
        let (max_size, rem) = layout.borrow().available_size();
        let child = Box::new(Layout::new(
            kind,
            layout.borrow().available_pos(),
            max_size + rem,
        ));

        self.stack.push(Rc::new(RefCell::new(child)));
    }

    pub fn br(&mut self) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render break line outside of any layout");
        let mut layout = layout.borrow_mut();
        layout.add_widget(Vec2::new(0, 1));
    }

    pub fn hl(&mut self) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render horizontal line outside of any layout");

        let text = "─".repeat(layout.borrow().max_size.x as usize - 1);

        self.label(&text);
    }

    pub fn label(&mut self, text: &str) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render label outside of any layout");
        let pos = layout.borrow().available_pos();

        term_goto(&mut self.stdout, (pos.y, pos.x));
        term_write(&mut self.stdout, text);

        layout
            .borrow_mut()
            .add_widget(Vec2::new(text.len() as u16, 1));
    }

    pub fn label_styled(&mut self, text: &str, pair: (&dyn color::Color, &dyn color::Color)) {
        term_set_style(&mut self.stdout, pair);
        self.label(text);
        term_style_reset(&mut self.stdout);
    }

    pub fn edit_label(&mut self, text: &String, cur: usize, prefix: String) {
        let layout = self
            .stack
            .last_mut()
            .expect("Tried to render edit mode outside of any layout");
        let pos = layout.borrow().available_pos();

        // Buffer
        {
            term_goto(&mut self.stdout, (pos.y, pos.x));
            term_write(&mut self.stdout, &format!("{prefix}{text}"));
            layout
                .borrow_mut()
                .add_widget(Vec2::new(text.len() as u16, 1));
        }

        // Cursor
        {
            term_goto(
                &mut self.stdout,
                (pos.y, pos.x + cur as u16 + prefix.len() as u16),
            );
            term_set_style(&mut self.stdout, HIGHLIGHT_PAIR);
            term_write(&mut self.stdout, text.get(cur..=cur).unwrap_or(" "));
            term_style_reset(&mut self.stdout);
        }
    }

    pub fn end_layout(&mut self) {
        let child = self
            .stack
            .pop()
            .expect("Can't end a non-existing layout. Was there UI::begin_layout()?");
        self.stack
            .last()
            .expect("Can't end a non-existing layout. Was there UI::begin_layout() or UI::begin()?")
            .borrow_mut()
            .add_child(Rc::clone(&child));
    }

    pub fn end(&mut self) {
        self.stack
            .pop()
            .expect("Can't end a non-existing UI. Was there UI::begin()?");
        self.stdout.flush().unwrap();
    }
}
