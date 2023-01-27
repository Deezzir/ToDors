use std::cmp::min;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::exit;

use chrono::Local;

use regex::Regex;

use termion::event::Key;

use crate::MAX_STACK_SIZE;

#[derive(PartialEq, Clone, Copy)]
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

#[derive(PartialEq)]
pub enum Action {
    Delete,
    DragUp,
    DragDown,
    Move,
    Insert,
    Edit,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::Delete => write!(f, "Delete"),
            Action::DragUp => write!(f, "Drag up"),
            Action::DragDown => write!(f, "Drag down"),
            Action::Move => write!(f, "Move"),
            Action::Insert => write!(f, "Insert"),
            Action::Edit => write!(f, "Edit"),
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
        Self { text, date }
    }
}

pub struct List {
    message: String,
    editing: bool,
    editing_cursor: usize,
    panel: Panel,
    operation_stack: Vec<Operation>,
    todos: Vec<Item>,
    todo_stack: Vec<Vec<Item>>,
    cur_todo: usize,
    dones: Vec<Item>,
    dones_stack: Vec<Vec<Item>>,
    cur_done: usize,
}

impl List {
    pub fn new(file_path: &str) -> Self {
        let mut todos = Vec::new();
        let mut dones = Vec::new();
        let message;

        match list_parse(file_path, &mut todos, &mut dones) {
            Ok(()) => message = format!("Loaded '{}' file.", file_path),
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    message = format!("File '{}' not found. Creating new one.", file_path);
                } else {
                    message = format!(
                        "Error occureed while opening the file '{}': {:?}",
                        file_path, err
                    );
                }
            }
        }

        Self {
            message,
            panel: Panel::Todo,
            editing: false,
            editing_cursor: 0,

            operation_stack: Vec::new(),
            todos,

            todo_stack: Vec::new(),
            cur_todo: 0,
            dones,

            dones_stack: Vec::new(),
            cur_done: 0,
        }
    }

    pub fn get_message(&self) -> &String {
        &self.message
    }

    pub fn get_panel(&self) -> Panel {
        self.panel
    }

    pub fn get_editing_cursor(&self) -> usize {
        self.editing_cursor
    }

    pub fn get_cur_todo(&self) -> usize {
        self.cur_todo
    }

    pub fn get_cur_done(&self) -> usize {
        self.cur_done
    }


    pub fn get_mut_todos(&mut self) -> &mut Vec<Item> {
        &mut self.todos
    }

    pub fn get_mut_dones(&mut self) -> &mut Vec<Item> {
        &mut self.dones
    }

    pub fn clear_message(&mut self) {
        self.message.clear();
    }

    pub fn save(&self, file_path: &str) {
        list_dump(file_path, &self.todos, &self.dones).unwrap();
    }

    pub fn toggle_panel(&mut self) {
        self.panel = self.panel.togle();
    }

    pub fn go_up(&mut self) {
        match self.panel {
            Panel::Todo => list_up(&self.todos, &mut self.cur_todo),
            Panel::Done => list_up(&self.dones, &mut self.cur_done),
        }
    }

    pub fn go_down(&mut self) {
        match self.panel {
            Panel::Todo => list_down(&self.todos, &mut self.cur_todo),
            Panel::Done => list_down(&self.dones, &mut self.cur_done),
        }
    }

    pub fn go_top(&mut self) {
        match self.panel {
            Panel::Todo => list_first(&mut self.cur_todo),
            Panel::Done => list_first(&mut self.cur_done),
        }
    }

    pub fn go_bottom(&mut self) {
        match self.panel {
            Panel::Todo => list_last(&self.todos, &mut self.cur_todo),
            Panel::Done => list_last(&self.dones, &mut self.cur_done),
        }
    }

    pub fn drag_up(&mut self) {
        match self.panel {
            Panel::Todo => {
                list_record_state(&mut self.todo_stack, &self.todos);
                match list_drag_up(&mut self.todos, &mut self.cur_todo) {
                    Ok(()) => self.operation_stack.push(Operation::new(
                        Action::DragUp,
                        self.cur_todo + 1,
                        Panel::Todo,
                    )),
                    Err(err) => {
                        self.message.push_str(err);
                        list_revert_state(&mut self.todo_stack, &mut self.todos).unwrap();
                    }
                };
            }
            Panel::Done => {
                list_record_state(&mut self.dones_stack, &self.dones);
                match list_drag_up(&mut self.dones, &mut self.cur_done) {
                    Ok(()) => self.operation_stack.push(Operation::new(
                        Action::DragUp,
                        self.cur_done + 1,
                        Panel::Done,
                    )),
                    Err(err) => {
                        self.message.push_str(err);
                        list_revert_state(&mut self.dones_stack, &mut self.dones).unwrap();
                    }
                };
            }
        }
    }

    pub fn drag_down(&mut self) {
        match self.panel {
            Panel::Todo => {
                list_record_state(&mut self.todo_stack, &self.todos);
                match list_drag_down(&mut self.todos, &mut self.cur_todo) {
                    Ok(()) => self.operation_stack.push(Operation::new(
                        Action::DragDown,
                        self.cur_todo - 1,
                        Panel::Todo,
                    )),
                    Err(err) => {
                        self.message.push_str(err);
                        list_revert_state(&mut self.todo_stack, &mut self.todos).unwrap();
                    }
                };
            }
            Panel::Done => {
                list_record_state(&mut self.dones_stack, &self.dones);
                match list_drag_down(&mut self.dones, &mut self.cur_done) {
                    Ok(()) => self.operation_stack.push(Operation::new(
                        Action::DragDown,
                        self.cur_done - 1,
                        Panel::Done,
                    )),
                    Err(err) => {
                        self.message.push_str(err);
                        list_revert_state(&mut self.dones_stack, &mut self.dones).unwrap();
                    }
                };
            }
        }
    }

    pub fn move_item(&mut self) {
        list_record_state(&mut self.todo_stack, &self.todos);
        list_record_state(&mut self.dones_stack, &self.dones);

        let result = match self.panel {
            Panel::Todo => {
                self.todos[self.cur_todo].date = Local::now().format("%y-%m-%d").to_string();
                list_move(&mut self.todos, &mut self.dones, &mut self.cur_todo)
            }
            Panel::Done => {
                self.dones[self.cur_done].date = String::new();
                list_move(&mut self.dones, &mut self.todos, &mut self.cur_done)
            }
        };
        match result {
            Ok(()) => match self.panel {
                Panel::Todo => {
                    self.operation_stack.push(Operation::new(
                        Action::Move,
                        self.cur_todo,
                        Panel::Todo,
                    ));
                    self.message.push_str("Done! Great job!");
                }
                Panel::Done => {
                    self.operation_stack.push(Operation::new(
                        Action::Move,
                        self.cur_done,
                        Panel::Done,
                    ));
                    self.message.push_str("Not done yet? Keep going!")
                }
            },
            Err(err) => {
                self.message.push_str(err);
                list_revert_state(&mut self.todo_stack, &mut self.todos).unwrap();
                list_revert_state(&mut self.dones_stack, &mut self.dones).unwrap();
            }
        };
    }

    pub fn delete_item(&mut self) {
        match self.panel {
            Panel::Todo => self
                .message
                .push_str("Can't delete a TODO item. Mark it as DONE first."),
            Panel::Done => {
                list_record_state(&mut self.dones_stack, &self.dones);
                match list_delete(&mut self.dones, &mut self.cur_done) {
                    Ok(()) => {
                        self.message.push_str("DONE item deleted.");
                        self.operation_stack.push(Operation::new(
                            Action::Delete,
                            self.cur_done,
                            Panel::Done,
                        ));
                    }
                    Err(err) => {
                        self.message.push_str(err);
                        list_revert_state(&mut self.dones_stack, &mut self.dones).unwrap();
                    }
                };
            }
        }
    }

    pub fn undo(&mut self) {
        let op = self.operation_stack.pop();
        match op {
            Some(op) => {
                match op.action {
                    Action::Move => {
                        list_revert_state(&mut self.dones_stack, &mut self.dones).unwrap();
                        list_revert_state(&mut self.todo_stack, &mut self.todos).unwrap();
                    }
                    _ => match op.panel {
                        Panel::Todo => {
                            list_revert_state(&mut self.todo_stack, &mut self.todos).unwrap();
                        }
                        Panel::Done => {
                            list_revert_state(&mut self.dones_stack, &mut self.dones).unwrap();
                        }
                    },
                }
                if op.panel == Panel::Todo {
                    self.cur_todo = op.cur;
                } else {
                    self.cur_done = op.cur;
                }
                self.panel = op.panel;
                self.message.push_str(&format!("Undo: {}", op.action));
            }
            None => self.message.push_str("Nothing to undo."),
        }
    }

    pub fn insert_item(&mut self) -> bool {
        if !self.editing {
            match self.panel {
                Panel::Todo => {
                    list_record_state(&mut self.todo_stack, &self.todos);
                    self.operation_stack.push(Operation::new(
                        Action::Insert,
                        self.cur_todo,
                        Panel::Todo,
                    ));

                    list_insert(&mut self.todos, &mut self.cur_todo);
                    self.editing = true;
                    self.editing_cursor = 0;
                    self.message.push_str("What needs to be done?");
                }
                Panel::Done => self
                    .message
                    .push_str("Can't insert a new DONE item. Only new TODO allowed."),
            }
        }
        self.editing
    }

    pub fn edit_item(&mut self) -> bool {
        if !self.editing {
            if self.panel == Panel::Todo && !self.todos.is_empty() {
                self.editing_cursor = self.todos[self.cur_todo].text.len();
            } else if self.panel == Panel::Done && !self.dones.is_empty() {
                self.editing_cursor = self.dones[self.cur_done].text.len();
            }
            if self.editing_cursor > 0 {
                match self.panel {
                    Panel::Todo => {
                        list_record_state(&mut self.todo_stack, &self.todos);
                        self.operation_stack.push(Operation::new(
                            Action::Edit,
                            self.cur_todo,
                            Panel::Todo,
                        ));
                    }
                    Panel::Done => {
                        list_record_state(&mut self.dones_stack, &self.dones);
                        self.operation_stack.push(Operation::new(
                            Action::Edit,
                            self.cur_done,
                            Panel::Done,
                        ));
                    }
                };
                self.editing = true;
                self.message.push_str("Editing current item.");
            }
        }
        self.editing
    }

    pub fn edit_item_with(&mut self, key: Option<Key>) {
        if !self.editing {
            return;
        }

        match self.panel {
            Panel::Todo => {
                list_edit(
                    &mut self.todos[self.cur_todo],
                    &mut self.editing_cursor,
                    key,
                );
            }
            Panel::Done => {
                list_edit(
                    &mut self.dones[self.cur_done],
                    &mut self.editing_cursor,
                    key,
                );
            }
        }
    }

    pub fn finish_edit(&mut self) -> bool {
        if self.editing {
            match self.panel {
                Panel::Todo => {
                    if self.todos[self.cur_todo].text.is_empty() {
                        list_delete(&mut self.todos, &mut self.cur_todo).unwrap();
                    }
                }
                Panel::Done => {
                    if self.dones[self.cur_done].text.is_empty() {
                        list_delete(&mut self.dones, &mut self.cur_done).unwrap();
                    }
                }
            }

            self.editing = false;
            self.editing_cursor = 0;
            self.clear_message();
        }
        self.editing
    }
}

