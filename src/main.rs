extern crate termion;

use std::cmp::*;
use std::io::{stdin, stdout, Read, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode};
use termion::screen::{IntoAlternateScreen, AlternateScreen};
use termion::{color, style};

fn screen_highlight<W: Write>(s: &mut AlternateScreen<W>) {
    write!(
        s,
        "{}{}",
        color::Fg(color::Black),
        color::Bg(color::White)
    ).unwrap();
}

fn screen_write<W: Write>(s: &mut AlternateScreen<W>, text: &str, row: u16) {
    write!(s, "{}{}",  termion::cursor::Goto(1, (row + 1) as u16),  text).unwrap();
}

fn screen_style_reset<W: Write>(s: &mut AlternateScreen<W>) {
    write!(s, "{}", style::Reset).unwrap();
}

fn main() {
    let mut stdin = stdin();
    let mut screen = stdout()
        .into_raw_mode()
        .unwrap()
        .into_alternate_screen()
        .unwrap();

    write!(
        screen,
        "{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();
    screen.flush().unwrap();

    let mut cur_todo: usize = 0;
    let mut quit: bool = false;
    let todos: Vec<&str> = vec![
        "Finish Scancore",
        "Make a cup of tea",
        "Write a Rust TODO app",
    ];

    while !quit {
        for (row, todo) in todos.iter().enumerate() {
            if cur_todo == row { screen_highlight(&mut screen); }
            screen_write(&mut screen, todo, row as u16);
            screen_style_reset(&mut screen);
        }
        screen.flush().unwrap();

        if let Some(Ok(key)) = stdin.by_ref().keys().next() {
            match key {
                Key::Esc | Key::Char('q') => quit = true,
                Key::Up | Key::Char('w') => {
                    if cur_todo > 0 {
                        cur_todo -= 1;
                    }
                }
                Key::Down | Key::Char('s') => cur_todo = min(cur_todo + 1, todos.len() - 1),
                _ => {}
            }
        }
    }
}
