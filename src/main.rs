use std::{
    ffi::OsString,
    fs::DirEntry,
    io::Write,
    path::{self, PathBuf},
    str::FromStr,
};

use crossterm::{
    cursor::{self, MoveToColumn},
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyEventState, KeyModifiers,
    },
    execute, queue,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Keybinds {
    duplicate: KeyEvent,
    remove: KeyEvent,
    copy: KeyEvent,
    paste: KeyEvent,
    up: KeyEvent,
    down: KeyEvent,
    left: KeyEvent,
    right: KeyEvent,
    dir_walk: KeyEvent,
    dir_up: KeyEvent,
    insert_mode: KeyEvent,
    normal_mode: KeyEvent,
    quit: KeyEvent,
}

enum EditorMode {
    Normal,
    Insert,
}

struct Editor {
    clipboard: PathBuf,
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
        let _ = queue!(w, cursor::MoveTo(0, 1));
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
    fn refresh(&mut self, w: &mut impl std::io::Write) {
        if let Ok(cr) = cursor::position() {
            let cl = self.current_line;
            let scr = self.scroll;
            self.read_working_dir();
            self.current_line = cl;
            self.scroll = scr;
            self.show(w, cr.1);
            let _ = queue!(w, MoveToColumn(cr.0));
        }
    }
}

fn main() -> std::io::Result<()> {
    let current_dir =
        PathBuf::from(path::absolute(".").expect("Can't parse current working directory"));
    let mut config_file = String::new();
    let args: Vec<String> = std::env::args().collect();
    let mut custom_conf_detected = false;
    if args.len() > 1 {
        custom_conf_detected = true;
        config_file = args[1].clone();
    }
    let mut keybinds = Keybinds {
        duplicate: KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        remove: KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        copy: KeyEvent {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        paste: KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        up: KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        down: KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        left: KeyEvent {
            code: KeyCode::Char('h'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        right: KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        insert_mode: KeyEvent {
            code: KeyCode::Char('i'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        normal_mode: KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        quit: KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        dir_walk: KeyEvent {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
        dir_up: KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        },
    };
    if custom_conf_detected {
        if let Ok(config_str) = std::fs::read_to_string(&config_file) {
            keybinds = toml::from_str(&config_str).expect("Couldn't parse the config file.");
        } else {
            let config =
                toml::to_string_pretty(&keybinds).expect("Couldn't parse keybinds to config file.");
            std::fs::write(&config_file, config)?;
        }
    }
    let mut ed = Editor {
        clipboard: PathBuf::new(),
        mode: EditorMode::Normal,
        left: 2,
        top: 4,
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
                    if event == Event::Key(keybinds.dir_walk) {
                        ed.walk(&mut stdout);
                        queue!(
                            stdout,
                            cursor::MoveToColumn(ed.left + ed.render_current_entry().len() as u16)
                        )?;
                    }
                    if event == Event::Key(keybinds.left) {
                        if let Ok(cr) = cursor::position() {
                            queue!(stdout, cursor::MoveToColumn(ed.left.max(cr.0 - 1)))?;
                        }
                    }
                    if event == Event::Key(keybinds.down) {
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
                    if event == Event::Key(keybinds.up) {
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
                    if event == Event::Key(keybinds.right) {
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
                    if event == Event::Key(keybinds.dir_up) {
                        ed.parent(&mut stdout);
                        queue!(
                            stdout,
                            cursor::MoveToColumn(ed.left + ed.render_current_entry().len() as u16)
                        )?;
                    }
                    if event == Event::Key(keybinds.insert_mode) {
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
                    if event == Event::Key(keybinds.duplicate) {
                        let entry_path = &ed.entries[ed.current_entry()];
                        let entry = entry_path.to_str().unwrap();
                        let mut new_entry = entry.to_string();
                        let mut new_entry_path = entry_path.clone();
                        while new_entry_path.exists() {
                            new_entry += ".1";
                            // TODO: If the path is somehow corrupted and from_str fails repeatedly, this would result in an infinite loop.
                            if let Ok(nep) = PathBuf::from_str(&new_entry) {
                                new_entry_path = nep;
                            }
                        }
                        // TODO: Handle Error
                        let _ = std::fs::copy(entry_path, new_entry_path);
                        ed.refresh(&mut stdout);
                    }
                    if event == Event::Key(keybinds.copy) {
                        ed.clipboard = ed.entries[ed.current_entry()].clone();
                    }
                    if event == Event::Key(keybinds.paste) {
                        let entry_path = &ed.clipboard;

                        if entry_path.is_file() {
                            let mut new_entry_path =
                                ed.working_directory.join(entry_path.file_name().unwrap());
                            let mut new_entry = new_entry_path.to_str().unwrap().to_string();
                            while new_entry_path.exists() {
                                new_entry += ".1";
                                // TODO: If the path is somehow corrupted and from_str fails repeatedly, this would result in an infinite loop.
                                if let Ok(nep) = PathBuf::from_str(&new_entry) {
                                    new_entry_path = nep;
                                }
                            }
                            // TODO: Handle Error
                            let _ = std::fs::copy(entry_path, new_entry_path);
                            ed.refresh(&mut stdout);
                        }
                    }
                    if event == Event::Key(keybinds.remove) {
                        let entry = &ed.entries[ed.current_entry()];
                        if entry.is_file() {
                            // TODO: Handle Error
                            let _ = std::fs::remove_file(entry);
                        } else if entry.is_dir() {
                            // TODO: Handle Error
                            let _ = std::fs::remove_dir_all(entry);
                        }
                        ed.refresh(&mut stdout);
                    }
                    if event == Event::Key(keybinds.quit) {
                        stdout.flush()?;
                        break;
                    }
                }
                EditorMode::Insert => {
                    if event == Event::Key(keybinds.normal_mode) {
                        ed.mode = EditorMode::Normal;
                        if modified_entry {
                            let i = ed.current_entry();
                            if ed.buffer[i].len() > 0 {
                                let mut dist = ed.working_directory.clone();
                                dist.push(&ed.buffer[i]);
                                if dist.exists() {
                                    if !(dist == ed.entries[i]) {
                                        ed.mode = EditorMode::Insert;
                                    }
                                } else {
                                    std::fs::rename(&ed.entries[i], &dist)?;
                                    ed.entries[i] = dist;
                                }
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
                    if event == Event::Key(KeyCode::Backspace.into()) {
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
                    match event {
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
                    }
                }
            }
            stdout.flush()?;
        }
        Ok(()) as std::io::Result<()>
    } {
        println!("Error: {e}");
    }

    execute!(stdout, DisableMouseCapture, LeaveAlternateScreen)?;
    let res = disable_raw_mode();
    println!("{}", ed.working_directory.to_str().unwrap());

    res
}