fn list_record_state(stack: &mut Vec<Vec<Item>>, list: &[Item]) {
    stack.push(list.to_owned());
    if stack.len() > MAX_STACK_SIZE {
        stack.truncate(MAX_STACK_SIZE);
    }
}

fn list_revert_state(stack: &mut Vec<Vec<Item>>, list: &mut Vec<Item>) -> Result<(), &'static str> {
    if !stack.is_empty() {
        *list = stack.pop().unwrap();
        Ok(())
    } else {
        Err("Nothing to undo.")
    }
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

fn list_drag_up(list: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
    if *cur != 0 && list.len() > 1 {
        list.swap(*cur, *cur - 1);
        *cur -= 1;
        Ok(())
    } else {
        Err("Can't drag up. Item is already at the top.")
    }
}

fn list_drag_down(list: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
    if *cur < list.len() - 1 && list.len() > 1 {
        list.swap(*cur, *cur + 1);
        *cur += 1;
        Ok(())
    } else {
        Err("Can't drag down. Item is already at the bottom.")
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

fn list_delete(list: &mut Vec<Item>, cur: &mut usize) -> Result<(), &'static str> {
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

fn list_move(
    from: &mut Vec<Item>,
    to: &mut Vec<Item>,
    cur: &mut usize,
) -> Result<(), &'static str> {
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

fn list_parse(
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

fn list_dump(file_path: &str, todos: &[Item], dones: &[Item]) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    for todo in todos.iter() {
        writeln!(file, "TODO(): {}", todo.text).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE({}): {}", done.date, done.text).unwrap();
    }
    Ok(())
}
