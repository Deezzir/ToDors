use std::io::Write;
use termion::{clear, color, cursor, style};

pub fn term_set_color_str(fg: &dyn color::Color, bg: Option<&dyn color::Color>) -> String {
    let mut s = String::new();
    s.push_str(&format!("{}", color::Fg(fg)));
    if let Some(bg) = bg {
        s.push_str(&format!("{}", color::Bg(bg)));
    }
    s
}

pub fn term_goto_str(pos: (u16, u16)) -> String {
    format!("{}", cursor::Goto(pos.1 + 1, pos.0 + 1))
}

pub fn term_clear_afer_cursor_str() -> String {
    format!("{}", clear::AfterCursor)
}

pub fn term_style_reset_str() -> String {
    format!("{}", style::Reset)
}

pub fn term_reset_str() -> String {
    let mut s = term_goto_str((0, 0));
    s.push_str(&format!("{}{}", clear::AfterCursor, style::Reset));
    s
}

pub fn term_hide_cursor_str() -> String {
    format!("{}", cursor::Hide)
}

pub fn term_show_cursor_str() -> String {
    format!("{}", cursor::Show)
}

pub fn term_write<W: Write>(s: &mut W, text: &str) {
    write!(s, "{text}").unwrap();
}
