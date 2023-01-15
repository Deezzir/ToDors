extern crate termion;

use std::cmp::*;
use std::io::{stdin, stdout, Read, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, style};

type Id = usize;

struct UI {
    cur_id: Option<Id>,
    term: termion::raw::RawTerminal<std::io::Stdout>,
    row: u16,
    col: u16,
}

impl UI {
    fn new() -> UI {
        let mut term = stdout().into_raw_mode().unwrap();
        term_reset(&mut term);
        UI {
            cur_id: None,
            term,
            row: 0,
            col: 0,
        }
    }

    fn clear(&mut self) {
        term_reset(&mut self.term);
    }

    fn begin(&mut self, row: u16, col: u16) {
        self.row = row;
        self.col = col;
        write!(
            self.term,
            "{}{}{}",
            cursor::Hide,
            clear::All,
            cursor::Goto(1, 1)
        )
        .unwrap();
    }

    fn end(&mut self) {
        self.term.flush().unwrap();
    }

    fn label(&mut self, text: &str) {
        term_goto(&mut self.term, (self.row + 1, self.col));
        term_write(&mut self.term, text);
        self.row += 1;
    }

    fn begin_list(&mut self, id: Id) {
        assert!(self.cur_id.is_none(), "Nested lists are not allowed");
        self.cur_id = Some(id);
    }

    fn list_item(&mut self, text: &str, id: Id) {
        let cur_id = self.cur_id.expect("List item must be inside a list");
        if cur_id == id {
            term_set_style(&mut self.term, (&color::Black, &color::White));
        }
        self.label(text);
        term_style_reset(&mut self.term);
    }

    fn end_list(&mut self) {
        self.cur_id = None;
    }
}

fn term_set_style<W: Write>(s: &mut W, pair: (&dyn color::Color, &dyn color::Color)) {
    write!(s, "{}{}", color::Fg(pair.0), color::Bg(pair.1)).unwrap();
}

fn term_goto<W: Write>(s: &mut W, pos: (u16, u16)) {
    write!(s, "{}", cursor::Goto(pos.1, pos.0)).unwrap();
}

fn term_write<W: Write>(s: &mut W, text: &str) {
    write!(s, "{}", text).unwrap();
}

fn term_style_reset<W: Write>(s: &mut W) {
    write!(s, "{}", style::Reset).unwrap();
}

fn term_reset<W: Write>(s: &mut W) {
    term_goto(s, (1, 1));
    write!(s, "{}{}{}", clear::All, cursor::Show, style::Reset).unwrap();
    s.flush().unwrap();
}

fn main() {
    let mut stdin = stdin();
    let mut ui: UI = UI::new();
    let mut cur_todo: Id = 0;
    let _cur_done: Id = 0;
    let mut quit: bool = false;

    let mut todos: Vec<String> = vec![
        "Finish Scancore".to_string(),
        "Make a cup of tea".to_string(),
        "Write a Rust TODO app".to_string(),
    ];
    let mut dones: Vec<String> = vec!["Pet a cat".to_string(), "Have a lunch".to_string()];

    while !quit {
        ui.begin(0, 0);
        {
            ui.label("TODO:");
            ui.label("------------------------------");
            ui.begin_list(cur_todo);
            for (id, todo) in todos.iter().enumerate() {
                ui.list_item(&format!("- [ ] {}", todo), id);
            }
            ui.end_list();

            ui.label("");
            ui.label("DONE:");
            ui.label("------------------------------");
            ui.begin_list(0);
            for (id, done) in dones.iter().enumerate() {
                ui.list_item(&format!("- [X] {}", done), id + 1);
            }
            ui.end_list();
        }
        ui.end();

        if let Some(Ok(key)) = stdin.by_ref().keys().next() {
            match key {
                Key::Esc | Key::Char('q') => quit = true,
                Key::Up | Key::Char('w') => {
                    if cur_todo > 0 {
                        cur_todo -= 1;
                    }
                }
                Key::Down | Key::Char('s') => {
                    if todos.len() > 0 {
                        cur_todo = min(cur_todo + 1, todos.len() - 1)
                    }
                }
                Key::Char('\n') => {
                    if cur_todo < todos.len() {
                        dones.push(todos.remove(cur_todo));
                        if todos.len() > 0 {
                            cur_todo = min(cur_todo, todos.len() - 1);
                        } else {
                            cur_todo = 0;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    ui.clear();
}
