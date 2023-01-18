extern crate regex;
extern crate termion;
mod todo;

use std::env::args;
use std::io::{self, stdin};
use std::process::exit;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::Local;

use termion::event::Key;
use termion::input::TermRead;
use termion::terminal_size;

use todo::list::*;
use todo::ui::*;

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

    let mut operation_stack: Vec<Operation> = Vec::new();

    let mut todos: Vec<Item> = Vec::<Item>::new();
    let mut todo_stack: Vec<Vec<Item>> = Vec::new();
    let mut cur_todo: usize = 0;

    let mut dones: Vec<Item> = Vec::<Item>::new();
    let mut dones_stack: Vec<Vec<Item>> = Vec::new();
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
                    Key::Char('g') => match panel {
                        Panel::Todo => list_first(&mut cur_todo),
                        Panel::Done => list_first(&mut cur_done),
                    },
                    Key::Char('G') => match panel {
                        Panel::Todo => list_last(&todos, &mut cur_todo),
                        Panel::Done => list_last(&dones, &mut cur_done),
                    },
                    Key::Char('K') => match panel {
                        Panel::Todo => {
                            list_record_state(&mut todo_stack, &todos);
                            match list_drag_up(&mut todos, &mut cur_todo) {
                                Ok(()) => operation_stack.push(Operation::new(
                                    Action::DragUp,
                                    cur_todo + 1,
                                    Panel::Todo,
                                )),
                                Err(err) => {
                                    message.push_str(err);
                                    list_revert_state(&mut todo_stack, &mut todos).unwrap();
                                }
                            };
                        }
                        Panel::Done => {
                            list_record_state(&mut dones_stack, &dones);
                            match list_drag_up(&mut dones, &mut cur_done) {
                                Ok(()) => operation_stack.push(Operation::new(
                                    Action::DragUp,
                                    cur_done + 1,
                                    Panel::Done,
                                )),
                                Err(err) => {
                                    message.push_str(err);
                                    list_revert_state(&mut dones_stack, &mut dones).unwrap();
                                }
                            };
                        }
                    },
                    Key::Char('J') => match panel {
                        Panel::Todo => {
                            list_record_state(&mut todo_stack, &todos);
                            match list_drag_down(&mut todos, &mut cur_todo) {
                                Ok(()) => operation_stack.push(Operation::new(
                                    Action::DragDown,
                                    cur_todo - 1,
                                    Panel::Todo,
                                )),
                                Err(err) => {
                                    message.push_str(err);
                                    list_revert_state(&mut todo_stack, &mut todos).unwrap();
                                }
                            };
                        }
                        Panel::Done => {
                            list_record_state(&mut dones_stack, &dones);
                            match list_drag_down(&mut dones, &mut cur_done) {
                                Ok(()) => operation_stack.push(Operation::new(
                                    Action::DragDown,
                                    cur_done - 1,
                                    Panel::Done,
                                )),
                                Err(err) => {
                                    message.push_str(err);
                                    list_revert_state(&mut dones_stack, &mut dones).unwrap();
                                }
                            };
                        }
                    },
                    Key::Char('\n') => {
                        list_record_state(&mut todo_stack, &todos);
                        list_record_state(&mut dones_stack, &dones);

                        let op = match panel {
                            Panel::Todo => {
                                todos[cur_todo].date = Local::now().format("%y-%m-%d").to_string();
                                list_move(&mut todos, &mut dones, &mut cur_todo)
                            }
                            Panel::Done => {
                                dones[cur_done].date = String::new();
                                list_move(&mut dones, &mut todos, &mut cur_done)
                            }
                        };
                        match op {
                            Ok(()) => {
                                if panel == Panel::Todo {
                                    operation_stack.push(Operation::new(
                                        Action::Move,
                                        cur_done,
                                        Panel::Todo,
                                    ));
                                    message.push_str("Done! Great job!");
                                } else {
                                    operation_stack.push(Operation::new(
                                        Action::Move,
                                        cur_done,
                                        Panel::Done,
                                    ));
                                    message.push_str("Not done yet? Keep going!");
                                }
                            }
                            Err(err) => {
                                message.push_str(err);
                                list_revert_state(&mut todo_stack, &mut todos).unwrap();
                                list_revert_state(&mut dones_stack, &mut dones).unwrap();
                            }
                        };
                    }
                    Key::Char('d') => match panel {
                        Panel::Todo => {
                            message.push_str("Can't delete a TODO item. Mark it as DONE first.")
                        }
                        Panel::Done => {
                            list_record_state(&mut dones_stack, &dones);
                            match list_delete(&mut dones, &mut cur_done) {
                                Ok(()) => {
                                    message.push_str("DONE item deleted.");
                                    operation_stack.push(Operation::new(
                                        Action::Delete,
                                        cur_done,
                                        Panel::Done,
                                    ));
                                }
                                Err(err) => {
                                    message.push_str(err);
                                    list_revert_state(&mut dones_stack, &mut dones).unwrap();
                                }
                            };
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
                    Key::Char('u') => {
                        let op = operation_stack.pop();
                        match op {
                            Some(op) => {
                                match op.action {
                                    Action::Move => {
                                        list_revert_state(&mut dones_stack, &mut dones).unwrap();
                                        list_revert_state(&mut todo_stack, &mut todos).unwrap();
                                    }
                                    _ => match op.panel {
                                        Panel::Todo => {
                                            list_revert_state(&mut todo_stack, &mut todos).unwrap();
                                        }
                                        Panel::Done => {
                                            list_revert_state(&mut dones_stack, &mut dones)
                                                .unwrap();
                                        }
                                    },
                                }
                                if op.panel == Panel::Todo {
                                    cur_todo = op.cur;
                                } else {
                                    cur_done = op.cur;
                                }
                                panel = op.panel;
                                message.push_str(&format!("Undo: {}", op.action));
                            }
                            None => message.push_str("Nothing to undo."),
                        }
                    }
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
                            // TODO: this will remove the TODO if it's empty, however its not allowed
                            Panel::Todo => {
                                if todos[cur_todo].text.is_empty() {
                                    list_delete(&mut todos, &mut cur_todo).unwrap();
                                }
                            }
                            Panel::Done => {
                                if dones[cur_done].text.is_empty() {
                                    list_delete(&mut dones, &mut cur_done).unwrap();
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
