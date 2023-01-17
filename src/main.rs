extern crate regex;
extern crate termion;
mod todo;

use std::cmp::min;
use std::env::args;
use std::fs::File;
use std::io::{self, stdin, BufRead, Write};
use std::process::exit;

use regex::Regex;

use termion::event::Key;
use termion::terminal_size;
use termion::input::TermRead;

use todo::ui::*;

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

    // let file_path = match args.next() {
    //     Some(file_path) => file_path,
    //     None => {
    //         eprintln!("Usage: todo <file>");
    //         eprintln!("[ERROR]: No file specified");
    //         exit(1);
    //     }
    // };

    let file_path = "TODO";

    let mut quit: bool = false;
    let mut editing = false;
    let mut editing_cursor = 0;
    let mut panel = Panel::Todo;
    let mut message: String;

    let mut todos: Vec<String> = Vec::<String>::new();
    let mut cur_todo: usize = 0;
    let mut dones: Vec<String> = Vec::<String>::new();
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

    let mut keys = stdin().keys();
    let mut ui = UI::new();

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
                    ui.label("-".repeat(width as usize / 2 - 1).as_str());
                    for (id, todo) in todos.iter_mut().enumerate() {
                        if id == cur_todo && panel == Panel::Todo {
                            if editing {
                                ui.edit_field(todo, &mut editing_cursor, "- [ ] ".to_string());
                            } else {
                                ui.label_styled(&format!("- [ ] {}", todo), HIGHLIGHT_PAIR);
                            }
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
                    ui.label("-".repeat(width as usize / 2 - 1).as_str());
                    for (id, done) in dones.iter_mut().enumerate() {
                        if id == cur_done && panel == Panel::Done {
                            if editing {
                                ui.edit_field(done, &mut editing_cursor, "- [X] ".to_string());
                            } else {
                                ui.label_styled(&format!("- [X] {}", done), HIGHLIGHT_PAIR);
                            }
                        } else {
                            ui.label(&format!("- [X] {}", done))
                        }
                    }
                }
                ui.end_layout();
            }
            ui.end_layout();
        }
        ui.end();

        if let Some(Ok(key)) = keys.next() {
            if !editing {
                message.clear();
            }
            ui.key = Some(key);

            match key {
                Key::Char('q') | Key::Ctrl('c') => quit = true,
                _ => {}
            }
        }

        if let Some(key) = ui.key.take() {
            if !editing {
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
                            message.push_str("Can not delete TODO item. Mark it as DONE first.")
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
                        Panel::Done => message.push_str(
                            "Can't insert a new DONE item. Only TODO items can inserted.",
                        ),
                    },
                    Key::Char('r') => { 
                        editing_cursor = 0;
                        if panel == Panel::Todo && !todos.is_empty() {
                            editing_cursor = todos[cur_todo].len();
                        } else if panel == Panel::Done && !dones.is_empty() {
                            editing_cursor = dones[cur_done].len();
                        }
                        if editing_cursor > 0 {
                            ui.key = None;
                            editing = true;
                            message.push_str("Editing item.");
                        }
                    }
                    Key::Char('\n') => match panel {
                        Panel::Todo => {
                            list_move(&mut todos, &mut dones, &mut cur_todo);
                            message.push_str("Done!");
                        }
                        Panel::Done => {
                            list_move(&mut dones, &mut todos, &mut cur_done);
                            message.push_str("Back to TODO list.");
                        }
                    },
                    Key::Char('\t') => {
                        panel = panel.togle();
                    }
                    _ => {}
                }
            } else {
                match key {
                    Key::Char('\n') | Key::Esc => {
                        editing = false;
                        message.clear();
                    }
                    _ => ui.key = Some(key),
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

fn list_first(cur: &mut usize) {
    if *cur != 0 {
        *cur = 0;
    }
}

fn list_last(list: &Vec<String>, cur: &mut usize) {
    if !list.is_empty() {
        *cur = list.len() - 1;
    }
}

fn list_insert(list: &mut Vec<String>, cur: &mut usize) {
    list.insert(*cur, String::new());
}

fn list_delete(list: &mut Vec<String>, cur: &mut usize) {
    if *cur < list.len() {
        list.remove(*cur);
        if !list.is_empty() {
            *cur = min(*cur, list.len() - 1);
        } else {
            *cur = 0;
        }
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
