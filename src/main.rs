use std::{
    ffi::OsString,
    fs::DirEntry,
    io::Write,
    path::{self, PathBuf},
};

use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, queue,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};

struct Editor {
    entries: Vec<PathBuf>,
    working_directory: PathBuf,
    current_entry: usize,
    last_selected: Vec<usize>,
}

impl Editor {
    fn move_up(&mut self, w: &mut impl std::io::Write) {
        if self.current_entry > 0 {
            self.current_entry -= 1;
            let _ = queue!(w, cursor::MoveUp(1));
        }
    }
    fn move_down(&mut self, w: &mut impl std::io::Write) {
        if self.current_entry < self.entries.len() - 1 {
            self.current_entry += 1;
            let _ = queue!(w, cursor::MoveDown(1));
        }
    }
    fn add_entry(&mut self, w: &mut impl std::io::Write, d: DirEntry) {
        let p = d.path();
        let last = {
            if let Some(l) = p.file_name() {
                l.to_os_string()
            } else {
                OsString::from("..")
            }
        };
        self.entries.push(p);
        self.println(w, last.to_str().unwrap());
    }

    fn println(&mut self, w: &mut impl std::io::Write, s: impl AsRef<str>) {
        let _ = writeln!(w, "{}", s.as_ref());
        self.current_entry += 1;
    }

    fn show(&mut self, w: &mut impl std::io::Write) {
        let _ = queue!(
            w,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        );
        if let Ok(dir) = std::fs::read_dir(&self.working_directory) {
            self.entries.clear();
            for d in dir {
                if let Ok(d) = d {
                    self.add_entry(w, d);
                }
            }
        }
        let _ = queue!(w, cursor::MoveTo(0, 0));
        self.current_entry = 0;
    }

    fn walk(&mut self, w: &mut impl std::io::Write) {
        let selected = &self.entries[self.current_entry];
        if selected.is_dir() {
            self.last_selected.push(self.current_entry);
            self.working_directory = selected.clone();
            self.show(w);
        }
    }

    fn parent(&mut self, w: &mut impl std::io::Write) {
        if let Some(p) = self.working_directory.parent() {
            self.working_directory = p.to_path_buf();
            self.show(w);
            if let Some(e) = self.last_selected.pop() {
                self.current_entry = e;
                let _ = queue!(w, cursor::MoveTo(0, self.current_entry as u16));
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let current_dir =
        PathBuf::from(path::absolute(".").expect("Can't parse current working directory"));
    let mut ed = Editor {
        working_directory: current_dir,
        entries: vec![],
        current_entry: 0,
        last_selected: vec![],
    };
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnableMouseCapture,
        EnterAlternateScreen,
        cursor::MoveTo(0, 0)
    )?;
    enable_raw_mode()?;
    if let Ok(dir) = std::fs::read_dir(&ed.working_directory) {
        for d in dir {
            if let Ok(d) = d {
                ed.add_entry(&mut stdout, d);
            }
        }
    }
    execute!(stdout, cursor::MoveTo(0, 0))?;
    ed.current_entry = 0;

    if let Err(e) = {
        loop {
            let event = crossterm::event::read()?;
            if event == Event::Key(KeyCode::Char(' ').into()) {
                ed.walk(&mut stdout);
            }
            if event == Event::Key(KeyCode::Char('h').into()) {
                queue!(stdout, cursor::MoveLeft(1))?;
            }
            if event == Event::Key(KeyCode::Char('j').into()) {
                ed.move_down(&mut stdout);
            }
            if event == Event::Key(KeyCode::Char('k').into()) {
                ed.move_up(&mut stdout);
            }
            if event == Event::Key(KeyCode::Char('l').into()) {
                queue!(stdout, cursor::MoveRight(1))?;
            }
            if event == Event::Key(KeyCode::Char('x').into()) {
                ed.parent(&mut stdout);
            }
            stdout.flush()?;

            if event == Event::Key(KeyCode::Esc.into()) {
                break;
            }
        }
        Ok(()) as std::io::Result<()>
    } {
        println!("Error: {e}");
    }

    execute!(stdout, DisableMouseCapture, LeaveAlternateScreen)?;
    disable_raw_mode()
}
