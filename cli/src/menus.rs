use crate::colorize::ToColored;
use std::fmt::Display;
use std::io::{self, BufWriter, StdoutLock, Write};
use termion::cursor::DetectCursorPos;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::terminal_size;
use termion::{clear, cursor, event::Key};

#[macro_export]
macro_rules! text {
    ($dst:expr, $($arg:tt)*) => {{
            write!(
                $dst.stdout,
                "{}{}{}{}\r",
                cursor::Up(1),
                clear::CurrentLine,
                format_args!($($arg)*),
                cursor::Down(1)
            )?;
            $dst.stdout.flush()?;
    }};
}

#[macro_export]
macro_rules! textln {
    ($dst:expr, $($arg:tt)*) => {{
        text!($dst, "{}\n", format_args!($($arg)*));
    }};
}

pub enum SelectNumberedResp {
    Index(usize),
    UndefinedKey(Key),
    Quit,
}
pub struct Menus {
    pub(crate) stdout: BufWriter<RawTerminal<StdoutLock<'static>>>,
}
impl Menus {
    pub fn new() -> Self {
        let (r, c) = terminal_size().unwrap();
        if r < 46 || c < 29 {
            eprintln!("Terminal screen too small");
            std::process::exit(1);
        }
        Self {
            stdout: BufWriter::new(io::stdout().lock().into_raw_mode().unwrap()),
        }
    }

    pub fn cursor_hide(&mut self) -> io::Result<()> {
        write!(self.stdout, "{}", cursor::Hide)?;
        Ok(())
    }

    pub fn cursor_show(&mut self) -> io::Result<()> {
        write!(self.stdout, "{}", cursor::Show)?;
        Ok(())
    }

    pub fn select_menu<L: Display, I: Iterator<Item = L> + Clone>(
        &mut self,
        list: I,
        title: impl Display,
        prompt: impl Display,
        quit: Option<Key>,
    ) -> io::Result<Option<usize>> {
        let mut select_idx = 0;
        let list_len = list.clone().count();
        let mut keys = io::stdin().lock().keys();
        let pos = self.stdout.cursor_pos().unwrap();

        write!(self.stdout, "{}\r\n", title)?;
        let ret = loop {
            for (i, selection) in list.clone().enumerate() {
                if i == select_idx {
                    write!(
                        self.stdout,
                        "{} {}\r\n",
                        prompt,
                        selection.black().white_bg()
                    )?;
                } else {
                    write!(self.stdout, "{}\r\n", selection.faint())?;
                }
            }
            self.stdout.flush()?;

            let key = keys
                .next()
                .expect("keys() should block")
                .expect("faulty keyboard?");
            write!(
                self.stdout,
                "\r{}{}",
                cursor::Goto(pos.0, pos.1),
                clear::AfterCursor
            )?;
            match key {
                Key::Char('\n') => {
                    break Ok(Some(select_idx));
                }
                Key::Up => select_idx = select_idx.saturating_sub(1),
                Key::Down => {
                    if select_idx + 1 < list_len {
                        select_idx += 1;
                    }
                }
                k if k == Key::Ctrl('c') || quit.is_some_and(|q| q == key) => {
                    break Ok(None);
                }
                _ => {}
            }
        };
        write!(self.stdout, "{}{}", cursor::Up(1), clear::CurrentLine)?;
        self.stdout.flush()?;
        ret
    }

