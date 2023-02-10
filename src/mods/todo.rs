use std::cmp::{min, Ordering};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, Write};

use chrono::{DateTime, Local};

use ncurses::constants;
use regex::Regex;
// const MAX_STACK_SIZE: usize = 20;

#[derive(PartialEq, Clone, Copy, Debug)]
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

#[derive(PartialEq, Clone, Copy, Debug)]
enum Action {
    Delete,
    DragUp,
    DragDown,
    Transfer,
    Insert,
    Append,
    Edit,
    InEdit,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::Delete => write!(f, "Delete"),
            Action::DragUp => write!(f, "Drag up"),
            Action::DragDown => write!(f, "Drag down"),
            Action::Transfer => write!(f, "Transfer"),
            Action::Insert => write!(f, "Insert"),
            Action::Append => write!(f, "Append"),
            Action::Edit => write!(f, "Edit"),
            Action::InEdit => write!(f, ""),
        }
    }
}

#[derive(Debug)]
struct Operation {
    action: Action,
    panel: Panel,
}

impl Operation {
    fn new(action: Action, panel: Panel) -> Self {
        Self { action, panel }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Item {
    text: String,
    date: DateTime<Local>,
    parent: Option<usize>,
    children: Vec<usize>,
    dones_num: usize,
}

impl Item {
    fn new(text: String, date: DateTime<Local>, parent: Option<usize>) -> Self {
        Self {
            text,
            date,
            parent,
            children: Vec::new(),
            dones_num: 0,
        }
    }

    pub fn get_text(&self) -> &String {
        &self.text
    }

    pub fn get_date(&self) -> String {
        self.date.format("%y-%m-%d").to_string()
    }

    pub fn trim_text(&mut self) {
        self.text = self.text.trim().to_string();
    }
}

#[derive(Debug)]
struct List {
    state_stack: Vec<(Vec<Item>, usize)>,
    cur: usize,
    list: Vec<Item>,
}

pub struct ListIter<'a> {
    obj: &'a List,
    cur: usize,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = (&'a Item, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.obj.list.len() {
            let item = &self.obj.list[self.cur];
            self.cur += 1;

            let mut level = 0;
            let mut parent = item.parent;
            while let Some(p) = parent {
                level += 1;
                parent = self.obj.list[p].parent;
            }

            Some((item, level))
        } else {
            None
        }
    }
}

impl List {
    fn new() -> Self {
        Self {
            state_stack: Vec::new(),
            cur: 0,
            list: Vec::new(),
        }
    }

    fn add_item(&mut self, item: Item) {
        self.list.push(item);
    }

    fn add_child_to(&mut self, parent: usize, child: usize) {
        if let Some(item) = self.list.get_mut(parent) {
            item.children.push(child);
            item.dones_num += 1;
        }
    }

    fn get_cur_item(&self) -> Option<&Item> {
        self.list.get(self.cur)
    }

    fn get_cur_item_mut(&mut self) -> Option<&mut Item> {
        self.list.get_mut(self.cur)
    }

    fn record_state(&mut self) {
        self.state_stack.push((self.list.to_owned(), self.cur));
        // if self.state_stack.len() > MAX_STACK_SIZE {
        //     self.state_stack.truncate(MAX_STACK_SIZE);
        // }
    }

