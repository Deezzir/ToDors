extern crate regex;
extern crate termion;
mod todo;

use std::cmp::min;
use std::env::args;
use std::fs::File;
use std::io::{self, stdin, BufRead, Write};
use std::process::exit;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::Local;

use regex::Regex;

use termion::event::Key;
use termion::input::TermRead;
use termion::terminal_size;

use todo::ui::*;

#[derive(PartialEq)]
enum Panel {
    Todo,
    Done,
}

impl Panel {
    fn togle(&self) -> Panel {
        match self {
            Panel::Todo => Panel::Done,
            Panel::Done => Panel::Todo,
        }
    }
}

struct Item {
    text: String,
    date: String,
    done: bool,
}

impl Item {
    fn new(text: String, date: String) -> Self {
        Self {
            text,
            date,
            done: false,
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
    let mut editing = false;
    let mut editing_cursor = 0;
    let mut panel = Panel::Todo;
    let mut message: String;

    let mut todos: Vec<Item> = Vec::<Item>::new();
    let mut cur_todo: usize = 0;
    let mut dones: Vec<Item> = Vec::<Item>::new();
    let mut cur_done: usize = 0;

    match parse_items(&file_path, &mut todos, &mut dones) {
        Ok(()) => message = format!("Loaded '{}' file.", file_path),
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                message = format!("File '{}' not found. Creating new one.", file_path);
            } else {
                eprintln!("Error while opening file '{}': {:?}", file_path, err);
                exit(1);
            }
        }
    }

    let mut ui = UI::new();