    pub fn select_menu_with_input<F: Fn(&str) -> Vec<L>, L: Display>(
        &mut self,
        lister: F,
        prompt: impl Display,
        input_prompt: &str,
        quit: Option<Key>,
    ) -> io::Result<Option<L>> {
        let mut select_idx = 0;
        let mut cursor = 0;
        let mut input = String::new();
        let pos = self.stdout.cursor_pos().unwrap();

        let mut keys = io::stdin().lock().keys();
        let ret = loop {
            write!(
                self.stdout,
                "\r{}{}{}",
                clear::AfterCursor,
                input_prompt.magenta(),
                input,
            )?;
            let mut list = lister(&input);
            let list_len = list.len();

            select_idx = select_idx.min(list_len);
            if list_len > 0 {
                write!(self.stdout, "\r\n\n↑ and ↓ to navigate")?;
                write!(self.stdout, "\n\rENTER to select\r\n")?;
            }

            for (i, selection) in list.iter().enumerate() {
                if i == select_idx {
                    write!(
                        self.stdout,
                        "{} {}\r\n",
                        prompt,
                        selection.black().white_bg()
                    )?;
                } else {
                    write!(self.stdout, "{}\r\n", selection.faint())?;
                }
            }
            if list_len > 0 {
                write!(self.stdout, "{}", cursor::Goto(pos.0, pos.1))?;
            }
            write!(
                self.stdout,
                "\r{}",
                cursor::Right(input_prompt.len() as u16 + cursor as u16)
            )?;
            self.stdout.flush()?;
            write!(self.stdout, "\r{}", clear::AfterCursor)?;

            match keys
                .next()
                .expect("keys() should block")
                .expect("faulty keyboard?")
            {
                Key::Char('\n') => {
                    break Ok(if list_len > select_idx {
                        Some(list.remove(select_idx))
                    } else {
                        None
                    });
                }
                Key::Up => select_idx = select_idx.saturating_sub(1),
                Key::Down => {
                    if select_idx + 1 < list_len {
                        select_idx += 1;
                    }
                }
                Key::Backspace => {
                    if cursor > 0 {
                        cursor -= 1;
                        input.remove(cursor);
                    }
                }
                Key::Char(c) => {
                    if c.is_ascii() {
                        input.insert(cursor, c);
                        cursor += 1;
                    } else {
                        write!(
                            self.stdout,
                            "{}{}{}{}\r",
                            cursor::Up(1),
                            clear::CurrentLine,
                            format_args!("Only ASCII characters"),
                            cursor::Down(1)
                        )?;
                    }
                }
                Key::Right => {
                    if cursor < input.len() {
                        cursor += 1
                    }
                }
                Key::Left => cursor = cursor.saturating_sub(1),
                k if k == Key::Ctrl('c') || quit.is_some_and(|q| q == k) => {
                    break Ok(None);
                }
                _ => {}
            }
        };
        write!(self.stdout, "\r{}{}\r\n", cursor::Up(1), clear::AfterCursor)?;
        self.stdout.flush()?;
        ret
    }

    pub fn select_menu_numbered<L: Display, I: Iterator<Item = L> + Clone>(
        &mut self,
        list: I,
        quit: Key,
        title: &str,
    ) -> io::Result<SelectNumberedResp> {
        let list_len = list.clone().count();
        let pos = self.stdout.cursor_pos().unwrap();

        write!(self.stdout, "\r{title}\r\n")?;
        for (i, s) in list.enumerate() {
            write!(self.stdout, "{}. {}\r\n", (i + 1).green(), s)?;
        }
        write!(self.stdout, "{}. Quit\r\n", 'q'.green())?;
        self.stdout.flush()?;
        let key = io::stdin()
            .lock()
            .keys()
            .next()
            .expect("keys() should block")
            .expect("faulty keyboard?");
        write!(
            self.stdout,
            "\r{}{}",
            cursor::Goto(pos.0, pos.1),
            clear::AfterCursor,
        )?;
        self.stdout.flush()?;
        match key {
            Key::Char(c) if c.to_digit(10).is_some_and(|c| c as usize <= list_len) => Ok(
                SelectNumberedResp::Index(c.to_digit(10).unwrap() as usize - 1),
            ),
            k if k == Key::Ctrl('c') || k == quit => Ok(SelectNumberedResp::Quit),
            k => Ok(SelectNumberedResp::UndefinedKey(k)),
        }
    }
}