    fn revert_state(&mut self) -> Result<(), &'static str> {
        if !self.state_stack.is_empty() {
            (self.list, self.cur) = self.state_stack.pop().unwrap();
            Ok(())
        } else {
            Err("Nothing to undo.")
        }
    }

    fn up(&mut self) {
        if self.cur > 0 && !self.list.is_empty() {
            self.cur -= 1;
        }
    }

    fn down(&mut self) {
        if !self.list.is_empty() {
            self.cur = min(self.cur + 1, self.list.len() - 1)
        }
    }

    fn drag_up(&mut self) -> Result<(), &'static str> {
        if self.list.is_empty() {
            return Err("Can't drag up. List is empty.");
        }

        if self.cur != 0 && self.list.len() > 1 {
            self.list.swap(self.cur, self.cur - 1);
            self.cur -= 1;
            Ok(())
        } else {
            Err("Can't drag up. Item is already at the top.")
        }
    }

    fn drag_down(&mut self) -> Result<(), &'static str> {
        if self.list.is_empty() {
            return Err("Can't drag down. List is empty.");
        }

        if self.cur < self.list.len() - 1 && self.list.len() > 1 {
            self.list.swap(self.cur, self.cur + 1);
            self.cur += 1;
            Ok(())
        } else {
            Err("Can't drag down. Item is already at the bottom.")
        }
    }

    fn first(&mut self) {
        self.cur = 0;
    }

    fn half(&mut self) {
        if !self.list.is_empty() {
            self.cur = self.list.len() / 2;
        }
    }

    fn last(&mut self) {
        if !self.list.is_empty() {
            self.cur = self.list.len() - 1;
        }
    }

    fn shift_indices(&mut self, from: usize, by: usize, parent: Option<usize>) {
        for item in self.list.iter_mut().skip(from) {
            if item.parent > parent {
                if let Some(p) = item.parent {
                    item.parent = Some(p + by);
                }
            }
            for child_id in item.children.iter_mut() {
                *child_id += by;
            }
        }
    }

    fn insert(&mut self) -> Result<(), &'static str> {
        let item = self.get_cur_item();
        if item.is_some() && item.unwrap().parent.is_some() {
            return Err("Can't insert item. Current item is a subtask.");
        }

        self.shift_indices(self.cur, 1, None);
        self.list
            .insert(self.cur, Item::new(String::new(), Local::now(), None));

        Ok(())
    }

    fn append(&mut self) -> Result<(), &'static str> {
        if self.list.is_empty() {
            return Err("Can't add subtask item. List is empty.");
        }

        self.shift_indices(self.cur, 1, Some(self.cur));
        self.list.insert(
            self.cur + 1,
            Item::new(String::new(), Local::now(), Some(self.cur)),
        );
        self.list[self.cur].children.push(self.cur + 1);
        self.list[self.cur].dones_num += 1;
        self.cur += 1;

        Ok(())
    }

    fn delete(&mut self) -> Result<(), &'static str> {
        if self.cur < self.list.len() {
            self.list.remove(self.cur);
            if !self.list.is_empty() {
                self.cur = min(self.cur, self.list.len() - 1);
            } else {
                self.cur = 0;
            }
            Ok(())
        } else {
            Err("Can't delete item. List is empty.")
        }
    }

    fn transfer(&mut self, rhs: &mut Self) -> Result<(), &'static str> {
        if self.cur < self.list.len() {
            self.list[self.cur].date = Local::now();
            rhs.list.push(self.list.remove(self.cur));
            if !self.list.is_empty() {
                self.cur = min(self.cur, self.list.len() - 1);
            } else {
                self.cur = 0;
            }
            Ok(())
        } else {
            Err("Can't move item. List is empty.")
        }
    }

    fn edit(&mut self, cur: &mut usize, key: i32) {
        let item = &mut self.list[self.cur];
        *cur = min(*cur, item.text.len());

        match key {
            32..=126 => {
                if *cur > item.text.len() {
                    item.text.push(key as u8 as char);
                } else {
                    item.text.insert(*cur, key as u8 as char);
                }
                *cur += 1;
            }
            constants::KEY_LEFT => {
                if *cur > 0 {
                    *cur -= 1;
                }
            }
            constants::KEY_RIGHT => {
                if *cur < item.text.len() {
                    *cur += 1;
                }
            }
            constants::KEY_BACKSPACE | 127 => {
                // 127 is backspace
                if *cur > 0 {
                    *cur -= 1;
                    if *cur < item.text.len() {
                        item.text.remove(*cur);
                    }
                }
            }
            constants::KEY_DC => {
                if *cur < item.text.len() {
                    item.text.remove(*cur);
                }
            }
            constants::KEY_HOME | 1 => *cur = 0, // 1 is ctrl + a
            constants::KEY_END | 5 => *cur = item.text.len(), // 5 is ctrl + e
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct TodoApp {
    message: String,
    panel: Panel,

    operation_stack: Vec<Operation>,

    todos: List,
    dones: List,
}

impl TodoApp {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            panel: Panel::Todo,

            operation_stack: Vec::new(),

            todos: List::new(),
            dones: List::new(),
        }
    }

    pub fn is_in_todo_panel(&self) -> bool {
        self.panel == Panel::Todo
    }

    pub fn is_in_done_panel(&self) -> bool {
        self.panel == Panel::Done
    }

    pub fn is_cur_todo(&self, todo: &Item) -> bool {
        self.todos.get_cur_item().map_or(false, |t| t == todo)
    }

    pub fn is_cur_done(&self, done: &Item) -> bool {
        self.dones.get_cur_item().map_or(false, |d| d == done)
    }

    pub fn get_message(&self) -> &String {
        &self.message
    }

    pub fn iter_todos(&self) -> ListIter {
        ListIter {
            obj: &self.todos,
            cur: 0,
        }
    }

    pub fn get_todos_n(&self) -> usize {
        self.todos
            .list
            .iter()
            .filter(|item| item.parent.is_none())
            .count()
    }

    pub fn iter_dones(&self) -> ListIter {
        ListIter {
            obj: &self.dones,
            cur: 0,
        }
    }

    pub fn get_dones_n(&self) -> usize {
        self.dones
            .list
            .iter()
            .filter(|item| item.parent.is_none())
            .count()
    }

    pub fn clear_message(&mut self) {
        self.message.clear();
    }

    pub fn parse(&mut self, file_path: &str) {
        let file = File::open(file_path);
        let re_indent = Regex::new(r"^((\s{4})*)\S+").unwrap();
        let re_todo = Regex::new(r"^(\s{4})*TODO\(\): (.*)$").unwrap();
        let re_done = Regex::new(r"^(\s{4})*DONE\((.*)\): (.*)$").unwrap();

        match file {
            Ok(file) => {
                let mut stack = Vec::new();
                let mut cur_indent = 0;

                for (i, line) in io::BufReader::new(file).lines().enumerate() {
                    let line = line.unwrap();
                    let parent: Option<usize>;

                    if let Some(m) = re_indent.captures(&line) {
                        let indent = m[1].len() / 4;
                        match indent.cmp(&cur_indent) {
                            Ordering::Less => {
                                (0..(cur_indent - indent + 1)).map(|_| stack.pop()).last();
                                cur_indent = indent;
                            }
                            Ordering::Equal => drop(stack.pop()),
                            Ordering::Greater => cur_indent = indent,
                        }
                        parent = stack.last().copied();
                        stack.push(i);
                    } else {
                        panic!("[ERROR]: {}:{}: invalid indentation", file_path, i + 1);
                    }

                    if let Some(c) = re_todo.captures(&line) {
                        self.todos
                            .add_item(Item::new(c[2].to_string(), Local::now(), parent));
                        if let Some(parent) = parent {
                            self.todos.add_child_to(parent, i);
                        }
                    } else if let Some(c) = re_done.captures(&line) {
                        let date = DateTime::parse_from_str(&c[2], "%Y-%m-%d %H:%M %z")
                            .unwrap_or_else(|_| {
                                panic!("[ERROR]: {}:{}: invalid date format", file_path, i + 1)
                            });
                        self.dones.add_item(Item::new(
                            c[3].to_string(),
                            <DateTime<Local>>::from(date),
                            parent,
                        ));
                        if let Some(parent) = parent {
                            self.dones.add_child_to(parent, i);
                        }
                    } else {
                        panic!("[ERROR]: {}:{}: invalid format", file_path, i + 1);
                    }
                }
                self.message = format!("Loaded '{file_path}' file.")
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    self.message = format!("File '{file_path}' not found. Creating new one.");
                } else {
                    self.message =
                        format!("Error occureed while opening the file '{file_path}': {err:?}");
                }
            }
        }
    }

    pub fn save(&self, file_path: &str) -> std::io::Result<()> {
        let mut file = File::create(file_path)?;
        for (todo, i) in self.iter_todos() {
            let indent = "    ".repeat(i);
            writeln!(file, "{indent}TODO(): {}", todo.text).unwrap();
        }
        for (done, i) in self.iter_dones() {
            let indent = "    ".repeat(i);
            let date = done.date.format("%Y-%m-%d %H:%M %z");
            writeln!(file, "{indent}DONE({date}): {}", done.text).unwrap();
        }
        Ok(())
    }

    pub fn toggle_panel(&mut self) {
        assert!(!self.is_in_edit(), "Can't toggle panel while in edit mode.");
        self.panel = self.panel.togle();
    }

    pub fn go_up(&mut self) {
        assert!(!self.is_in_edit(), "Can't go up while in edit mode.");

        match self.panel {
            Panel::Todo => self.todos.up(),
            Panel::Done => self.dones.up(),
        }
    }

    pub fn go_down(&mut self) {
        assert!(!self.is_in_edit(), "Can't go down while in edit mode.");
        match self.panel {
            Panel::Todo => self.todos.down(),
            Panel::Done => self.dones.down(),
        }
    }

    pub fn go_top(&mut self) {
        assert!(!self.is_in_edit(), "Can't go top while in edit mode.");
        match self.panel {
            Panel::Todo => self.todos.first(),
            Panel::Done => self.dones.first(),
        }
    }

    pub fn go_half(&mut self) {
        assert!(!self.is_in_edit(), "Can't go half while in edit mode.");
        match self.panel {
            Panel::Todo => self.todos.half(),
            Panel::Done => self.dones.half(),
        }
    }

    pub fn go_bottom(&mut self) {
        assert!(!self.is_in_edit(), "Can't go bottom while in edit mode.");
        match self.panel {
            Panel::Todo => self.todos.last(),
            Panel::Done => self.dones.last(),
        }
    }

    pub fn drag_up(&mut self) {
        assert!(!self.is_in_edit(), "Can't drag up while in edit mode.");
        match self.panel {
            Panel::Todo => {
                self.todos.record_state();
                match self.todos.drag_up() {
                    Ok(()) => self
                        .operation_stack
                        .push(Operation::new(Action::DragUp, Panel::Todo)),
                    Err(err) => {
                        self.message.push_str(err);
                        self.todos.revert_state().unwrap();
                    }
                };
            }
            Panel::Done => {
                self.dones.record_state();
                match self.dones.drag_up() {
                    Ok(()) => self
                        .operation_stack
                        .push(Operation::new(Action::DragUp, Panel::Done)),
                    Err(err) => {
                        self.message.push_str(err);
                        self.dones.revert_state().unwrap();
                    }
                };
            }
        }
    }

    pub fn drag_down(&mut self) {
        assert!(!self.is_in_edit(), "Can't drag while in edit mode.");

        match self.panel {
            Panel::Todo => {
                self.todos.record_state();
                match self.todos.drag_down() {
                    Ok(()) => self
                        .operation_stack
                        .push(Operation::new(Action::DragDown, Panel::Todo)),
                    Err(err) => {
                        self.message.push_str(err);
                        self.todos.revert_state().unwrap();
                    }
                };
            }
            Panel::Done => {
                self.dones.record_state();
                match self.dones.drag_down() {
                    Ok(()) => self
                        .operation_stack
                        .push(Operation::new(Action::DragDown, Panel::Done)),
                    Err(err) => {
                        self.message.push_str(err);
                        self.dones.revert_state().unwrap();
                    }
                };
            }
        }
    }

    pub fn transfer_item(&mut self) {
        assert!(!self.is_in_edit(), "Can't move item while in edit mode");

        self.todos.record_state();
        self.dones.record_state();

        let result = match self.panel {
            Panel::Todo => self.todos.transfer(&mut self.dones),
            Panel::Done => self.dones.transfer(&mut self.todos),
        };
        match result {
            Ok(()) => match self.panel {
                Panel::Todo => {
                    self.operation_stack
                        .push(Operation::new(Action::Transfer, Panel::Todo));
                    self.message.push_str("Done! Great job!");
                }
                Panel::Done => {
                    self.operation_stack
                        .push(Operation::new(Action::Transfer, Panel::Done));
                    self.message.push_str("Not done yet? Keep going!")
                }
            },
            Err(err) => {
                self.message.push_str(err);
                self.todos.revert_state().unwrap();
                self.dones.revert_state().unwrap();
            }
        };
    }

    pub fn delete_item(&mut self) {
        assert!(!self.is_in_edit(), "Can't delete while in edit mode");

        match self.panel {
            Panel::Todo => self
                .message
                .push_str("Can't delete a TODO item. Mark it as DONE first."),
            Panel::Done => {
                self.dones.record_state();
                match self.dones.delete() {
                    Ok(()) => {
                        self.message.push_str("DONE item deleted.");
                        self.operation_stack
                            .push(Operation::new(Action::Delete, Panel::Done));
                    }
                    Err(err) => {
                        self.message.push_str(err);
                        self.dones.revert_state().unwrap();
                    }
                };
            }
        }
    }

    pub fn undo(&mut self) {
        assert!(!self.is_in_edit(), "Can't undo while in edit mode");

        let op = self.operation_stack.pop();
        match op {
            Some(op) => {
                match op.action {
                    Action::Transfer => {
                        self.dones.revert_state().unwrap();
                        self.todos.revert_state().unwrap();
                    }
                    _ => match op.panel {
                        Panel::Todo => {
                            self.todos.revert_state().unwrap();
                        }
                        Panel::Done => {
                            self.dones.revert_state().unwrap();
                        }
                    },
                }
                self.panel = op.panel;
                self.message.push_str(&format!("Undo: {}", op.action));
            }
            None => self.message.push_str("Nothing to undo."),
        }
    }

    pub fn insert_item(&mut self) -> Option<usize> {
        assert!(
            !self.is_in_edit(),
            "insert_item() called in already running edit mode."
        );

        let mut editing_cursor = None;

        match self.panel {
            Panel::Todo => {
                self.todos.record_state();
                match self.todos.insert() {
                    Ok(()) => {
                        self.operation_stack
                            .push(Operation::new(Action::Insert, Panel::Todo));
                        editing_cursor = Some(0);

                        self.operation_stack
                            .push(Operation::new(Action::InEdit, self.panel));
                        self.message.push_str("What needs to be done?");
                    }
                    Err(err) => {
                        self.message.push_str(err);
                        self.todos.revert_state().unwrap();
                    }
                }
            }
            Panel::Done => self
                .message
                .push_str("Can't insert a new DONE item. Only new TODO allowed."),
        }

        editing_cursor
    }

    pub fn append_item(&mut self) -> Option<usize> {
        assert!(
            !self.is_in_edit(),
            "append_item() called in already running edit mode."
        );

        let mut editing_cursor = None;

        match self.panel {
            Panel::Todo => {
                self.todos.record_state();
                match self.todos.append() {
                    Ok(()) => {
                        self.operation_stack
                            .push(Operation::new(Action::Append, Panel::Todo));
                        editing_cursor = Some(0);

                        self.operation_stack
                            .push(Operation::new(Action::InEdit, self.panel));
                        self.message
                            .push_str("What needs to be done for the current TODO?");
                    }
                    Err(err) => {
                        self.message.push_str(err);
                        self.todos.revert_state().unwrap();
                    }
                }
            }
            Panel::Done => self.message.push_str("Can't add sub-tasks for DONE items."),
        }

        editing_cursor
    }

    pub fn edit_item(&mut self) -> Option<usize> {
        assert!(
            !self.is_in_edit(),
            "edit_item() called in already running edit mode."
        );

        let mut editing_cursor = 0;

        if self.panel == Panel::Todo && !self.todos.list.is_empty() {
            editing_cursor = self.todos.get_cur_item().unwrap().text.len();
        } else if self.panel == Panel::Done && !self.dones.list.is_empty() {
            editing_cursor = self.dones.get_cur_item().unwrap().text.len();
        }

        if editing_cursor > 0 {
            match self.panel {
                Panel::Todo => {
                    self.todos.record_state();
                    self.operation_stack
                        .push(Operation::new(Action::Edit, Panel::Todo));
                }
                Panel::Done => {
                    self.dones.record_state();
                    self.operation_stack
                        .push(Operation::new(Action::Edit, Panel::Done));
                }
            };
            self.operation_stack
                .push(Operation::new(Action::InEdit, self.panel));
            self.message.push_str("Editing current item.");

            return Some(editing_cursor);
        } else {
            self.message.push_str("Nothing to edit.");
        }
        None
    }

    pub fn edit_item_with(&mut self, cur: &mut usize, key: i32) {
        assert!(
            self.is_in_edit(),
            "edit_item_with() called without a matching edit_item() or insert_item()"
        );

        match self.panel {
            Panel::Todo => self.todos.edit(cur, key),
            Panel::Done => self.dones.edit(cur, key),
        }
    }

    pub fn finish_edit(&mut self) -> bool {
        assert!(
            self.is_in_edit() && self.operation_stack.len() >= 2,
            "finish_edit() called without a matching edit_item() or insert_item()"
        );

        self.clear_message();

        match self.panel {
            Panel::Todo => {
                if self.todos.get_cur_item().unwrap().text.is_empty() {
                    let act = self
                        .operation_stack
                        .get(self.operation_stack.len() - 2)
                        .unwrap()
                        .action;
                    match act {
                        Action::Insert | Action::Append => {
                            self.todos.delete().unwrap();
                            if act == Action::Append {
                                self.todos.up();
                            }
                            self.todos.revert_state().unwrap();
                            self.operation_stack.pop();
                        }
                        Action::Edit => {
                            self.message.push_str("TODO item can't be empty.");
                            return false;
                        }
                        _ => unreachable!(),
                    }
                } else {
                    self.todos.get_cur_item_mut().unwrap().trim_text();
                }
            }
            Panel::Done => {
                if self.dones.get_cur_item().unwrap().text.is_empty() {
                    self.dones.delete().unwrap();
                } else {
                    self.dones.get_cur_item_mut().unwrap().trim_text();
                }
            }
        }

        self.operation_stack.pop();
        true
    }

    fn is_in_edit(&self) -> bool {
        let last_op = self.operation_stack.last();
        if last_op.is_none() {
            false
        } else {
            last_op.unwrap().action == Action::InEdit
        }
    }
}