    let timeout = Duration::from_millis(100);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut keys = stdin().keys();
        while let Some(Ok(key)) = keys.next() {
            tx.send(key).unwrap();
        }
    });

    while !quit {
        let (width, _) = terminal_size().unwrap();

        ui.begin(Point::new(0, 0), LayoutKind::Vert);
        {
            ui.label(&format!("[MESSAGE]: {}", message));
            ui.label("");

            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    if panel == Panel::Todo {
                        ui.label_styled("[TODO]", HIGHLIGHT_PAIR);
                    } else {
                        ui.label(" TODO ");
                    }
                    ui.label("-".repeat(width as usize / 2).as_str());
                    for (id, todo) in todos.iter_mut().enumerate() {
                        if id == cur_todo && panel == Panel::Todo {
                            if editing {
                                ui.edit_label(
                                    &mut todo.text,
                                    &mut editing_cursor,
                                    "- [ ] ".to_string(),
                                );
                            } else {
                                ui.label_styled(&format!("- [ ] {}", todo.text), HIGHLIGHT_PAIR);
                            }
                        } else {
                            ui.label(&format!("- [ ] {}", todo.text));
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
                    ui.label("-".repeat(width as usize / 2).as_str());
                    for (id, done) in dones.iter_mut().enumerate() {
                        if id == cur_done && panel == Panel::Done {
                            if editing {
                                ui.edit_label(
                                    &mut done.text,
                                    &mut editing_cursor,
                                    "- [X] ".to_string(),
                                );
                            } else {
                                ui.label_styled(
                                    &format!("- [X] ({}) {}", done.date, done.text),
                                    HIGHLIGHT_PAIR,
                                );
                            }
                        } else {
                            ui.label(&format!("- [X] ({}) {}", done.date, done.text));
                        }
                    }
                }
                ui.end_layout();
            }
            ui.end_layout();
        }
        ui.end();

        if let Ok(key) = rx.recv_timeout(timeout) {
            if !editing {
                message.clear();

                match key {
                    Key::Up | Key::Char('k') => match panel {
                        Panel::Todo => list_up(&todos, &mut cur_todo),
                        Panel::Done => list_up(&dones, &mut cur_done),
                    },
                    Key::Down | Key::Char('j') => match panel {
                        Panel::Todo => list_down(&todos, &mut cur_todo),
                        Panel::Done => list_down(&dones, &mut cur_done),
                    },
                    Key::Char('K') => match panel {
                        Panel::Todo => list_drag_up(&mut todos, &mut cur_todo),
                        Panel::Done => list_drag_up(&mut dones, &mut cur_done),
                    },
                    Key::Char('J') => match panel {
                        Panel::Todo => list_drag_down(&mut todos, &mut cur_todo),
                        Panel::Done => list_drag_down(&mut dones, &mut cur_done),
                    },
                    Key::Char('g') => match panel {
                        Panel::Todo => list_first(&mut cur_todo),
                        Panel::Done => list_first(&mut cur_done),
                    },
                    Key::Char('G') => match panel {
                        Panel::Todo => list_last(&todos, &mut cur_todo),
                        Panel::Done => list_last(&dones, &mut cur_done),
                    },
                    Key::Char('d') => match panel {
                        Panel::Todo => {
                            message.push_str("Can't delete a TODO item. Mark it as DONE first.")
                        }
                        Panel::Done => {
                            list_delete(&mut dones, &mut cur_done);
                            message.push_str("DONE item deleted.");
                        }
                    },
                    Key::Char('i') => match panel {
                        Panel::Todo => {
                            list_insert(&mut todos, &mut cur_todo);
                            editing_cursor = 0;
                            editing = true;
                            message.push_str("What needs to be done?");
                        }
                        Panel::Done => {
                            message.push_str("Can't insert a new DONE item. Only new TODO allowed.")
                        }
                    },
                    Key::Char('r') => {
                        if panel == Panel::Todo && !todos.is_empty() {
                            editing_cursor = todos[cur_todo].text.len();
                        } else if panel == Panel::Done && !dones.is_empty() {
                            editing_cursor = dones[cur_done].text.len();
                        }
                        if editing_cursor > 0 {
                            editing = true;
                            message.push_str("Editing current item.");
                        }
                    }
                    Key::Char('\n') => match panel {
                        Panel::Todo => {
                            todos[cur_todo].done = true;
                            todos[cur_todo].date = Local::now().format("%y-%m-%d").to_string();

                            list_move(&mut todos, &mut dones, &mut cur_todo);
                            message.push_str("Done! Great job!");
                        }
                        Panel::Done => {
                            dones[cur_done].done = false;
                            dones[cur_done].date = String::new();

                            list_move(&mut dones, &mut todos, &mut cur_done);
                            message.push_str("Back to TODO list...");
                        }
                    },
                    Key::Char('\t') => {
                        panel = panel.togle();
                    }
                    Key::Char('q') | Key::Ctrl('c') => quit = true,
                    _ => {}
                }
            } else {
                match key {
                    Key::Char('\n') | Key::Esc => {
                        match panel {
                            Panel::Todo => {
                                if todos[cur_todo].text.is_empty() {
                                    list_delete(&mut todos, &mut cur_todo);
                                }
                            }
                            Panel::Done => {
                                if dones[cur_done].text.is_empty() {
                                    list_delete(&mut dones, &mut cur_done);
                                }
                            }
                        }

                        editing = false;
                        editing_cursor = 0;
                        message.clear();
                    }
                    _ => match panel {
                        Panel::Todo => {
                            list_edit(&mut todos[cur_todo], &mut editing_cursor, Some(key));
                        }
                        Panel::Done => {
                            list_edit(&mut dones[cur_done], &mut editing_cursor, Some(key));
                        }
                    },
                }
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

fn list_up(list: &Vec<Item>, cur: &mut usize) {
    if *cur > 0 && !list.is_empty() {
        *cur -= 1;
    }
}

fn list_down(list: &Vec<Item>, cur: &mut usize) {
    if !list.is_empty() {
        *cur = min(*cur + 1, list.len() - 1)
    }
}

fn list_drag_up(list: &mut Vec<Item>, cur: &mut usize) {
    if *cur != 0 && list.len() > 1 {
        list.swap(*cur, *cur - 1);
        *cur -= 1;
    }
}

fn list_drag_down(list: &mut Vec<Item>, cur: &mut usize) {
    if *cur < list.len() - 1 && list.len() > 1 {
        list.swap(*cur, *cur + 1);
        *cur += 1;
    }
}

fn list_first(cur: &mut usize) {
    if *cur != 0 {
        *cur = 0;
    }
}

fn list_last(list: &Vec<Item>, cur: &mut usize) {
    if !list.is_empty() {
        *cur = list.len() - 1;
    }
}

fn list_insert(list: &mut Vec<Item>, cur: &mut usize) {
    list.insert(*cur, Item::new(String::new(), String::new()));
}

fn list_delete(list: &mut Vec<Item>, cur: &mut usize) {
    if *cur < list.len() {
        list.remove(*cur);
        if !list.is_empty() {
            *cur = min(*cur, list.len() - 1);
        } else {
            *cur = 0;
        }
    }
}

fn list_edit(item: &mut Item, cur: &mut usize, mut key: Option<Key>) {
    if *cur > item.text.len() {
        *cur = item.text.len();
    }

    if let Some(key) = key.take() {
        match key {
            Key::Left => {
                if *cur > 0 {
                    *cur -= 1;
                }
            }
            Key::Right => {
                if *cur < item.text.len() {
                    *cur += 1;
                }
            }
            Key::Backspace => {
                if *cur > 0 {
                    *cur -= 1;
                    if *cur < item.text.len() {
                        item.text.remove(*cur);
                    }
                }
            }
            Key::Delete => {
                if *cur < item.text.len() {
                    item.text.remove(*cur);
                }
            }
            Key::Home => *cur = 0,
            Key::End => *cur = item.text.len(),
            Key::Char(c) => {
                let c = c as u8;
                if c.is_ascii() && (32..127).contains(&c) {
                    if *cur > item.text.len() {
                        item.text.push(c as char);
                    } else {
                        item.text.insert(*cur, c as char);
                    }
                    *cur += 1;
                }
            }
            _ => {}
        }
    }
}

fn list_move(from: &mut Vec<Item>, to: &mut Vec<Item>, cur: &mut usize) {
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
    todos: &mut Vec<Item>,
    dones: &mut Vec<Item>,
) -> std::io::Result<()> {
    let file = File::open(file_path)?;
    let re_todo = Regex::new(r"^TODO\(\): (.*)$").unwrap();
    let re_done = Regex::new(r"^DONE\((\d{2}-\d{2}-\d{2})\): (.*)$").unwrap();

    for (id, line) in io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if let Some(caps) = re_todo.captures(&line) {
            let item = Item::new(caps[1].to_string(), String::new());
            todos.push(item);
        } else if let Some(caps) = re_done.captures(&line) {
            let item = Item::new(caps[2].to_string(), caps[1].to_string());
            dones.push(item);
        } else {
            eprintln!("[ERROR]: {}:{}: invalid format", file_path, id + 1);
            exit(1);
        }
    }
    Ok(())
}

fn dump_items(file_path: &str, todos: &[Item], dones: &[Item]) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    for todo in todos.iter() {
        writeln!(file, "TODO(): {}", todo.text).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE({}): {}", done.date, done.text).unwrap();
    }
    Ok(())
}
