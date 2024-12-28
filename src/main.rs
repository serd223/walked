use std::{
    ffi::OsString,
    fs::DirEntry,
    io::Write,
    path::{self, PathBuf},
};

use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};

enum EditorMode {
    Normal,
    Insert,
}

struct Editor {
    mode: EditorMode,
    buffer: Vec<String>,
    left: u16,
    top: u16,
    bottom: u16,
    buf_size_col: u16,
    buf_size_row: u16,
    scroll: u16,
    entries: Vec<PathBuf>,
    working_directory: PathBuf,
    current_line: u16,
}

impl Editor {
    fn update_size(&mut self) {
        if let Ok(size) = terminal::size() {
            (self.buf_size_col, self.buf_size_row) =
                (size.0 - self.left, size.1 - self.top - self.bottom);
        }
    }

    fn move_up(&mut self, w: &mut impl std::io::Write) {
        if self.current_line > 0 {
            self.current_line -= 1;
            let _ = queue!(w, cursor::MoveUp(1));
        } else {
            if self.scroll > 0 {
                self.scroll -= 1;
                self.show(w, self.top);
            }
        }
    }
    fn move_down(&mut self, w: &mut impl std::io::Write) {
        if self.entries.len() <= 1 {
            return;
        }
        let mut should_scroll = false;
        let visible_entry_count = {
            let l = self.entries.len() as u16 - self.scroll;
            let r = self.buf_size_row;
            if l > r {
                should_scroll = true;
                r
            } else {
                l
            }
        };
        if self.current_line < visible_entry_count - 1 {
            self.current_line += 1;
            let _ = queue!(w, cursor::MoveDown(1));
        } else {
            if should_scroll {
                self.scroll += 1;
            }
            self.show(w, visible_entry_count + self.top - 1);
        }
    }
    fn add_entry(&mut self, d: DirEntry) {
        let p = d.path();
        let last = {
            if let Some(l) = p.file_name() {
                l.to_os_string()
            } else {
                OsString::from("..")
            }
        };
        self.entries.push(p);
        self.println(last.to_str().unwrap());
    }

    fn println(&mut self, s: impl AsRef<str>) {
        self.buffer.push(s.as_ref().to_string());
        self.current_line += 1;
    }

    fn write_buffer_at(&self, w: &mut impl std::io::Write, cursor_row: u16) {
        let _ = queue!(w, cursor::MoveToRow(cursor_row));
        for (i, _l) in self.buffer.iter().enumerate() {
            let i = i as u16;
            if i >= self.scroll && i < self.scroll + self.buf_size_row - self.bottom {
                // let entry_i = i as usize;
                let i = i - self.scroll;
                let _ = queue!(w, cursor::MoveTo(self.left, cursor_row + i));
                let _ = write!(w, "{}", self.render_entry_at((i + self.scroll) as usize));
            }
        }
    }

    fn read_working_dir(&mut self) {
        self.buffer.clear();
        if let Ok(dir) = std::fs::read_dir(&self.working_directory) {
            self.entries.clear();
            for d in dir {
                if let Ok(d) = d {
                    self.add_entry(d);
                }
            }
        }
    }

    fn show_badge(&mut self, w: &mut impl std::io::Write) {
        // Implies that Editor.top must be greater that or equal to 2.
        let _ = queue!(w, cursor::MoveTo(0, self.top - 2));
        match self.mode {
            EditorMode::Normal => {
                let _ = write!(w, "NORMAL");
            }
            EditorMode::Insert => {
                let _ = write!(w, "INSERT");
            }
        };
    }

    fn show(&mut self, w: &mut impl std::io::Write, cursor_row: u16) {
        let _ = queue!(
            w,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(self.left, self.top),
        );
        self.show_badge(w);
        let _ = queue!(w, cursor::MoveTo(self.left, self.top));
        self.write_buffer_at(w, self.top);
        let _ = queue!(w, cursor::MoveToRow(cursor_row));
    }

    fn render_current_entry(&self) -> String {
        self.render_entry_at(self.current_entry())
    }

    fn render_entry_at(&self, i: usize) -> String {
        format!("{}: {}", i, &self.buffer[i])
    }
    fn current_entry(&self) -> usize {
        (self.current_line + self.scroll) as usize
    }
    fn walk(&mut self, w: &mut impl std::io::Write) {
        let selected = &self.entries[self.current_entry()];
        if selected.is_dir() {
            self.working_directory = selected.clone();
            self.read_working_dir();
            self.current_line = 0;
            self.scroll = 0;
            self.show(w, self.top);
        }
    }

    fn parent(&mut self, w: &mut impl std::io::Write) {
        if let Some(p) = self.working_directory.parent() {
            self.working_directory = p.to_path_buf();
            self.read_working_dir();
            self.current_line = 0;
            self.scroll = 0;
            self.show(w, self.top + self.current_line as u16 - self.scroll);
        }
    }
}

