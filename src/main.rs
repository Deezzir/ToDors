extern crate regex;
mod mods;

use chrono::Local;
use std::path::Path;

use ncurses::*;

use mods::todo::*;
use mods::ui::*;
use mods::utils::*;

const TIMEOUT: i32 = 1000; // 1 second
const FPS: i32 = 30; // 30 frames per second

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

    ncurses_init();

    let mut editing_cursor: usize = 0;
    let mut term_size = Vec2::new(0, 0);
    let mut timeout = 0;

    let mut mode: Mode = Mode::Normal;
    let mut app: TodoApp = TodoApp::new();
    let mut ui = UI::new();

    app.parse(&file_path);
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
                            if app.is_cur_todo(todo) && app.is_in_todo_panel() {
                                if mode == Mode::Edit {
                                    ui.edit_label(
                                        todo.get_text(),
                                        editing_cursor,
                                        format!("{indent}[ ] "),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("{indent}[ ] {}", todo.get_text()),
                                        SELECTED_PAIR,
                                        None,
                                    );
                                }
                            } else {
                                ui.label(&format!("{indent}[ ] {}", todo.get_text()));
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
                            if app.is_cur_done(done) && app.is_in_done_panel() {
                                if mode == Mode::Edit {
                                    ui.edit_label(
                                        done.get_text(),
                                        editing_cursor,
                                        format!("{indent}[X] "),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("[X] ({}) {}", done.get_date(), done.get_text()),
                                        SELECTED_PAIR,
                                        None,
                                    );
                                }
                            } else {
                                ui.label(&format!("[X]|{}| {}", done.get_date(), done.get_text()));
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
                        'k' | '\u{103}' => app.go_up(),   // 'k' or 'up'
                        'j' | '\u{102}' => app.go_down(), // 'j' or 'down'
                        'K' | '\u{151}' => app.drag_up(),
                        'J' | '\u{150}' => app.drag_down(),
                        'g' => app.go_top(),
                        'G' => app.go_bottom(),
                        'h' => app.go_half(),
                        '\n' => app.transfer_item(),
                        'd' => app.delete_item(),
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
                        'u' => app.undo(),
                        '\t' => app.toggle_panel(),
                        't' => todo!(),
                        '?' => todo!(),
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
    println!("{app:#?}");
}
