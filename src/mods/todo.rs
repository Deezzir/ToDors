use std::cmp::{min, Ordering};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::exit;

use chrono::{DateTime, Local};

use ncurses::constants;
use regex::Regex;

use crate::INDENT_SIZE;
// const MAX_STACK_SIZE: usize = 20;
const SEP: &str = "<--->";
const DATE_FMT: &str = "%Y-%m-%d %H:%M %z";

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
    Mark,
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
            Action::Mark => write!(f, "Mark"),
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
    act_cnt: usize,
}

impl Item {
    fn new(text: String, date: DateTime<Local>, parent: Option<usize>, act_cnt: usize) -> Self {
        Self {
            text,
            date,
            parent,
            children: Vec::new(),
            act_cnt,
        }
    }

    pub fn get_text(&self) -> &String {
        &self.text
    }

    pub fn get_date(&self) -> String {
        self.date.format("%y-%m-%d").to_string()
    }

    pub fn is_active(&self) -> bool {
        self.act_cnt > 0
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    fn trim_text(&mut self) {
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
    skip_children: bool,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = (&'a Item, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.obj.list.len() {
            let item = &self.obj.list[self.cur];
            self.cur += 1;

            if self.skip_children && item.parent.is_some() {
                return self.next();
            }

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

    fn add_child_to(&mut self, parent: usize, child: usize, inc: bool) {
        if let Some(item) = self.list.get_mut(parent) {
            item.children.push(child);
            if inc {
                item.act_cnt += 1;
            }
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
        if let Some(item) = self.list.get_mut(self.cur) {
            let parent = item.parent;
            let pier = self.list[..self.cur]
                .iter()
                .rposition(|item| item.parent == parent);

            match (parent, pier) {
                (None, None) => Err("Can't drag up. Item is already at the top."),
                (Some(_), None) => Err("Can't move a subtask out from its parent."),
                (_, Some(pier)) => {
                    let pier_child_cnt = self.children_cnt(pier) + 1;
                    let child_cnt = self.children_cnt(self.cur) + 1;
                    let move_to = self.cur - pier_child_cnt;

                    self.shift_indices(
                        -(pier_child_cnt as isize),
                        self.cur,
                        Some(self.cur + child_cnt),
                        parent,
                    );
                    self.shift_indices(
                        child_cnt as isize,
                        pier,
                        Some(self.cur),
                        self.list[pier].parent,
                    );

                    let to_drag: Vec<Item> =
                        self.list.drain(self.cur..self.cur + child_cnt).collect();
                    self.list.splice(move_to..move_to, to_drag);
                    if let Some(parent) = parent {
                        self.list[parent].children.retain(|&x| x != self.cur);
                        self.list[parent].children.push(move_to + child_cnt);
                    }

                    self.cur = move_to;
                    Ok(())
                }
            }
        } else {
            Err("Can't drag up. List is empty.")
        }
    }

    fn drag_down(&mut self) -> Result<(), &'static str> {
        if let Some(item) = self.list.get_mut(self.cur) {
            let parent = item.parent;
            let pier = self.list[self.cur + 1..]
                .iter()
                .position(|item| item.parent == parent);

            match (parent, pier) {
                (None, None) => Err("Can't drag down. Item is already at the bottom."),
                (Some(_), None) => Err("Can't move a subtask out from its parent."),
                (_, Some(pier)) => {
                    let pier = pier + self.cur + 1;
                    let pier_child_cnt = self.children_cnt(pier) + 1;
                    let child_cnt = self.children_cnt(self.cur) + 1;
                    let move_to = self.cur + pier_child_cnt;

                    self.shift_indices(pier_child_cnt as isize, self.cur, Some(pier), parent);
                    self.shift_indices(
                        -(child_cnt as isize),
                        pier,
                        Some(pier + pier_child_cnt),
                        self.list[pier].parent,
                    );

                    let to_drag: Vec<Item> =
                        self.list.drain(self.cur..self.cur + child_cnt).collect();
                    self.list.splice(move_to..move_to, to_drag);

                    if let Some(parent) = parent {
                        self.list[parent].children.retain(|&x| x != pier);
                        self.list[parent].children.push(move_to);
                    }

                    self.cur = move_to;
                    Ok(())
                }
            }
        } else {
            Err("Can't drag up. List is empty.")
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

    fn children_cnt(&self, parent: usize) -> usize {
        let mut cnt = 0;
        if let Some(item) = self.list.get(parent) {
            if item.children.is_empty() {
                return 0;
            }

            for child in item.children.iter() {
                cnt += 1 + self.children_cnt(*child);
            }
        }
        cnt
    }

    fn shift_indices(&mut self, by: isize, from: usize, to: Option<usize>, parent: Option<usize>) {
        let to = to.unwrap_or(self.list.len());
        assert!(from <= to, "from must be less or equal than to");

        for item in self.list.iter_mut().skip(from).take(to - from) {
            if item.parent > parent {
                if let Some(p) = item.parent {
                    if p as isize + by >= 0 {
                        item.parent = Some((p as isize + by) as usize);
                    }
                }
            }
            for child_id in item.children.iter_mut() {
                if *child_id as isize + by >= 0 && *child_id >= parent {
                    *child_id = (*child_id as isize + by) as usize;
                }
            }
        }
    }

    fn insert(&mut self) -> Result<(), &'static str> {
        if let Some(item) = self.get_cur_item() {
            if item.parent.is_some() {
                return Err("Can't insert item. Current item is a subtask.");
            }
        }

        let item = Item::new(String::new(), Local::now(), None, 1);

        self.shift_indices(1, self.cur, None, None);
        self.list.insert(self.cur, item);

        Ok(())
    }

    fn append(&mut self) -> Result<(), &'static str> {
        if let Some(parent) = self.list.get_mut(self.cur) {
            let item = Item::new(String::new(), Local::now(), Some(self.cur), 1);
            parent.children.push(self.cur);

            self.unmark_parents(Some(self.cur));
            self.shift_indices(1, 0, None, Some(self.cur));

            self.list.insert(self.cur + 1, item);
            self.cur += 1;

            Ok(())
        } else {
            Err("Can't add subtask item. List is empty.")
        }
    }

    fn unmark_parents(&mut self, parent: Option<usize>) {
        let mut parent = parent;
        while let Some(p) = parent {
            if self.list[p].act_cnt >= 1 {
                self.list[p].act_cnt += 1;
                break;
            } else if self.list[p].act_cnt == 0 {
                self.list[p].act_cnt += 2;
            }
            parent = self.list[p].parent;
        }
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

    fn mark(&mut self) -> Result<(), &'static str> {
        if let Some(item) = self.get_cur_item_mut() {
            if item.act_cnt > 1 {
                return Err("Can't mark item. Item has active subtasks.");
            }

            let parent = item.parent;
            if item.act_cnt == 1 {
                item.act_cnt = 0;
                if let Some(p) = parent {
                    self.list[p].act_cnt -= 1;
                }
            } else if item.act_cnt == 0 {
                item.act_cnt = 1;
                self.unmark_parents(parent);
            }

            Ok(())
        } else {
            Err("Can't mark item. List is empty.")
        }
    }

    fn transfer(&mut self, rhs: &mut Self) -> Result<(), &'static str> {
        assert!(!std::ptr::eq(self, rhs), "Can't transfer item to itself.");

        if let Some(item) = self.get_cur_item_mut() {
            if !item.is_root() {
                return Err("Can't transfer item. Item is a subtask.");
            } else if item.is_active() {
                return Err("Can't transfer item. Item is active.");
            }

            item.date = Local::now();
            let child_cnt = self.children_cnt(self.cur) + 1;
            let move_to = rhs.list.len();

            self.shift_indices(-(child_cnt as isize), self.cur + child_cnt, None, None);

            let to_transfer: Vec<Item> = self.list.drain(self.cur..self.cur + child_cnt).collect();
            rhs.list.append(&mut to_transfer.into());

            rhs.shift_indices(-(self.cur as isize) + move_to as isize, move_to, None, None);

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
        if let Some(item) = self.get_cur_item_mut() {
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
}

#[derive(Debug)]
pub struct TodoApp {
    message: String,
    panel: Panel,
    hide_subs: bool,
    operation_stack: Vec<Operation>,
    todos: List,
    dones: List,
}

impl TodoApp {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            panel: Panel::Todo,
            hide_subs: false,
            operation_stack: Vec::new(),
            todos: List::new(),
            dones: List::new(),
        }
    }

    pub fn is_in_todos(&self) -> bool {
        self.panel == Panel::Todo
    }

    pub fn is_subs_hidden(&self) -> bool {
        self.hide_subs
    }

    pub fn is_in_dones(&self) -> bool {
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
            skip_children: self.hide_subs,
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
            skip_children: self.hide_subs,
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

        let sep = SEP;
        let re_indent = Regex::new(r"^((\s{4})*)\S+").unwrap();
        let mut panel = Panel::Todo;

        let mut stack = Vec::new();
        let mut cnt_todos = 0;
        let mut cur_indent = 0;

        match file {
            Ok(file) => {
                for (i, line) in io::BufReader::new(file).lines().enumerate() {
                    let line = line.unwrap();
                    let parent: Option<usize>;

                    if line == sep {
                        if panel == Panel::Todo {
                            cur_indent = 0;
                            stack.clear();
                            panel = Panel::Done;
                        } else {
                            eprintln!("[ERROR]: {}:{}: invalid separator", file_path, i + 1);
                            exit(1);
                        }
                        continue;
                    }

                    if let Some(m) = re_indent.captures(&line) {
                        let indent = m[1].len() / INDENT_SIZE;
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
                        eprintln!("[ERROR]: {}:{}: invalid indentation", file_path, i + 1);
                        exit(1);
                    }

                    match panel {
                        Panel::Todo => match self.parse_todo(&line, parent) {
                            Err(e) => {
                                eprintln!("[ERROR]: {}:{}: {}", file_path, i + 1, e);
                                exit(1);
                            }
                            Ok(todo) => {
                                let active = todo.is_active();
                                cnt_todos += 1;

                                self.todos.add_item(todo);
                                if let Some(parent) = parent {
                                    self.todos.add_child_to(parent, i, active);
                                }
                            }
                        },
                        Panel::Done => {
                            let parent = parent.map(|p| p - cnt_todos - 1);
                            match self.parse_done(&line, parent) {
                                Err(e) => {
                                    eprintln!("[ERROR]: {}:{}: {}", file_path, i + 1, e);
                                    exit(1);
                                }
                                Ok(done) => {
                                    let i = i - cnt_todos - 1;
                                    self.dones.add_item(done);
                                    if let Some(parent) = parent {
                                        self.dones.add_child_to(parent, i, false);
                                    }
                                }
                            }
                        }
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

    fn parse_todo(&mut self, line: &str, parent: Option<usize>) -> Result<Item, &'static str> {
        let re_todo = Regex::new(r"^(\s{4})*TODO\((\*|)\): (.*)$").unwrap();

        if let Some(caps) = re_todo.captures(line) {
            let act_cnt = if caps[2].is_empty() { 0 } else { 1 };
            Ok(Item::new(
                caps[3].trim().to_string(),
                Local::now(),
                parent,
                act_cnt,
            ))
        } else {
            Err("invalid format for TODO item")
        }
    }

    fn parse_done(&mut self, line: &str, parent: Option<usize>) -> Result<Item, &str> {
        let re_done = Regex::new(r"^(\s{4})*DONE\((.*)\): (.*)$").unwrap();

        if let Some(caps) = re_done.captures(line) {
            let date = DateTime::parse_from_str(&caps[2], DATE_FMT);
            if let Ok(d) = date {
                Ok(Item::new(caps[3].trim().to_string(), d.into(), parent, 0))
            } else {
                Err("invalid date format for DONE item")
            }
        } else {
            Err("invalid format for DONE item")
        }
    }

    pub fn save(&mut self, file_path: &str) -> std::io::Result<()> {
        let sep = SEP;
        self.hide_subs = false;

        let mut file = File::create(file_path)?;
        for (todo, level) in self.iter_todos() {
            let indent = " ".repeat(level * INDENT_SIZE);
            let act = if todo.is_active() { "*" } else { "" };
            writeln!(file, "{indent}TODO({act}): {}", todo.text).unwrap();
        }

        writeln!(file, "{sep}").unwrap();

        for (done, level) in self.iter_dones() {
            let indent = " ".repeat(level * INDENT_SIZE);
            let date = done.date.format(DATE_FMT);
            writeln!(file, "{indent}DONE({date}): {}", done.text).unwrap();
        }

        Ok(())
    }

    pub fn toggle_panel(&mut self) {
        assert!(!self.is_in_edit(), "Can't toggle panel while in edit mode.");
        self.panel = self.panel.togle();
    }

    pub fn toggle_subtasks(&mut self) {
        assert!(!self.is_in_edit(), "Can't toggle hide while in edit mode.");
        self.hide_subs = !self.hide_subs;
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

    pub fn mark_item(&mut self) {
        assert!(!self.is_in_edit(), "Can't mark item while in edit mode");

        match self.panel {
            Panel::Todo => {
                self.todos.record_state();
                match self.todos.mark() {
                    Ok(()) => {
                        self.operation_stack
                            .push(Operation::new(Action::Mark, Panel::Todo));
                    }
                    Err(err) => {
                        self.message.push_str(err);
                        self.todos.revert_state().unwrap();
                    }
                };
            }
            Panel::Done => self
                .message
                .push_str("Can't mark done item, try transfering it first."),
        };
    }

    pub fn transfer_item(&mut self) {
        assert!(!self.is_in_edit(), "Can't transfer item while in edit mode");

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
                .push_str("Can't delete a TODO item. Transfer it to DONE first."),
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
                        self.todos.revert_state().unwrap();
                        self.dones.revert_state().unwrap();
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

        let editing_cursor = match self.panel {
            Panel::Todo => self.todos.get_cur_item().map_or(0, |item| item.text.len()),
            Panel::Done => self.dones.get_cur_item().map_or(0, |item| item.text.len()),
        };

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

            Some(editing_cursor)
        } else {
            self.message.push_str("Nothing to edit.");
            None
        }
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
                if let Some(item) = self.todos.get_cur_item() {
                    if item.text.is_empty() {
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
                    }
                    self.todos.get_cur_item_mut().unwrap().trim_text();
                }
            }
            Panel::Done => {
                if let Some(cur_done) = self.dones.get_cur_item_mut() {
                    if cur_done.text.is_empty() {
                        self.dones.delete().unwrap();
                    } else {
                        cur_done.trim_text();
                    }
                }
            }
        }

        self.operation_stack.pop();
        true
    }

    fn is_in_edit(&self) -> bool {
        if let Some(op) = self.operation_stack.last() {
            op.action == Action::InEdit
        } else {
            false
        }
    }
}
