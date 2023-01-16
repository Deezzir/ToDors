extern crate regex;
extern crate termion;

use std::cmp::{max, min};
use std::env::args;
use std::fs::File;
use std::io::{self, stdin, stdout, BufRead, Read, Write};
use std::ops::{Add, Mul};
use std::process::exit;

use regex::Regex;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, style};

#[derive(PartialEq)]
enum Status {
    Todo,
    Done,
}

enum LayoutKind {
    Vert,
    Horz,
}

#[derive(Default, Clone, Copy)]
struct Point {
    x: u16,
    y: u16,
}

struct Layout {
    kind: LayoutKind,
    pos: Point,
    size: Point,
}

struct UI<W: Write> {
    term: W,
    layouts: Vec<Layout>,
}

const HIGHLIGHT_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::White);

fn main() {
    let mut args = args();
    args.next().unwrap();

    let file_path = match args.next() {
        Some(file_path) => file_path,
        None => {
            eprintln!("Usage: todo <file>");
            eprintln!("[ERROR]: No file specified");
            exit(1);
        }
    };

    let mut quit: bool = false;
    let mut tab = Status::Todo;

    let mut todos: Vec<String> = Vec::<String>::new();
    let mut cur_todo: usize = 0;
    let mut dones: Vec<String> = Vec::<String>::new();
    let mut cur_done: usize = 0;

    parse_items(&file_path, &mut todos, &mut dones).unwrap();

    let mut stdin = stdin();
    let mut ui: UI<termion::raw::RawTerminal<std::io::Stdout>> = UI::new();

    while !quit {
        ui.begin(Point::new(0, 0), LayoutKind::Horz);
        {
            ui.begin_layout(LayoutKind::Vert);
            {
                ui.label("TODO");
                ui.label("------------");
                for (id, todo) in todos.iter().enumerate() {
                    if id == cur_todo && tab == Status::Todo {
                        ui.label_styled(&format!("- [ ] {}", todo), HIGHLIGHT_PAIR);
                    } else {
                        ui.label(&format!("- [ ] {}", todo))
                    }
                }
            }
            ui.end_layout();

            ui.begin_layout(LayoutKind::Vert);
            {
                ui.label("DONE");
                ui.label("------------");
                for (id, done) in dones.iter().enumerate() {
                    if id == cur_done && tab == Status::Done {
                        ui.label_styled(&format!("- [X] {}", done), HIGHLIGHT_PAIR);
                    } else {
                        ui.label(&format!("- [X] {}", done))
                    }
                }
            }
            ui.end_layout();
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

impl Point {
    fn new(x: u16, y: u16) -> Self {
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

impl UI<termion::raw::RawTerminal<std::io::Stdout>> {
    fn new() -> UI<termion::raw::RawTerminal<std::io::Stdout>> {
        let mut term = stdout().into_raw_mode().unwrap();
        term_reset(&mut term);
        Self {
            term,
            layouts: Vec::<Layout>::new(),
        }
    }

    fn clear(&mut self) {
        term_reset(&mut self.term);
    }

    fn begin_layout(&mut self, kind: LayoutKind) {
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

    fn begin(&mut self, pos: Point, kind: LayoutKind) {
        assert!(self.layouts.is_empty());

        self.layouts.push(Layout {
            kind,
            pos,
            size: Point::new(0, 0),
        });

        term_write(
            &mut self.term,
            &format!(
                "{}{}{}",
                cursor::Hide,
                cursor::Goto(1, 1),
                clear::AfterCursor
            ),
        );
    }

    fn label(&mut self, text: &str) {
        let layout = self
            .layouts
            .last_mut()
            .expect("Tried to render label outside of any layout");
        let pos = layout.available_pos();

        term_goto(&mut self.term, (pos.y, pos.x));
        term_write(&mut self.term, text);

        layout.add_widget(Point::new(text.len() as u16, 1));
    }

    fn label_styled(&mut self, text: &str, pair: (&dyn color::Color, &dyn color::Color)) {
        term_set_style(&mut self.term, pair);
        self.label(text);
        term_style_reset(&mut self.term);
    }

    fn end(&mut self) {
        self.layouts
            .pop()
            .expect("Can't end a non-existing UI. Was there UI:begin()?");
        self.term.flush().unwrap();
    }

    fn end_layout(&mut self) {
        let layout = self
            .layouts
            .pop()
            .expect("Can't end a non-existing layout. Was there UI:begin_layout()?");
        self.layouts
            .last_mut()
            .expect("Can't end a non-existing layout. Was there UI:begin_layout()?")
            .add_widget(layout.size);
    }
}

fn term_set_style<W: Write>(s: &mut W, pair: (&dyn color::Color, &dyn color::Color)) {
    write!(s, "{}{}", color::Fg(pair.0), color::Bg(pair.1)).unwrap();
}

fn term_goto<W: Write>(s: &mut W, pos: (u16, u16)) {
    write!(s, "{}", cursor::Goto(pos.1 + 1, pos.0 + 1)).unwrap();
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

fn list_up(list: &Vec<String>, cur: &mut usize) {
    if *cur > 0 && list.len() > 0 {
        *cur -= 1;
    }
}

fn list_down(list: &Vec<String>, cur: &mut usize) {
    if list.len() > 0 {
        *cur = min(*cur + 1, list.len() - 1)
    }
}

fn list_move(from: &mut Vec<String>, to: &mut Vec<String>, cur: &mut usize) {
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
            eprintln!(
                "[ERROR]: {}:{}: invalid format in the line",
                file_path,
                id + 1
            );
            exit(1);
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
