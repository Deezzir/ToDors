use std::io::Write;
use termion::{clear, color, cursor, style};

pub fn term_set_color<W: Write>(s: &mut W, fg: &dyn color::Color, bg: Option<&dyn color::Color>) {
    write!(s, "{}", color::Fg(fg)).unwrap();
    if let Some(bg) = bg {
        write!(s, "{}", color::Bg(bg)).unwrap();
    }
}

pub fn term_goto<W: Write>(s: &mut W, pos: (u16, u16)) {
    write!(s, "{}", cursor::Goto(pos.1 + 1, pos.0 + 1)).unwrap();
}

pub fn term_write<W: Write>(s: &mut W, text: &str) {
    write!(s, "{text}").unwrap();
}

pub fn term_style_reset<W: Write>(s: &mut W) {
    write!(s, "{}", style::Reset).unwrap();
}

pub fn term_reset<W: Write>(s: &mut W) {
    term_goto(s, (0, 0));
    write!(s, "{}{}", clear::AfterCursor, style::Reset).unwrap();
}

pub fn term_hide_cursor<W: Write>(s: &mut W) {
    write!(s, "{}", cursor::Hide).unwrap();
}

pub fn term_show_cursor<W: Write>(s: &mut W) {
    write!(s, "{}", cursor::Show).unwrap();
}
