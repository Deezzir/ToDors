extern crate termion;

use std::cmp::*;
use std::io::{stdin, stdout, Read, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use termion::{clear, color, cursor, style};

type Id = usize;

struct UI<'a> {
    cur_id: Option<Id>,
    screen: &'a mut AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    row: u16,
    col: u16,
}

impl<'a> UI<'a> {
    fn new(screen: &'a mut AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>) -> Self {
        UI {
            screen,
            row: 0,
            col: 0,
            cur_id: None,
        }
    }

    fn begin(
        &mut self,
        row: u16,
        col: u16,
    ) {
        self.row = row;
        self.col = col;

        write!(
            self.screen,
            "{}{}{}",
            cursor::Hide,
            clear::All,
            cursor::Goto(1, 1)
        )
        .unwrap();
    }

    fn end(&mut self) {
        self.screen.flush().unwrap();
    }

    fn label(&mut self, text: &str) {
        screen_write(self.screen, text, self.row, self.col);
        self.row += 1;
    }

    fn begin_list(&mut self, id: Id) {
        assert!(self.cur_id.is_none(), "Nested lists are not allowed");
        self.cur_id = Some(id);
    }

    fn list_item(&mut self, text: &str, id: Id) {
        let cur_id= self.cur_id.expect("List item must be inside a list");
        if cur_id == id {
            screen_set_style(self.screen, (&color::Black, &color::White));
        }
        self.label(text);
        screen_style_reset(self.screen);
    }

    fn end_list(&mut self) {
        self.cur_id = None;
    }
}

fn screen_set_style<W: Write>(
    s: &mut AlternateScreen<W>,
    pair: (&dyn color::Color, &dyn color::Color),
) {
    write!(s, "{}{}", color::Fg(pair.0), color::Bg(pair.1)).unwrap();
}

fn screen_write<W: Write>(s: &mut AlternateScreen<W>, text: &str, row: u16, col: u16) {
    write!(s, "{}{}", cursor::Goto(col, (row + 1) as u16), text).unwrap();
}

fn screen_style_reset<W: Write>(s: &mut AlternateScreen<W>) {
    write!(s, "{}", style::Reset).unwrap();
}

fn main() {
    let mut stdin = stdin();
    let mut screen = stdout()
        .into_raw_mode()
        .unwrap()
        .into_alternate_screen()
        .unwrap();
    screen.flush().unwrap();
    
    let mut ui: UI = UI::new(&mut screen);
    let mut cur_todo: Id = 0;
    let cur_done: Id = 0;
    let mut quit: bool = false;

    let todos: Vec<String> = vec![
        "Finish Scancore".to_string(),
        "Make a cup of tea".to_string(),
        "Write a Rust TODO app".to_string(),
    ];
    let dones: Vec<String> = vec![
        "Pet a cat".to_string(),
        "Have a lunch".to_string(),
    ];

    while !quit {
        ui.begin( 0, 0);
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
            ui.begin_list(cur_done,);
            for (id, done) in dones.iter().enumerate() {
                ui.list_item(&format!("- [X] {}", done), id);
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
                Key::Down | Key::Char('s') => cur_todo = min(cur_todo + 1, todos.len() - 1),
                _ => {}
            }
        }
    }

    screen.flush().unwrap();
    write!(screen, "{}{}", cursor::Show, style::Reset).unwrap();
}
