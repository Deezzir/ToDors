extern crate regex;
extern crate termion;
mod mods;

use std::env::args;
use std::io::stderr;
use std::io::stdin;
use std::io::stdout;
use std::io::StderrLock;
use std::io::Write;
use std::process::exit;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::Local;

use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use mods::todo::*;
use mods::ui::*;

const SELECTED_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::Cyan);
const UNSELECTED_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::LightBlack);
const HIGHLIGHT_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::Black, &color::LightGreen);
const UI_PAIR: (&dyn color::Color, &dyn color::Color) = (&color::White, &color::Black);

const USAGE: &str = "Usage: todo [-f | --file <file>] [-h | --help]";

const HELP: &str = r#"ToDors - a simple todo list manager in terminal.
Author: Iurii Kondrakov <deezzir@gmail.com>

    Options:
        -f, --file <file>   The file to use for the todo list.
        -h, --help          Show this help message.

    Controls:
        <k/up>, <j/down>  ~ Move the cursor up
        <K>, <J>          ~ Drag item UP/DOWN
        <g>, <G>          ~ Go to the top/bottom of the list
        <d>               ~ Delete 'Done' element
        <i>               ~ Insert a new 'Todo' element
        <u>               ~ Undo last action
        <r>               ~ Edit current item
        <enter>           ~ Transfer current elemen/Save edited element
        <esc>             ~ Cancel editing
        <tab>             ~ Switch between Switch between 'Todos'/'Dones'
        <q>, <ctrl-c>     ~ Quit
"#;

const FILE_PATH: &str = "TODO";

fn main() {
    let stdout = stdout();
    let stdout = stdout.lock();
    let stderr = stderr();
    let stderr = stderr.lock();
    let file_path: String = get_args(stderr);

    let timeout = Duration::from_millis(16);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = stdin();
        let stdin = stdin.lock();
        let mut stdin = stdin.keys();

        while let Some(Ok(key)) = stdin.next() {
            tx.send(key).unwrap();
        }
    });

    let mut editing: bool = false;
    let mut editing_cursor: usize = 0;
    let mut app: TodoApp = TodoApp::new();

    let stdout_raw = stdout.into_raw_mode().unwrap();
    let mut ui = UI::new(stdout_raw);

    app.parse(&file_path);

    loop {
        let date = Local::now().format("%Y-%m-%d %H:%M:%S");

        ui.begin(Vec2::new(0, 0), LayoutKind::Vert);
        {
            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_styled(&format!("[MESSAGE]: {}", app.get_message()), UI_PAIR);
                }
                ui.end_layout();
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_styled(&format!("[DATE]: {}", date), UI_PAIR);
                }
                ui.end_layout();
            }
            ui.end_layout();

            ui.br();

            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    if app.is_in_todo_panel() {
                        ui.label_styled("[TODO]", HIGHLIGHT_PAIR);
                    } else {
                        ui.label_styled(" TODO ", UNSELECTED_PAIR);
                    }
                    ui.hl();
                    for todo in app.get_todos() {
                        if app.is_cur_todo(todo) {
                            if app.is_in_todo_panel() {
                                if editing {
                                    ui.edit_label(
                                        todo.get_text(),
                                        editing_cursor,
                                        "- [ ] ".to_string(),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("- [ ] {}", todo.get_text()),
                                        SELECTED_PAIR,
                                    );
                                }
                            } else {
                                ui.label_styled(
                                    &format!("- [ ] {}", todo.get_text()),
                                    UNSELECTED_PAIR,
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
                        ui.label_styled(" DONE ", UNSELECTED_PAIR);
                    }
                    ui.hl();
                    for done in app.get_dones() {
                        if app.is_cur_done(done) {
                            if app.is_in_done_panel() {
                                if editing {
                                    ui.edit_label(
                                        done.get_text(),
                                        editing_cursor,
                                        "- [X] ".to_string(),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("- [X] ({}) {}", done.get_date(), done.get_text()),
                                        SELECTED_PAIR,
                                    );
                                }
                            } else {
                                ui.label_styled(
                                    &format!("- [X] ({}) {}", done.get_date(), done.get_text()),
                                    UNSELECTED_PAIR,
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
            use Key::*;

            if !editing {
                app.clear_message();
                match key {
                    Up | Char('k') => app.go_up(),
                    Down | Char('j') => app.go_down(),
                    Char('g') => app.go_top(),
                    Char('G') => app.go_bottom(),
                    Char('K') => app.drag_up(),
                    Char('J') => app.drag_down(),
                    Char('\n') => app.move_item(),
                    Char('d') => app.delete_item(),
                    Char('i') => {
                        editing_cursor = app.insert_item();
                        editing = editing_cursor == 0;
                    }
                    Char('r') => {
                        editing_cursor = app.edit_item();
                        editing = editing_cursor > 0;
                    }
                    Char('u') => app.undo(),
                    Char('\t') => app.toggle_panel(),
                    Char('q') | Key::Ctrl('c') => break,
                    _ => {}
                }
            } else {
                match key {
                    Char('\n') | Esc => {
                        editing = app.finish_edit();
                        editing_cursor = if editing { editing_cursor } else { 0 };
                    }
                    _ => app.edit_item_with(&mut editing_cursor, Some(key)),
                }
            }
        }
    }

    app.save(&file_path).unwrap();
}

fn get_args(mut stderr: StderrLock) -> String {
    let mut args = args().skip(1);

    match args.next() {
        Some(arg) => match arg.as_str() {
            "-f" | "--file" => args.next().unwrap_or_else(|| {
                stderr
                    .write_all(format!("[ERROR]: No file given for '{arg}'.\n").as_bytes())
                    .unwrap();
                stderr.write_all(USAGE.as_bytes()).unwrap();
                stderr.flush().unwrap();
                exit(1);
            }),
            "-h" | "--help" => {
                stderr
                    .write_all(format!("{HELP}\n{USAGE}\n").as_bytes())
                    .unwrap();
                stderr.flush().unwrap();
                exit(0);
            }
            _ => {
                stderr
                    .write_all(format!("[ERROR]: Unknown argument: '{arg}'.\n").as_bytes())
                    .unwrap();
                stderr.write_all(USAGE.as_bytes()).unwrap();
                stderr.flush().unwrap();
                exit(1);
            }
        },
        None => FILE_PATH.to_string(),
    }
}
