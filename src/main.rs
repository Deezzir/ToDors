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
    let mut editing_cursor: usize = 0;
    let mut app: TodoApp = TodoApp::new();
    let mut ui = UI::new();

    app.parse(&file_path);

    // Main loop
    while !quit {
        let (width, _) = terminal_size().unwrap();

        ui.begin(Point::new(0, 0), LayoutKind::Vert);
        {
            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label(&format!("[MESSAGE]: {}", app.get_message()));
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
                ui.begin_layout(LayoutKind::Vert);
                {
                    if app.is_in_todo_panel() {
                        ui.label_styled("[TODO]", HIGHLIGHT_PAIR);
                    } else {
                        ui.label(" TODO ");
                    }
                    ui.label("-".repeat(width as usize / 2).as_str());
                    for todo in app.get_todos() {
                        if app.is_cur_todo(todo) && app.is_in_todo_panel() {
                            if editing {
                                ui.edit_label(
                                    todo.get_text(),
                                    editing_cursor,
                                    "- [ ] ".to_string(),
                                );
                            } else {
                                ui.label_styled(
                                    &format!("- [ ] {}", todo.get_text()),
                                    HIGHLIGHT_PAIR,
                                );
                            }
                        } else {
                            ui.label(&format!("- [ ] {}", todo.get_text()));
                        }
                    }
                }
                ui.end_layout();

                ui.begin_layout(LayoutKind::Vert);
                {
                    if app.is_in_done_panel() {
                        ui.label_styled("[DONE]", HIGHLIGHT_PAIR);
                    } else {
                        ui.label(" DONE ");
                    }
                    ui.label("-".repeat(width as usize / 2).as_str());
                    for done in app.get_dones() {
                        if app.is_cur_done(done) && app.is_in_done_panel() {
                            if editing {
                                ui.edit_label(
                                    done.get_text(),
                                    editing_cursor,
                                    "- [X] ".to_string(),
                                );
                            } else {
                                ui.label_styled(
                                    &format!("- [X] ({}) {}", done.get_date(), done.get_text()),
                                    HIGHLIGHT_PAIR,
                                );
                            }
                        } else {
                            ui.label(&format!("- [X] ({}) {}", done.get_date(), done.get_text()));
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
                app.clear_message();
                match key {
                    Key::Up | Key::Char('k') => app.go_up(),
                    Key::Down | Key::Char('j') => app.go_down(),
                    Key::Char('g') => app.go_top(),
                    Key::Char('G') => app.go_bottom(),
                    Key::Char('K') => app.drag_up(),
                    Key::Char('J') => app.drag_down(),
                    Key::Char('\n') => app.move_item(),
                    Key::Char('d') => app.delete_item(),
                    Key::Char('i') => {
                        app.insert_item();
                        editing_cursor = app.edit_item();
                        editing = editing_cursor == 0;
                    }
                    Key::Char('r') => {
                        editing_cursor = app.edit_item();
                        editing = editing_cursor > 0;
                    }
                    Key::Char('u') => app.undo(),
                    Key::Char('\t') => app.toggle_panel(),
                    Key::Char('q') | Key::Ctrl('c') => quit = true,
                    _ => {}
                }
            } else {
                match key {
                    Key::Char('\n') | Key::Esc => {
                        app.finish_edit();
                        editing = false;
                        editing_cursor = 0;
                    }
                    _ => app.edit_item_with(&mut editing_cursor, Some(key)),
                }
            }
        }
    }

    ui.clear();
    app.save(&file_path).unwrap();

    print!(
        "[INFO]: Goodbye, stranger! Your todo app is saved to '{}'.",
        file_path
    );
}
