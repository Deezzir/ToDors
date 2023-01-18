use std::fmt;
use std::cmp::min;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::exit;

use regex::Regex;

use termion::event::Key;

pub const MAX_STACK_SIZE: usize = 20;

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

pub enum Action {
    Delete,
    DragUp,
    DragDown,
    Move,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::Delete => write!(f, "Delete"),
            Action::DragUp => write!(f, "Drag up"),
            Action::DragDown => write!(f, "Drag down"),
            Action::Move => write!(f, "Move"),
        }
    }
}

pub struct Operation {
    pub action: Action,
    pub cur: usize,
    pub panel: Panel,
}

impl Operation {
    pub fn new(action: Action, cur: usize, panel: Panel) -> Self {
        Self { action, cur, panel }
    }
}

#[derive(Clone)]
pub struct Item {
    pub text: String,
    pub date: String,
}

impl Item {
    pub fn new(text: String, date: String) -> Self {
        Self {
            text,
            date,
        }
    }
}

pub fn list_record_state(stack: &mut Vec<Vec<Item>>, list: &[Item]) {
    stack.push(list.to_owned());
    if stack.len() > MAX_STACK_SIZE {
        stack.truncate(MAX_STACK_SIZE);
    }
}

pub fn list_revert_state(stack: &mut Vec<Vec<Item>>, list: &mut Vec<Item>) -> Result<(), &'static str> {
    if !stack.is_empty() {
        *list = stack.pop().unwrap();
        Ok(())
    } else {
        Err("Nothing to undo.")
    }
}

pub fn list_up(list: &Vec<Item>, cur: &mut usize) {
    if *cur > 0 && !list.is_empty() {
        *cur -= 1;
    }
}

pub fn list_down(list: &Vec<Item>, cur: &mut usize) {
    if !list.is_empty() {
        *cur = min(*cur + 1, list.len() - 1)
    }
}

pub fn list_drag_up(list: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
    if *cur != 0 && list.len() > 1 {
        list.swap(*cur, *cur - 1);
        *cur -= 1;
        Ok(())
    } else {
        Err("Can't drag up. Item is already at the top.")
    }
}

pub fn list_drag_down(list: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
    if *cur < list.len() - 1 && list.len() > 1 {
        list.swap(*cur, *cur + 1);
        *cur += 1;
        Ok(())
    } else {
        Err("Can't drag down. Item is already at the bottom.")
    }
}

pub fn list_first(cur: &mut usize) {
    if *cur != 0 {
        *cur = 0;
    }
}

pub fn list_last(list: &Vec<Item>, cur: &mut usize) {
    if !list.is_empty() {
        *cur = list.len() - 1;
    }
}

pub fn list_insert(list: &mut Vec<Item>, cur: &mut usize) {
    list.insert(*cur, Item::new(String::new(), String::new()));
}

pub fn list_delete(list: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
    if *cur < list.len() {
        list.remove(*cur);
        if !list.is_empty() {
            *cur = min(*cur, list.len() - 1);
        } else {
            *cur = 0;
        }
        Ok(())
    } else {
        Err("Can't delete item. List is empty.")
    }
}

pub fn list_edit(item: &mut Item, cur: &mut usize, mut key: Option<Key>) {
    if *cur > item.text.len() {
        *cur = item.text.len();
    }

    if let Some(key) = key.take() {
        match key {
            Key::Left => {
                if *cur > 0 {
                    *cur -= 1;
                }
            },
            Key::Right => {
                if *cur < item.text.len() {
                    *cur += 1;
                }
            },
            Key::Backspace => {
                if *cur > 0 {
                    *cur -= 1;
                    if *cur < item.text.len() {
                        item.text.remove(*cur);
                    }
                }
            },
            Key::Delete => {
                if *cur < item.text.len() {
                    item.text.remove(*cur);
                }
            },
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
            },
            _ => {}
        }
    }
}

pub fn list_move(from: &mut Vec<Item>, to: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
    if *cur < from.len() {
        to.push(from.remove(*cur));
        if !from.is_empty() {
            *cur = min(*cur, from.len() - 1);
        } else {
            *cur = 0;
        }
        Ok(())
    } else {
        Err("Can't move item. List is empty.")
    }
}

pub fn parse_items(
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

pub fn dump_items(file_path: &str, todos: &[Item], dones: &[Item]) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    for todo in todos.iter() {
        writeln!(file, "TODO(): {}", todo.text).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE({}): {}", done.date, done.text).unwrap();
    }
    Ok(())
}