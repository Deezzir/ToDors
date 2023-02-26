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
const INDENT_SIZE: usize = 4;

const SELECTED_PAIR: i16 = 1;
const UNSELECTED_PAIR: i16 = 2;
const HIGHLIGHT_PAIR: i16 = 3;
const UI_PAIR: i16 = 4;
const HELP_PAIR: i16 = 5;

const USAGE: &str = "Usage: todors [-f | --file <file>] [-h | --help]";
const HELP: &str = r#"ToDors - a simple todo list manager in terminal.
Author: Iurii Kondrakov <deezzir@gmail.com>

    Options:
        -f, --file <file>   The file to use for the todo list.
        -h, --help          Show this help message.

    Controls:
        <k/up>, <j/down>                ~ Move the cursor UP/DOWN
        <K/shift+up>, <J/shift+down>    ~ Drag item UP/DOWN
        <g>, <G>, <h>                   ~ Jump to the TOP/BOTTOM/HALF of the list
        <d>                             ~ Delete 'Done' item/subtask
        <i>                             ~ Insert a new 'Todo' item
        <a>                             ~ Add a subtask to the current 'Todo' item
        <u>                             ~ Undo last action
        <r>                             ~ Edit current item
        <t>                             ~ Hide subtasks
        <?>                             ~ Show help
        <space>                         ~ Mark current item as 'Done'
        <enter>                         ~ Transfer item/Save edited item
        <esc>                           ~ Cancel editing/inserting
        <tab>                           ~ Switch between 'Todos'/'Dones'
        <q>, <ctrl+c>                   ~ Quit
"#;

const FILE_PATH: &str = "TODO.list";

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Edit,
    Normal,
}

enum Display {
    App,
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
    let mut disp: Display = Display::App;
    let mut ui = UI::new();

    let mut app: TodoApp = TodoApp::new();
    app.parse(&file_path);

    ncurses_init();

    while !ctrlc_poll() {
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

                match disp {
                    Display::App => display_app(&mut ui, &mut app, mode, editing_cursor),
                    Display::Help => display_help(&mut ui),
                }
            }
            ui.end();

            timeout = TIMEOUT;
        }

        refresh();

        let key = getch();
        if key != ERR {
            match disp {
                Display::App => {
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
                                ' ' => app.mark_item(),
                                '\n' => app.transfer_item(),
                                'd' => app.delete_item(),
                                'u' => app.undo(),
                                '\t' => app.toggle_panel(),
                                't' => app.toggle_subtasks(),
                                '?' => disp = Display::Help,
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
                }
                Display::Help => match char::from_u32(key as u32).unwrap() {
                    ' ' => disp = Display::App,
                    'q' => break,
                    _ => {}
                },
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

fn prefix(subs_hidden: bool, has_children: bool, active: bool) -> &'static str {
    match (subs_hidden, has_children, active) {
        (true, true, true) => "[+]",
        (true, true, false) => "[-]",
        (_, _, true) => "[ ]",
        (_, _, false) => "[X]",
    }
}

fn display_app(ui: &mut UI, app: &mut TodoApp, mode: Mode, editing_cursor: usize) {
    ui.begin_layout(LayoutKind::Horz);
    {
        ui.begin_layout(LayoutKind::Vert);
        {
            if app.is_in_todos() {
                ui.label_styled("[TODO]", HIGHLIGHT_PAIR, None);
            } else {
                ui.label_styled(" TODO ", UNSELECTED_PAIR, None);
            }
            ui.hl();

            for (todo, level) in app.iter_todos() {
                let indent = " ".repeat(level * INDENT_SIZE);
                let prefix = prefix(app.is_subs_hidden(), todo.has_children(), todo.is_active());
                let text = todo.get_text();

                if app.is_cur_todo(todo) && app.is_in_todos() {
                    if mode == Mode::Edit {
                        ui.edit_label(text, editing_cursor, format!("{indent}{prefix} "));
                    } else {
                        ui.label_styled(&format!("{indent}{prefix} {text}"), SELECTED_PAIR, None);
                    }
                } else {
                    ui.label(&format!("{indent}{prefix} {text}"));
                }
            }
        }
        ui.end_layout();

        ui.begin_layout(LayoutKind::Vert);
        {
            if app.is_in_dones() {
                ui.label_styled("[DONE]", HIGHLIGHT_PAIR, None);
            } else {
                ui.label_styled(" DONE ", UNSELECTED_PAIR, None);
            }
            ui.hl();

            for (done, level) in app.iter_dones() {
                let indent = " ".repeat(level * INDENT_SIZE);
                let prefix = prefix(app.is_subs_hidden(), done.has_children(), done.is_active());
                let text = done.get_text();
                let date = if done.is_root() {
                    format!("({})", done.get_date())
                } else {
                    String::new()
                };

                if app.is_cur_done(done) && app.is_in_dones() {
                    if mode == Mode::Edit {
                        ui.edit_label(text, editing_cursor, format!("{indent}{prefix} "));
                    } else {
                        ui.label_styled(
                            &format!("{indent}{prefix}{date} {text}"),
                            SELECTED_PAIR,
                            None,
                        );
                    }
                } else {
                    ui.label(&format!("{indent}{prefix}{date} {text}"));
                }
            }
        }
        ui.end_layout();
    }
    ui.end_layout();
}

fn display_help(ui: &mut UI) {
    ui.label_styled("CONTROLS", UNSELECTED_PAIR, None);
    ui.hl();

    ui.begin_layout(LayoutKind::Horz);
    {
        ui.begin_layout(LayoutKind::Vert);
        {
            ui.label_styled("k/↑, j/↓", HELP_PAIR, None);
            ui.label("K/SHIFT+↑, J/SHIFT+↓");
            ui.label_styled("g, G, h", HELP_PAIR, None);
            ui.label("d");
            ui.label_styled("i", HELP_PAIR, None);
            ui.label("a");
            ui.label_styled("u", HELP_PAIR, None);
            ui.label("r");
            ui.label_styled("t", HELP_PAIR, None);
            ui.label("?");
            ui.label_styled("SPACE", HELP_PAIR, None);
            ui.label("ENTER");
            ui.label_styled("ESC", HELP_PAIR, None);
            ui.label("TAB");
            ui.label_styled("q/CTRL+c", HELP_PAIR, None);
        }
        ui.end_layout();

        ui.begin_layout(LayoutKind::Vert);
        {
            ui.label_styled("Move the cursor UP/DOWN", HELP_PAIR, None);
            ui.label("Drag item UP/DOWN");
            ui.label_styled("Jump to the TOP/BOTTOM/HALF of the list", HELP_PAIR, None);
            ui.label("Delete 'Done' item/subtask");
            ui.label_styled("Insert a new 'Todo' item", HELP_PAIR, None);
            ui.label("Add a subtask to the current 'Todo' item");
            ui.label_styled("Undo last action", HELP_PAIR, None);
            ui.label("Edit current item");
            ui.label_styled("Hide subtasks", HELP_PAIR, None);
            ui.label("Show this help");
            ui.label_styled("Mark current item as 'Done'", HELP_PAIR, None);
            ui.label("Transfer item/Save edited item");
            ui.label_styled("Cancel editing/inserting", HELP_PAIR, None);
            ui.label("Switch between 'Todos'/'Dones'");
            ui.label_styled("Quit", HELP_PAIR, None);
        }
        ui.end_layout();
    }
    ui.end_layout();

    ui.br();
    ui.hl();
    ui.label("Press SPACE to continue...");
}
