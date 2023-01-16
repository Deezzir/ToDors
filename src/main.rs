extern crate regex;
extern crate termion;

use std::cmp::min;
use std::env;
use std::fs::File;
use std::io::{self, stdin, stdout, BufRead, Read, Write};
use std::process;

use regex::Regex;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, style};

type Id = usize;
enum Status {
    Todo,
    Done,
}

struct UI {
    cur_id: Option<Id>,
    term: termion::raw::RawTerminal<std::io::Stdout>,
    row: u16,
    col: u16,
}

fn main() {
    let mut args = env::args();
    args.next().unwrap();

    let file_path = match args.next() {
        Some(file_path) => file_path,
        None => {
            eprintln!("Usage: todo <file>");
            eprintln!("[ERROR]: No file specified");
            process::exit(1);
        }
    };

    let mut quit: bool = false;
    let mut tab = Status::Todo;
    
    let mut todos: Vec<String> = Vec::<String>::new();
    let mut cur_todo: Id = 0;
    let mut dones: Vec<String> = Vec::<String>::new();
    let mut cur_done: Id = 0;

    parse_items(&file_path, &mut todos, &mut dones).unwrap();

    let mut stdin = stdin();
    let mut ui: UI = UI::new();

    while !quit {
        ui.begin(0, 0);
        {
            match tab {
                Status::Todo => {
                    ui.label("[TODO] DONE ");
                    ui.label("------------");
                    ui.begin_list(cur_todo);
                    for (id, todo) in todos.iter().enumerate() {
                        ui.list_item(&format!("- [ ] {}", todo), id);
                    }
                    ui.end_list();
                }
                Status::Done => {
                    ui.label(" TODO [DONE]");
                    ui.label("------------");
                    ui.begin_list(cur_done);
                    for (id, done) in dones.iter().enumerate() {
                        ui.list_item(&format!("- [X] {}", done), id);
                    }
                    ui.end_list();
                }
            }
        }
        ui.end();

        if let Some(Ok(key)) = stdin.by_ref().keys().next() {
            match key {
                Key::Esc | Key::Char('q') => quit = true,
                Key::Up | Key::Char('w') => match tab {
                    Status::Todo => list_up(&todos, &mut cur_todo),
                    Status::Done => list_up(&dones, &mut cur_done),
                },
                Key::Down | Key::Char('s') => match tab {
                    Status::Todo => list_down(&todos, &mut cur_todo),
                    Status::Done => list_down(&dones, &mut cur_done),
                },
                Key::Char('\n') => match tab {
                    Status::Todo => list_move(&mut todos, &mut dones, &mut cur_todo),
                    Status::Done => list_move(&mut dones, &mut todos, &mut cur_done),
                },
                Key::Char('\t') => {
                    tab = tab.togle();
                }
                _ => {}
            }
        }
    }

    ui.clear();
    dump_items(&file_path, &todos, &dones).unwrap();
}

impl Status {
    fn togle(&self) -> Status {
        match self {
            Status::Todo => Status::Done,
            Status::Done => Status::Todo,
        }
    }
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
            cursor::Goto(1, 1),
            clear::AfterCursor
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

fn list_up(list: &Vec<String>, cur: &mut Id) {
    if *cur > 0 && list.len() > 0 {
        *cur -= 1;
    }
}

fn list_down(list: &Vec<String>, cur: &mut Id) {
    if list.len() > 0 {
        *cur = min(*cur + 1, list.len() - 1)
    }
}

fn list_move(from: &mut Vec<String>, to: &mut Vec<String>, cur: &mut Id) {
    if *cur < from.len() {
        to.push(from.remove(*cur));
        if from.len() > 0 {
            *cur = min(*cur, from.len() - 1);
        } else {
            *cur = 0;
        }
    }
}

fn parse_items(
    file_path: &str,
    todos: &mut Vec<String>,
    dones: &mut Vec<String>,
) -> std::io::Result<()> {
    let file = File::open(file_path)?;
    let re_todo = Regex::new(r"^TODO: (.*)$").unwrap();
    let re_done = Regex::new(r"^DONE: (.*)$").unwrap();

    for (id, line) in io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if let Some(caps) = re_todo.captures(&line) {
            todos.push(caps[1].to_string());
        } else if let Some(caps) = re_done.captures(&line) {
            dones.push(caps[1].to_string());
        } else {
            eprintln!("[ERROR]: {}:{}: invalid format in the line", file_path, id + 1);
            process::exit(1);
        }
    }
    Ok(())
}

fn dump_items(file_path: &str, todos: &Vec<String>, dones: &Vec<String>) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
    Ok(())
}