fn main() -> std::io::Result<()> {
    let current_dir =
        PathBuf::from(path::absolute(".").expect("Can't parse current working directory"));
    let mut ed = Editor {
        mode: EditorMode::Normal,
        left: 2,
        top: 3,
        bottom: 0,
        buf_size_col: 0,
        buf_size_row: 0,
        scroll: 0,
        buffer: vec![],
        working_directory: current_dir,
        entries: vec![],
        current_line: 0,
    };
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnableMouseCapture,
        EnterAlternateScreen,
        cursor::MoveTo(0, 0)
    )?;
    enable_raw_mode()?;
    ed.update_size();
    if let Ok(dir) = std::fs::read_dir(&ed.working_directory) {
        for d in dir {
            if let Ok(d) = d {
                ed.add_entry(d);
            }
        }
    }
    execute!(stdout, cursor::MoveTo(ed.left, ed.top))?;
    ed.current_line = 0;
    ed.scroll = 0;
    ed.show(&mut stdout, ed.top);
    ed.current_line = 0;
    ed.scroll = 0;
    queue!(
        stdout,
        cursor::MoveToColumn(ed.left + ed.render_current_entry().len() as u16)
    )?;
    let mut modified_entry = false;
    if let Err(e) = {
        loop {
            ed.update_size();
            let event = crossterm::event::read()?;
            match ed.mode {
                EditorMode::Normal => {
                    if event == Event::Key(KeyCode::Char(' ').into()) {
                        ed.walk(&mut stdout);
                        queue!(
                            stdout,
                            cursor::MoveToColumn(ed.left + ed.render_current_entry().len() as u16)
                        )?;
                    }
                    if event == Event::Key(KeyCode::Char('h').into()) {
                        if let Ok(cr) = cursor::position() {
                            queue!(stdout, cursor::MoveToColumn(ed.left.max(cr.0 - 1)))?;
                        }
                    }
                    if event == Event::Key(KeyCode::Char('j').into()) {
                        ed.move_down(&mut stdout);
                        if let Ok(cr) = cursor::position() {
                            queue!(
                                stdout,
                                cursor::MoveToColumn(
                                    (ed.left + ed.render_current_entry().len() as u16).min(cr.0)
                                )
                            )?;
                        }
                    }
                    if event == Event::Key(KeyCode::Char('k').into()) {
                        ed.move_up(&mut stdout);
                        if let Ok(cr) = cursor::position() {
                            queue!(
                                stdout,
                                cursor::MoveToColumn(
                                    (ed.left + ed.render_current_entry().len() as u16).min(cr.0)
                                )
                            )?;
                        }
                    }
                    if event == Event::Key(KeyCode::Char('l').into()) {
                        if let Ok(cr) = cursor::position() {
                            queue!(
                                stdout,
                                cursor::MoveToColumn(
                                    (ed.left + ed.render_current_entry().len() as u16)
                                        .min(cr.0 + 1)
                                )
                            )?;
                        }
                    }
                    if event == Event::Key(KeyCode::Char('x').into()) {
                        ed.parent(&mut stdout);
                        queue!(
                            stdout,
                            cursor::MoveToColumn(ed.left + ed.render_current_entry().len() as u16)
                        )?;
                    }
                    if event == Event::Key(KeyCode::Char('i').into()) {
                        ed.mode = EditorMode::Insert;
                        modified_entry = false;
                        if let Ok(cr) = cursor::position() {
                            ed.show(&mut stdout, cr.1);
                            queue!(
                                stdout,
                                cursor::MoveToColumn(cr.0.clamp(
                                    ed.left + ed.render_current_entry().len() as u16
                                        - ed.buffer[ed.current_entry()].len() as u16,
                                    ed.left + ed.render_current_entry().len() as u16
                                ))
                            )?;
                        }
                    }
                    if event == Event::Key(KeyCode::Char('q').into()) {
                        stdout.flush()?;
                        break;
                    }
                }
                EditorMode::Insert => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        ed.mode = EditorMode::Normal;
                        if modified_entry {
                            let i = ed.current_entry();
                            if ed.buffer[i].len() > 0 {
                                let mut dist = ed.working_directory.to_owned();
                                dist.push(&ed.buffer[i]);
                                std::fs::rename(&ed.entries[i], &dist)?;
                                ed.entries[i] = dist;
                            } else {
                                ed.mode = EditorMode::Insert;
                            }
                        }
                        ed.show(&mut stdout, ed.top + ed.current_line);
                        if let Ok(cr) = cursor::position() {
                            queue!(
                                stdout,
                                cursor::MoveToColumn(
                                    (ed.left + ed.render_current_entry().len() as u16).min(cr.0)
                                )
                            )?;
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Backspace,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        let i = ed.current_entry();
                        let name_start =
                            ed.left as usize + ed.render_current_entry().len() - ed.buffer[i].len();
                        if let Ok(cr) = cursor::position() {
                            if cr.0 as usize > name_start {
                                let _ = ed.buffer[i].remove(cr.0 as usize - name_start - 1);
                                modified_entry = true;
                                ed.show(&mut stdout, cr.1);
                                queue!(
                                    stdout,
                                    cursor::MoveToColumn(
                                        (ed.left + ed.render_current_entry().len() as u16)
                                            .min(cr.0 - 1)
                                    )
                                )?;
                            }
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(c),
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        if !['\\', '/', ':', '*', '?', '\"', '<', '>', '|'].contains(&c) {
                            let i = ed.current_entry();
                            let name_start = ed.left as usize + ed.render_current_entry().len()
                                - ed.buffer[i].len();
                            if let Ok(cr) = cursor::position() {
                                ed.buffer[i].insert(cr.0 as usize - name_start, c);
                                modified_entry = true;
                                ed.show(&mut stdout, cr.1);
                                queue!(
                                    stdout,
                                    cursor::MoveToColumn(
                                        (ed.left + ed.render_current_entry().len() as u16)
                                            .min(cr.0 + 1)
                                    )
                                )?;
                            }
                        }
                    }
                    _ => (),
                },
            }
            stdout.flush()?;
        }
        Ok(()) as std::io::Result<()>
    } {
        println!("Error: {e}");
    }

    execute!(stdout, DisableMouseCapture, LeaveAlternateScreen)?;
    disable_raw_mode()
}
