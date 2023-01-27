extern crate regex;
extern crate termion;
mod mods;

use std::env::args;
use std::io::stdin;
use std::process::exit;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::Local;

use termion::event::Key;
use termion::input::TermRead;
use termion::{color, terminal_size};

use mods::todo::*;
use mods::ui::*;

const MAX_STACK_SIZE: usize = 20;
const HIGHLIGHT_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::White);

fn main() {
    let mut args = args();
    args.next().unwrap();

    // Get file path
    let file_path = match args.next() {
        Some(file_path) => file_path,
        None => {
            eprintln!("Usage: todo <file>");
            eprintln!("[ERROR]: No file specified");
            exit(1);
        }
    };

    // Keyboard input thread that will poll for input and send it to the main thread
    let timeout = Duration::from_millis(100);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut keys = stdin().keys();
        while let Some(Ok(key)) = keys.next() {
            tx.send(key).unwrap();
        }
    });

    let mut quit: bool = false;
    let mut editing: bool = false;
    let mut list: List = List::new(&file_path);
    let mut ui = UI::new();

    // Main loop
    while !quit {
        let (width, _) = terminal_size().unwrap();

        ui.begin(Point::new(0, 0), LayoutKind::Vert);
        {
            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label(&format!("[MESSAGE]: {}", list.get_message()));
                }
                ui.end_layout();
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label(&format!(
                        "[DATE]: {}",
                        Local::now().format("%Y-%m-%d %H:%M:%S")
                    ));
                }
                ui.end_layout();
            }
            ui.end_layout();

            ui.label("");

            ui.begin_layout(LayoutKind::Horz);
            {
                let panel = list.get_panel();
                let editing_cursor = list.get_editing_cursor();
                let cur_todo = list.get_cur_todo();
                let cur_done = list.get_cur_done();

                ui.begin_layout(LayoutKind::Vert);
                {
                    if panel == Panel::Todo {
                        ui.label_styled("[TODO]", HIGHLIGHT_PAIR);
                    } else {
                        ui.label(" TODO ");
                    }
                    ui.label("-".repeat(width as usize / 2).as_str());
                    for (i, todo) in list.get_mut_todos().iter_mut().enumerate() {
                        if i == cur_todo && panel == Panel::Todo {
                            if editing {
                                ui.edit_label(
                                    &todo.text,
                                    editing_cursor,
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
                    for (i, done) in list.get_mut_dones().iter_mut().enumerate() {
                        if i == cur_done && panel == Panel::Done {
                            if editing {
                                ui.edit_label(
                                    &done.text,
                                    editing_cursor,
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
                list.clear_message();
                match key {
                    Key::Up | Key::Char('k') => list.go_up(),
                    Key::Down | Key::Char('j') => list.go_down(),
                    Key::Char('g') => list.go_top(),
                    Key::Char('G') => list.go_bottom(),
                    Key::Char('K') => list.drag_up(),
                    Key::Char('J') => list.drag_down(),
                    Key::Char('\n') => list.move_item(),
                    Key::Char('d') => list.delete_item(),
                    Key::Char('i') => editing = list.insert_item(),
                    Key::Char('r') => editing = list.edit_item(),
                    Key::Char('u') => list.undo(),
                    Key::Char('\t') => list.toggle_panel(),
                    Key::Char('q') | Key::Ctrl('c') => quit = true,
                    _ => {}
                }
            } else {
                match key {
                    Key::Char('\n') | Key::Esc => editing = list.finish_edit(),
                    _ => list.edit_item_with(Some(key)),
                }
            }
        }
    }

    ui.clear();
    list.save(&file_path);

    println!(
        "[INFO]: Goodbye, stranger! Your todo list is saved to '{}'.",
        file_path
    );
}
