extern crate regex;
mod mods;

use chrono::Local;
use std::path::Path;

use ncurses::*;

use mods::todo::*;
use mods::ui::*;
use mods::utils::*;

const TIMEOUT: i32 = 1000; // 1 second
const FPS: i32 = 30;

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
        <k/up>, <j/down>                ~ Move the cursor up
        <K/shift+up>, <J/shift+down>    ~ Drag item UP/DOWN
        <g>, <G>, <h>                   ~ Jump to the top/bottom/half of the list
        <d>                             ~ Delete 'Done' item
        <i>                             ~ Insert a new 'Todo' item
        <a>                             ~ Add a subtask to the current 'Todo' item
        <u>                             ~ Undo last action
        <r>                             ~ Edit current item
        <t>                             ~ Hide subtasks
        <?>                             ~ Show help
        <enter>                         ~ Transfer current item/Save edited item
        <esc>                           ~ Cancel editing
        <tab>                           ~ Switch between Switch between 'Todos'/'Dones'
        <q>, <ctrl+c>                   ~ Quit
"#;

const FILE_PATH: &str = "TODO.list";

#[derive(PartialEq)]
enum Mode {
    Edit,
    Normal,
}

#[allow(dead_code)]
enum Display {
    All(Mode),
    Hide(Mode),
    Help,
}

fn main() {
    sig_handler_init();

    let file_path: String = get_args();
    let file_name: String = Path::new(&file_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let mut editing_cursor: usize = 0;
    let mut term_size = Vec2::new(0, 0);
    let mut timeout = 0;
    let mut mode: Mode = Mode::Normal;
    let mut app: TodoApp = TodoApp::new();
    let mut ui = UI::new();
    app.parse(&file_path);

    ncurses_init();

    while !poll() {
        getmaxyx(stdscr(), &mut term_size.y, &mut term_size.x);

        if timeout <= 0 {
            erase();
            let date = Local::now().format("%Y %a %b %d %H:%M:%S");

            ui.begin(Vec2::new(0, 0), LayoutKind::Vert, term_size);
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
                            Some(A_BOLD()),
                        );
                        ui.label_styled(
                            &format!("[MESSAGE]: {}", app.get_message()),
                            UI_PAIR,
                            Some(A_BOLD()),
                        );
                    }
                    ui.end_layout();

                    ui.begin_layout(LayoutKind::Vert);
                    {
                        ui.label_styled(&format!("[DATE]: {date}"), UI_PAIR, Some(A_BOLD()));
                        ui.label_styled(&format!("[FILE]: {file_name}"), UI_PAIR, Some(A_BOLD()));
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
                            ui.label_styled("[TODO]", HIGHLIGHT_PAIR, None);
                        } else {
                            ui.label_styled(" TODO ", UNSELECTED_PAIR, None);
                        }
                        ui.hl();

                        for (todo, level) in app.iter_todos() {
                            let indent = "    ".repeat(level);
                            let prefix = if todo.is_active() { "[ ] " } else { "[X] " };
                            let text = todo.get_text();
                            let _date = todo.get_date();

                            if app.is_cur_todo(todo) && app.is_in_todo_panel() {
                                if mode == Mode::Edit {
                                    ui.edit_label(
                                        text,
                                        editing_cursor,
                                        format!("{indent}{prefix}"),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("{indent}{prefix}{text}"),
                                        SELECTED_PAIR,
                                        None,
                                    );
                                }
                            } else {
                                ui.label(&format!("{indent}{prefix}{text}"));
                            }
                        }
                    }
                    ui.end_layout();

                    ui.begin_layout(LayoutKind::Vert);
                    {
                        if app.is_in_done_panel() {
                            ui.label_styled("[DONE]", HIGHLIGHT_PAIR, None);
                        } else {
                            ui.label_styled(" DONE ", UNSELECTED_PAIR, None);
                        }
                        ui.hl();

                        for (done, level) in app.iter_dones() {
                            let indent = "    ".repeat(level);
                            let prefix = if !done.is_active() { "[X]" } else { panic!() };
                            let text = done.get_text();
                            let date = done.get_date();

                            if app.is_cur_done(done) && app.is_in_done_panel() {
                                if mode == Mode::Edit {
                                    ui.edit_label(
                                        text,
                                        editing_cursor,
                                        format!("{indent}{prefix}"),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("{prefix} ({date}) {text}"),
                                        SELECTED_PAIR,
                                        None,
                                    );
                                }
                            } else {
                                ui.label(&format!("{prefix}|{date}| {text}"));
                            }
                        }
                    }
                    ui.end_layout();
                }
                ui.end_layout();
            }
            ui.end();
            timeout = TIMEOUT;
        }

        refresh();
        let key = getch();
        if key != ERR {
            match mode {
                Mode::Normal => {
                    app.clear_message();
                    match char::from_u32(key as u32).unwrap() {
                        'k' | '\u{103}' => app.go_up(),     // 'k' or 'up'
                        'j' | '\u{102}' => app.go_down(),   // 'j' or 'down'
                        'K' | '\u{151}' => app.drag_up(),   // 'K' or 'shift+up'
                        'J' | '\u{150}' => app.drag_down(), // 'J' or 'shift+down'
                        'g' => app.go_top(),
                        'G' => app.go_bottom(),
                        'h' => app.go_half(),
                        '\n' => app.mark_item(),
                        'd' => app.delete_item(),
                        'u' => app.undo(),
                        '\t' => app.toggle_panel(),
                        't' => todo!(),
                        '?' => todo!(),
                        'i' => {
                            if let Some(cur) = app.insert_item() {
                                editing_cursor = cur;
                                mode = Mode::Edit;
                            }
                        }
                        'a' => {
                            if let Some(cur) = app.append_item() {
                                editing_cursor = cur;
                                mode = Mode::Edit;
                            }
                        }
                        'r' => {
                            if let Some(cur) = app.edit_item() {
                                editing_cursor = cur;
                                mode = Mode::Edit;
                            }
                        }
                        'q' => break,
                        _ => {}
                    }
                }
                Mode::Edit => {
                    match key as u8 as char {
                        '\n' | '\u{1b}' => {
                            // Enter or Esc
                            mode = if app.finish_edit() {
                                editing_cursor = 0;
                                Mode::Normal
                            } else {
                                Mode::Edit
                            };
                        }
                        _ => app.edit_item_with(&mut editing_cursor, key),
                    }
                }
            }
            timeout = 0;
        } else {
            timeout -= 1000 / FPS;
        }
    }

    endwin();
    app.save(&file_path).unwrap();

    println!("[INFO]: Saved to '{file_path}', Bye!");
    // println!("{app:#?}");
}
