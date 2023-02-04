use super::term::*;

use std::cell::RefCell;
use std::cmp::{max, min};
use std::io::Write;
use std::ops::{Add, Div, Mul};
use std::rc::Rc;

use termion::{color, terminal_size};

const CURSOR_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::White);

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
    buffer: String
}

impl<W: Write> Drop for UI<W> {
    fn drop(&mut self) {
        let mut b = String::new();
        b.push_str(&term_show_cursor_str());
        b.push_str(&term_reset_str());
        term_write(&mut self.stdout, &b);
        self.stdout.flush().unwrap();
    }
}

impl<W: Write> UI<W> {
    pub fn new(mut stdout: W) -> Self {
        stdout.flush().unwrap();
        let mut b = String::new();
        b.push_str(&term_reset_str());
        b.push_str(&term_hide_cursor_str());
        Self {
            stdout,
            stack: Vec::new(),
            buffer: b,
        }
    }

    pub fn begin(&mut self, pos: Vec2, kind: LayoutKind) {
        assert!(self.stack.is_empty());
        self.stdout.flush().unwrap();

        let (w, h) = terminal_size().unwrap_or((80, 24));
        let root = Box::new(Layout::new(kind, pos, Vec2::new(w, h)));

        self.stack.push(Rc::new(RefCell::new(root)));

        self.buffer.push_str(&term_goto_str((pos.y, pos.x)));
        self.buffer.push_str(&term_clear_afer_cursor_str());
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

        let text = "‾".repeat(layout.borrow().max_size.x as usize);

        self.label(&text);
    }

    pub fn label(&mut self, text: &str) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render label outside of any layout");
        let pos = layout.borrow().available_pos();

        self.buffer.push_str(&term_goto_str((pos.y, pos.x)));
        self.buffer.push_str(text);

        let space_fill = " ".repeat(if layout.borrow().max_size.x as usize > text.len() {
            layout.borrow().max_size.x as usize - text.len()
        } else {
            0
        });
        self.buffer.push_str(&space_fill);

        layout
            .borrow_mut()
            .add_widget(Vec2::new(text.len() as u16, 1));
    }

    pub fn label_styled(&mut self, text: &str, pair: (&dyn color::Color, &dyn color::Color)) {
        self.buffer.push_str(&term_set_color_str( pair.0, Some(pair.1)));
        self.label(text);
        self.buffer.push_str(&term_style_reset_str());
    }

    pub fn edit_label(&mut self, text: &String, cur: usize, prefix: String) {
        let layout = self
            .stack
            .last_mut()
            .expect("Tried to render edit mode outside of any layout");
        let pos = layout.borrow().available_pos();

        // Buffer
        {
            self.buffer.push_str(&term_goto_str((pos.y, pos.x)));
            self.buffer.push_str(&format!("{prefix}{text}"));
            layout
                .borrow_mut()
                .add_widget(Vec2::new(text.len() as u16, 1));
        }

        // Cursor
        {
            self.buffer.push_str(&term_goto_str((
                pos.y,
                pos.x + cur as u16 + prefix.len() as u16,
            )));
            self.buffer.push_str(&term_set_color_str(CURSOR_PAIR.0, Some(CURSOR_PAIR.1)));
            self.buffer.push_str(&text.get(cur..=cur).unwrap_or(" "));
            self.buffer.push_str(&term_style_reset_str());
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
         
        term_write(&mut self.stdout, &self.buffer);
        self.stdout.flush().unwrap();
        self.buffer.clear();
    }
}
