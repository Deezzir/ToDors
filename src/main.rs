extern crate regex;
extern crate termion;
mod todo;

use std::cmp::min;
use std::env::args;
use std::fs::File;
use std::io::{self, stdin, BufRead, Read, Write};
use std::process::exit;

use regex::Regex;

use termion::color;
use termion::event::Key;
use termion::input::TermRead;

use todo::ui::*;

const HIGHLIGHT_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::White);

#[derive(PartialEq)]
pub enum Panel {
    Todo,
    Done,
}

impl Panel {
    pub fn togle(&self) -> Panel {
        match self {
            Panel::Todo => Panel::Done,
            Panel::Done => Panel::Todo,
        }
    }
}

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
    let mut panel = Panel::Todo;

    let mut todos: Vec<String> = Vec::<String>::new();
    let mut cur_todo: usize = 0;
    let mut dones: Vec<String> = Vec::<String>::new();
    let mut cur_done: usize = 0;

    parse_items(&file_path, &mut todos, &mut dones).unwrap();

    let mut stdin = stdin();
    let mut ui = UI::new();

    while !quit {
        ui.begin(Point::new(0, 0), LayoutKind::Horz);
        {
            ui.begin_layout(LayoutKind::Vert);
            {
                if panel == Panel::Todo {
                    ui.label_styled("[TODO]", HIGHLIGHT_PAIR);
                } else {
                    ui.label(" TODO ");
                }
                ui.label("-----------------------------");
                for (id, todo) in todos.iter().enumerate() {
                    if id == cur_todo && panel == Panel::Todo {
                        ui.label_styled(&format!("- [ ] {}", todo), HIGHLIGHT_PAIR);
                    } else {
                        ui.label(&format!("- [ ] {}", todo))
                    }
                }
            }
            ui.end_layout();

            ui.begin_layout(LayoutKind::Vert);
            {
                if panel == Panel::Done {
                    ui.label_styled("[DONE]", HIGHLIGHT_PAIR);
                } else {
                    ui.label(" DONE ");
                }
                ui.label("-----------------------------");
                for (id, done) in dones.iter().enumerate() {
                    if id == cur_done && panel == Panel::Done {
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
                Key::Esc | Key::Char('q') | Key::Ctrl('c') => quit = true,
                Key::Up | Key::Char('w') => match panel {
                    Panel::Todo => list_up(&todos, &mut cur_todo),
                    Panel::Done => list_up(&dones, &mut cur_done),
                },
                Key::Down | Key::Char('s') => match panel {
                    Panel::Todo => list_down(&todos, &mut cur_todo),
                    Panel::Done => list_down(&dones, &mut cur_done),
                },
                Key::Char('W') => match panel {
                    Panel::Todo => list_drag_up(&mut todos, &mut cur_todo),
                    Panel::Done => list_drag_up(&mut dones, &mut cur_done),
                },
                Key::Char('S') => match panel {
                    Panel::Todo => list_drag_down(&mut todos, &mut cur_todo),
                    Panel::Done => list_drag_down(&mut dones, &mut cur_done),
                },
                Key::Char('\n') => match panel {
                    Panel::Todo => list_move(&mut todos, &mut dones, &mut cur_todo),
                    Panel::Done => list_move(&mut dones, &mut todos, &mut cur_done),
                },
                Key::Char('\t') => {
                    panel = panel.togle();
                }
                _ => {}
            }
        }
    }

    ui.clear();
    dump_items(&file_path, &todos, &dones).unwrap();
    print!(
        "[INFO]: Goodbye, stranger! Your todo list is saved to '{}'.",
        file_path
    );
}

fn list_up(list: &Vec<String>, cur: &mut usize) {
    if *cur > 0 && !list.is_empty() {
        *cur -= 1;
    }
}

fn list_down(list: &Vec<String>, cur: &mut usize) {
    if !list.is_empty() {
        *cur = min(*cur + 1, list.len() - 1)
    }
}

fn list_drag_up(list: &mut Vec<String>, cur: &mut usize) {
    if *cur != 0 && list.len() > 1 {
        list.swap(*cur, *cur - 1);
        *cur -= 1;   
    }
}

fn list_drag_down(list: &mut Vec<String>, cur: &mut usize) {
    if *cur < list.len() - 1 && list.len() > 1 {
        list.swap(*cur, *cur + 1);
        *cur += 1;
    }
}

fn list_move(from: &mut Vec<String>, to: &mut Vec<String>, cur: &mut usize) {
    if *cur < from.len() {
        to.push(from.remove(*cur));
        if !from.is_empty() {
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

fn dump_items(file_path: &str, todos: &[String], dones: &[String]) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
    Ok(())
}
