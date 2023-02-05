extern crate regex;
mod mods;

use chrono::Local;
use std::env::args;
use std::process::exit;

use ncurses::*;

use mods::todo::*;
use mods::ui::*;

const SELECTED_PAIR: i16 = 1;
const UNSELECTED_PAIR: i16 = 2;
const HIGHLIGHT_PAIR: i16 = 3;
const UI_PAIR: i16 = 4;

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
    let file_path: String = get_args();

    ncurses_init();
    let mut editing: bool = false;
    let mut editing_cursor: usize = 0;
    let mut app: TodoApp = TodoApp::new();
    let mut ui = UI::new();

    app.parse(&file_path);
    loop {
        erase();
        let date = Local::now().format("%Y %a %b %d %H:%M:%S");
        let mut w = 0;
        let mut h = 0;
        getmaxyx(stdscr(), &mut h, &mut w);

        ui.begin(Vec2::new(0, 0), LayoutKind::Vert, Vec2::new(w, h));
        {
            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_styled(
                        &format!(
                            "[CONTENT]: ({})todos and ({})dones",
                            app.get_todos_n(),
                            app.get_dones_n()
                        ),
                        UI_PAIR,
                    );
                    ui.label_styled(&format!("[MESSAGE]: {}", app.get_message()), UI_PAIR);
                }
                ui.end_layout();
                
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_styled(&format!("[DATE]: {date}"), UI_PAIR);
                    ui.label_styled(&format!("[FILE]: {file_path}"), UI_PAIR);
                }
                ui.end_layout();
            }
            ui.end_layout();

            ui.hl();
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

        refresh();
        let key = getch();
        if key != ERR {
            if !editing {
                app.clear_message();
                match char::from_u32(key as u32).unwrap() {
                    'k' | '\u{103}' => app.go_up(),
                    'j' | '\u{102}' => app.go_down(),
                    'g' => app.go_top(),
                    'G' => app.go_bottom(),
                    'K' => app.drag_up(),
                    'J' => app.drag_down(),
                    '\n' => app.move_item(),
                    'd' => app.delete_item(),
                    'i' => {
                        editing_cursor = app.insert_item();
                        editing = editing_cursor == 0;
                    }
                    'r' => {
                        editing_cursor = app.edit_item();
                        editing = editing_cursor > 0;
                    }
                    'u' => app.undo(),
                    '\t' => app.toggle_panel(),
                    'q' | '\u{3}' => break,
                    _ => {}
                }
            } else {
                match key as u8 as char {
                    '\n' | '\u{1b}' => {
                        editing = app.finish_edit();
                        editing_cursor = if editing { editing_cursor } else { 0 };
                    }
                    _ => app.edit_item_with(&mut editing_cursor, key),
                }
            }
        }
    }

    endwin();
    app.save(&file_path).unwrap();
}

fn ncurses_init() {
    setlocale(LcCategory::all, "");
    // Init ncurses
    initscr();
    raw();
    // Allow for extended keyboard (like F1).
    noecho();
    keypad(stdscr(), true);
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    // Set timeout and esc delay
    timeout(16);
    set_escdelay(0);
    // Set colors
    use_default_colors();
    start_color();
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_GREEN);
    init_pair(SELECTED_PAIR, COLOR_BLACK, COLOR_CYAN);
    init_pair(UNSELECTED_PAIR, COLOR_BLACK, COLOR_WHITE);
    init_pair(UI_PAIR, COLOR_WHITE, COLOR_BLACK);
}

fn get_args() -> String {
    let mut args = args().skip(1);

    match args.next() {
        Some(arg) => match arg.as_str() {
            "-f" | "--file" => args.next().unwrap_or_else(|| {
                eprintln!("[ERROR]: No file given for '{arg}'.");
                eprintln!("{USAGE}");
                exit(1);
            }),
            "-h" | "--help" => {
                println!("{HELP}\n{USAGE}");
                exit(0);
            }
            _ => {
                eprintln!("[ERROR]: Unknown argument: '{arg}'.");
                eprintln!("{USAGE}");
                exit(1);
            }
        },
        None => FILE_PATH.to_string(),
    }
}
