use std::env::args;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};

use ncurses::*;

use crate::{FILE_PATH, HELP, HIGHLIGHT_PAIR, SELECTED_PAIR, UI_PAIR, UNSELECTED_PAIR, USAGE};

pub static QUIT: AtomicBool = AtomicBool::new(false);

pub fn set_sig_handler() {
    ctrlc::set_handler(move || QUIT.store(true, Ordering::SeqCst))
        .expect("[ERROR]: Failed to set Ctrl-C handler");
}

pub fn ncurses_init() {
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

pub fn get_args() -> String {
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
