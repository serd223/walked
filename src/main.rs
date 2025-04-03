mod config;

use std::{io::BufWriter, path::PathBuf, str::FromStr};

use config::Config;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Constraint,
    prelude::CrosstermBackend,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Padding, Row, Table, TableState},
    Terminal,
};

#[derive(Debug)]
pub enum PathKind {
    File,
    Dir,
    Ambigious,
}

#[derive(Debug)]
pub enum WalkedError {
    PathNotFound { path: PathBuf, path_kind: PathKind },
    PermissionDenied { path: PathBuf, path_kind: PathKind },
    Message(String),
}

impl std::fmt::Display for WalkedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalkedError::PathNotFound { path, path_kind } => write!(
                f,
                "Couldn't find {} '{}'",
                match path_kind {
                    PathKind::File => "file",
                    PathKind::Dir => "directory",
                    PathKind::Ambigious => "entry",
                },
                path.display()
            ),
            WalkedError::PermissionDenied { path, path_kind } => write!(
                f,
                "Couldn't access {} '{}'",
                match path_kind {
                    PathKind::File => "file",
                    PathKind::Dir => "directory",
                    PathKind::Ambigious => "entry",
                },
                path.display()
            ),
            WalkedError::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for WalkedError {}

const TABLE_HEADER_MIN_WIDTH: u16 = 8;
const HIGHLIGHT_SYMBOL: &str = ">>";
fn main() -> Result<(), std::io::Error> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(BufWriter::new(std::io::stderr())))?;
    let current_dir = std::path::absolute(".").expect("Can't parse current working directory");
    let mut config = Config::default();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if let Ok(config_str) = std::fs::read_to_string(&args[1]) {
            if let Ok(val) = toml::from_str(&config_str) {
                config.from_toml(val);
            }
        }
    }

    let mut ed = Editor {
        config,
        clipboard: Vec::new(),
        mode: EditorMode::Normal,
        left: 2,
        top: 2,
        bottom: 1,
        working_directory: current_dir,
        entries: vec![],
        edit_buffer: String::new(),
        cursor_offset: 0,
        current_entry_length: 0,
        header_width: TABLE_HEADER_MIN_WIDTH,
        selection_start: None,
    };
    let result = run(&mut terminal, &mut ed);
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    println!("{}", ed.working_directory.to_str().unwrap());
    result
}

#[derive(PartialEq, Eq)]
enum EditorMode {
    Normal,
    Insert,
}

impl EditorMode {
    fn to_string(&self, config: &Config) -> String {
        match *self {
            EditorMode::Normal => config.normal_mode_text.clone(),
            EditorMode::Insert => config.insert_mode_text.clone(),
        }
    }
}

struct Editor {
    config: Config,
    clipboard: Vec<PathBuf>,
    mode: EditorMode,
    left: u16,
    top: u16,
    bottom: u16,
    entries: Vec<PathBuf>,
    working_directory: PathBuf,
    edit_buffer: String,
    cursor_offset: u16,
    current_entry_length: usize,
    header_width: u16,
    selection_start: Option<usize>,
}

fn new_path<T: AsRef<std::path::Path>>(p: T) -> PathBuf {
    let mut res = PathBuf::from(p.as_ref());
    let mut res_string = res.to_str().unwrap().to_string();
    while res.exists() {
        res_string += ".1";
        res = PathBuf::from_str(&res_string).unwrap()
    }
    res
}

/// `dest` folder should already exist.
fn copy_recursively(src: &PathBuf, dest: &PathBuf, errors: &mut Vec<WalkedError>) {
    if let Ok(dir) = std::fs::read_dir(src) {
        for d in dir {
            if let Ok(d) = d {
                let p = d.path();
                if p.is_file() {
                    let file = p.file_name().unwrap();
                    let new_file = dest.join(file);
                    if let Err(err) = std::fs::copy(&p, &new_file) {
                        match err.kind() {
                            std::io::ErrorKind::NotFound => {
                                errors.push(WalkedError::PathNotFound {
                                    path: p,
                                    path_kind: PathKind::File,
                                })
                            }
                            std::io::ErrorKind::PermissionDenied => {
                                errors.push(WalkedError::PermissionDenied {
                                    path: new_file,
                                    path_kind: PathKind::File,
                                })
                            }
                            _ => errors.push(WalkedError::Message(format!(
                                "Couldn't copy file from '{}' to '{}'",
                                p.display(),
                                new_file.display()
                            ))),
                        }
                    }
                } else if p.is_dir() {
                    let dir = p.file_name().unwrap();
                    let new_dir = dest.join(dir);
                    if let Err(err) = std::fs::create_dir(&new_dir) {
                        match err.kind() {
                            std::io::ErrorKind::NotFound => {
                                errors.push(WalkedError::PathNotFound {
                                    path: new_dir,
                                    path_kind: PathKind::Dir,
                                })
                            }
                            std::io::ErrorKind::PermissionDenied => {
                                errors.push(WalkedError::PermissionDenied {
                                    path: new_dir,
                                    path_kind: PathKind::Dir,
                                })
                            }
                            _ => errors.push(WalkedError::Message(format!(
                                "Couldn't create directory '{}'",
                                new_dir.display()
                            ))),
                        }
                    } else {
                        copy_recursively(&p, &new_dir, errors);
                    }
                }
            }
        }
    }
}

impl Editor {
    fn refresh_cursor(&mut self, table_state: &TableState) {
        if let Some(i) = table_state.selected() {
            if i < self.entries.len() {
                let name = {
                    if let Some(l) = self.entries[i].file_name() {
                        l.to_str().unwrap().to_string()
                    } else {
                        String::new()
                    }
                };
                self.current_entry_length = name.chars().count();
                self.cursor_offset = self.cursor_offset.min(self.current_entry_length as u16)
            }
        }
    }
    fn walk(&mut self, current_entry: usize) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let selected = &self.entries[current_entry];
        if selected.is_dir() {
            self.working_directory = selected.clone();
            self.read_working_dir();
            return true;
        }
        false
    }
    fn parent(&mut self) -> bool {
        if let Some(p) = self.working_directory.parent() {
            self.working_directory = p.to_path_buf();
            self.read_working_dir();
            return true;
        }
        false
    }
    fn read_working_dir(&mut self) {
        if let Ok(dir) = std::fs::read_dir(&self.working_directory) {
            self.entries.clear();
            for d in dir {
                if let Ok(d) = d {
                    let p = d.path();
                    self.entries.push(p);
                }
            }
            self.header_width = TABLE_HEADER_MIN_WIDTH;
        }
    }
}

fn run<W: ratatui::prelude::Backend>(
    terminal: &mut Terminal<W>,
    ed: &mut Editor,
) -> Result<(), std::io::Error> {
    ed.read_working_dir();
    let mut table_state = TableState::default();
    table_state.select_first();
    ed.refresh_cursor(&table_state);
    let mut errors = Vec::new();

    let mut start = true;
    loop {
        // needed because otherwise the applications hangs until you press a key on startup.
        // i could just change the order of event processing and drawing, but i am pretty sure that
        // i made certain assumptions regarding their order of execution while writing this but tbh i dont remember
        // the spesifics so i feel like this hack is okay
        let event = {
            if start {
                start = false;
                Event::FocusGained
            } else {
                event::read()?
            }
        };
        let mut set_cursor = None;
        if let Event::Key(key_event) = event {
            if errors.len() > 0 {
                if key_event.kind == KeyEventKind::Press {
                    errors.clear();
                }
            } else {
                match ed.mode {
                    EditorMode::Normal => {
                        if key_event == ed.config.dir_walk {
                            if let Some(i) = table_state.selected() {
                                if ed.walk(i) {
                                    table_state.select_first();
                                    ed.refresh_cursor(&table_state);
                                }
                            }
                        } else if key_event == ed.config.dir_up {
                            if ed.parent() {
                                table_state.select_first();
                                ed.refresh_cursor(&table_state);
                            }
                        } else if key_event == ed.config.up {
                            ed.selection_start = None;
                            table_state.scroll_up_by(1);
                            ed.refresh_cursor(&table_state);
                        } else if key_event == ed.config.select_up {
                            if let None = ed.selection_start {
                                ed.selection_start = table_state.selected();
                            }
                            table_state.scroll_up_by(1);
                            ed.refresh_cursor(&table_state);
                        } else if key_event == ed.config.down {
                            ed.selection_start = None;
                            table_state.scroll_down_by(1);
                            ed.refresh_cursor(&table_state);
                        } else if key_event == ed.config.select_down {
                            if let None = ed.selection_start {
                                ed.selection_start = table_state.selected();
                            }
                            table_state.scroll_down_by(1);
                            ed.refresh_cursor(&table_state);
                        } else if key_event == ed.config.left {
                            if ed.cursor_offset > 0 {
                                ed.cursor_offset -= 1;
                            }
                        } else if key_event == ed.config.right {
                            if ed.cursor_offset < ed.current_entry_length as u16 {
                                ed.cursor_offset += 1;
                            }
                        } else if key_event == ed.config.new_file {
                            let new_file = new_path(ed.working_directory.join("NEWFILE"));
                            if let Err(err) = std::fs::File::create(&new_file) {
                                match err.kind() {
                                    std::io::ErrorKind::PermissionDenied => {
                                        errors.push(WalkedError::PermissionDenied {
                                            path: new_file.clone(),
                                            path_kind: PathKind::File,
                                        })
                                    }
                                    _ => errors.push(WalkedError::Message(format!(
                                        "Couldn't create file '{}'",
                                        new_file.display()
                                    ))),
                                }
                            } else {
                                ed.read_working_dir();

                                let mut index = None;
                                for (i, entry) in ed.entries.iter().enumerate() {
                                    if *entry == new_file {
                                        table_state.select(Some(i));
                                        index = Some(i);
                                    }
                                }

                                if let Some(i) = index {
                                    ed.mode = EditorMode::Insert;
                                    ed.edit_buffer.clear();
                                    ed.cursor_offset = 0;
                                    set_cursor = Some((
                                        ed.left + ed.header_width + 1,
                                        ed.top + 1 + i as u16,
                                    ));
                                    table_state.select_column(Some(1));
                                }
                            }
                        } else if key_event == ed.config.new_directory {
                            let new_dir = new_path(ed.working_directory.join("NEWDIR"));
                            if let Err(err) = std::fs::create_dir(&new_dir) {
                                match err.kind() {
                                    std::io::ErrorKind::PermissionDenied => {
                                        errors.push(WalkedError::PermissionDenied {
                                            path: new_dir.clone(),
                                            path_kind: PathKind::Dir,
                                        })
                                    }
                                    _ => errors.push(WalkedError::Message(format!(
                                        "Couldn't create directory '{}'",
                                        new_dir.display()
                                    ))),
                                }
                            } else {
                                ed.read_working_dir();

                                let mut index = None;
                                for (i, entry) in ed.entries.iter().enumerate() {
                                    if *entry == new_dir {
                                        table_state.select(Some(i));
                                        index = Some(i);
                                    }
                                }

                                if let Some(i) = index {
                                    ed.mode = EditorMode::Insert;
                                    ed.edit_buffer.clear();
                                    ed.cursor_offset = 0;
                                    set_cursor = Some((
                                        ed.left + ed.header_width + 1,
                                        ed.top + 1 + i as u16,
                                    ));
                                    table_state.select_column(Some(1));
                                }
                            }
                        } else if key_event == ed.config.duplicate && ed.entries.len() > 0 {
                            if let Some(current_entry) = table_state.selected() {
                                let selection_start =
                                    if let Some(selection_start) = ed.selection_start {
                                        ed.selection_start = None;
                                        selection_start
                                    } else {
                                        current_entry
                                    };
                                let mut refresh = false;

                                for i in current_entry.min(selection_start)
                                    ..=current_entry.max(selection_start)
                                {
                                    let entry_path = &ed.entries[i];
                                    let new_entry_path = new_path(entry_path);

                                    if entry_path.is_file() {
                                        if let Err(err) = std::fs::copy(entry_path, &new_entry_path)
                                        {
                                            match err.kind() {
                                                std::io::ErrorKind::NotFound => {
                                                    errors.push(WalkedError::PathNotFound {
                                                        path: entry_path.clone(),
                                                        path_kind: PathKind::File,
                                                    })
                                                }
                                                std::io::ErrorKind::PermissionDenied => errors
                                                    .push(WalkedError::PermissionDenied {
                                                        path: new_entry_path,
                                                        path_kind: PathKind::File,
                                                    }),
                                                _ => errors.push(WalkedError::Message(format!(
                                                    "Couldn't copy file from '{}' to '{}'",
                                                    entry_path.display(),
                                                    new_entry_path.display()
                                                ))),
                                            }
                                        }

                                        refresh = true;
                                    } else if entry_path.is_dir() {
                                        let new_dir = new_path(entry_path);
                                        if let Err(err) = std::fs::create_dir(&new_dir) {
                                            match err.kind() {
                                                std::io::ErrorKind::PermissionDenied => errors
                                                    .push(WalkedError::PermissionDenied {
                                                        path: new_dir,
                                                        path_kind: PathKind::Dir,
                                                    }),
                                                _ => errors.push(WalkedError::Message(format!(
                                                    "Couldn't create directory '{}'",
                                                    new_dir.display()
                                                ))),
                                            }
                                        } else {
                                            copy_recursively(entry_path, &new_dir, &mut errors);
                                        }
                                        refresh = true;
                                    }
                                }
                                if refresh {
                                    ed.read_working_dir();
                                }
                            }
                        } else if key_event == ed.config.copy && ed.entries.len() > 0 {
                            if let Some(current_entry) = table_state.selected() {
                                ed.clipboard.clear();
                                if let Some(selection_start) = ed.selection_start {
                                    for i in current_entry.min(selection_start)
                                        ..=current_entry.max(selection_start)
                                    {
                                        ed.clipboard.push(ed.entries[i].clone());
                                    }
                                } else {
                                    ed.clipboard.push(ed.entries[current_entry].clone());
                                }
                            }
                        } else if key_event == ed.config.paste {
                            let mut refresh = false;
                            for entry_path in ed.clipboard.iter() {
                                let new_entry_path = new_path(
                                    ed.working_directory.join(entry_path.file_name().unwrap()),
                                );

                                if entry_path.is_file() {
                                    if let Err(err) = std::fs::copy(entry_path, &new_entry_path) {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                errors.push(WalkedError::PathNotFound {
                                                    path: entry_path.clone(),
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            std::io::ErrorKind::PermissionDenied => {
                                                errors.push(WalkedError::PermissionDenied {
                                                    path: new_entry_path,
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            _ => errors.push(WalkedError::Message(format!(
                                                "Couldn't copy file from '{}' to '{}'",
                                                entry_path.display(),
                                                new_entry_path.display()
                                            ))),
                                        }
                                    }
                                    refresh = true;
                                } else if entry_path.is_dir() {
                                    if let Err(err) = std::fs::create_dir(&new_entry_path) {
                                        match err.kind() {
                                            std::io::ErrorKind::PermissionDenied => {
                                                errors.push(WalkedError::PermissionDenied {
                                                    path: new_entry_path,
                                                    path_kind: PathKind::Dir,
                                                })
                                            }
                                            _ => errors.push(WalkedError::Message(format!(
                                                "Couldn't create directory '{}'",
                                                new_entry_path.display()
                                            ))),
                                        }
                                    } else {
                                        copy_recursively(entry_path, &new_entry_path, &mut errors);
                                    }
                                    refresh = true;
                                }
                            }
                            if refresh {
                                ed.read_working_dir();
                            }
                        } else if key_event == ed.config.remove && ed.entries.len() > 0 {
                            if let Some(current_entry) = table_state.selected() {
                                let selection_start =
                                    if let Some(selection_start) = ed.selection_start {
                                        ed.selection_start = None;
                                        selection_start
                                    } else {
                                        current_entry
                                    };
                                let mut refresh = false;

                                for i in current_entry.min(selection_start)
                                    ..=current_entry.max(selection_start)
                                {
                                    let entry = &ed.entries[i];
                                    if entry.is_file() {
                                        if let Err(err) = std::fs::remove_file(entry) {
                                            match err.kind() {
                                                std::io::ErrorKind::NotFound => {
                                                    errors.push(WalkedError::PathNotFound {
                                                        path: entry.clone(),
                                                        path_kind: PathKind::File,
                                                    })
                                                }
                                                std::io::ErrorKind::PermissionDenied => errors
                                                    .push(WalkedError::PermissionDenied {
                                                        path: entry.clone(),
                                                        path_kind: PathKind::File,
                                                    }),
                                                _ => errors.push(WalkedError::Message(format!(
                                                    "Couldn't remove file '{}'",
                                                    entry.display()
                                                ))),
                                            }
                                        }
                                        refresh = true;
                                    } else if entry.is_dir() {
                                        if let Ok(dir) = std::fs::read_dir(entry) {
                                            if let Err(err) = if dir.count() > 0 {
                                                std::fs::remove_dir_all(entry)
                                            } else {
                                                std::fs::remove_dir(entry)
                                            } {
                                                match err.kind() {
                                                    std::io::ErrorKind::NotFound => {
                                                        errors.push(WalkedError::PathNotFound {
                                                            path: entry.clone(),
                                                            path_kind: PathKind::Dir,
                                                        })
                                                    }
                                                    std::io::ErrorKind::PermissionDenied => errors
                                                        .push(WalkedError::PermissionDenied {
                                                            path: entry.clone(),
                                                            path_kind: PathKind::Dir,
                                                        }),
                                                    _ => {
                                                        errors.push(WalkedError::Message(format!(
                                                            "Couldn't remove directory '{}'",
                                                            entry.display()
                                                        )))
                                                    }
                                                }
                                            }

                                            refresh = true;
                                        }
                                    }
                                }

                                if refresh {
                                    ed.read_working_dir();
                                }
                            }
                        } else if key_event == ed.config.insert_mode {
                            if ed.entries.len() > 0 {
                                ed.mode = EditorMode::Insert;
                                if let Some(i) = table_state.selected() {
                                    ed.edit_buffer = {
                                        if let Some(p) = ed.entries[i].file_name() {
                                            p.to_str().unwrap().to_string()
                                        } else {
                                            "".to_string()
                                        }
                                    };
                                    set_cursor = Some((
                                        ed.left + ed.header_width + 1,
                                        ed.top + 1 + i as u16,
                                    ));
                                }
                                table_state.select_column(Some(1));
                            }
                        } else if key_event == ed.config.quit {
                            return Ok(());
                        }
                        ed.refresh_cursor(&table_state);
                    }
                    EditorMode::Insert => {
                        if key_event == ed.config.normal_mode
                            || (key_event.code == KeyCode::Enter
                                && key_event.kind == KeyEventKind::Press)
                        {
                            let mut denied = false;
                            if let Some(i) = table_state.selected() {
                                if ed.edit_buffer.len() > 0 && ed.entries.len() > 0 {
                                    let mut dist = ed.working_directory.clone();
                                    dist.push(&ed.edit_buffer);
                                    let disallowed_chars =
                                        ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
                                    if ed.edit_buffer.contains(&disallowed_chars) {
                                        ed.mode = EditorMode::Insert;
                                        denied = true;
                                        errors.push(WalkedError::Message(format!("Paths can't contain the following characters: {disallowed_chars:?}")));
                                    } else if dist.exists() {
                                        if dist != ed.entries[i] {
                                            ed.mode = EditorMode::Insert;
                                            denied = true;
                                            errors.push(WalkedError::Message(format!(
                                                "'{}' already exists",
                                                dist.display()
                                            )));
                                        }
                                    } else {
                                        if let Err(err) = std::fs::rename(&ed.entries[i], &dist) {
                                            match err.kind() {
                                                std::io::ErrorKind::NotFound => {
                                                    errors.push(WalkedError::PathNotFound {
                                                        path: ed.entries[i].clone(),
                                                        path_kind: PathKind::Ambigious,
                                                    })
                                                }
                                                std::io::ErrorKind::PermissionDenied => errors
                                                    .push(WalkedError::PermissionDenied {
                                                        path: ed.entries[i].clone(),
                                                        path_kind: PathKind::Ambigious,
                                                    }),
                                                _ => errors.push(WalkedError::Message(format!(
                                                    "Couldn't rename '{}' to '{}'",
                                                    ed.entries[i].display(),
                                                    dist.display()
                                                ))),
                                            }
                                        } else {
                                            ed.entries[i] = dist;
                                        }
                                    }
                                }
                            }
                            if !denied {
                                ed.mode = EditorMode::Normal;
                                table_state.select_column(None);
                                ed.edit_buffer.clear();
                            }
                        } else if key_event.kind == KeyEventKind::Press {
                            if key_event.code == KeyCode::Backspace {
                                if ed.cursor_offset > 0 {
                                    let mut idx = ed.edit_buffer.len() - 1;
                                    for (i, (len, _)) in ed.edit_buffer.char_indices().enumerate() {
                                        if i >= ed.cursor_offset as usize {
                                            break;
                                        } else {
                                            idx = len;
                                        }
                                    }
                                    ed.edit_buffer.remove(idx);
                                    ed.cursor_offset -= 1;
                                }
                            } else if let KeyCode::Char(c) = key_event.code {
                                let mut idx = ed.edit_buffer.len();
                                for (i, (len, _)) in ed.edit_buffer.char_indices().enumerate() {
                                    if i == ed.cursor_offset as usize {
                                        idx = len;
                                        break;
                                    }
                                }
                                ed.edit_buffer.insert(idx, c);
                                ed.cursor_offset += 1;
                            }
                        }
                    }
                }
            }
        }

        terminal.draw(|f| {
            if let Some(cursor) = set_cursor {
                f.set_cursor_position(cursor);
            }
            let view = Block::new()
                .padding(Padding::new(ed.left, 0, ed.top, ed.bottom))
                .title(if errors.len() > 0 {
                    {
                        let mut res = String::new();
                        for err in errors.iter() {
                            res.push_str(&format!("{err} "));
                        }
                        res
                    }
                    .into_left_aligned_line()
                    .red()
                } else {
                    ed.working_directory.to_str().unwrap().into_centered_line()
                })
                .title_bottom(ed.mode.to_string(&ed.config).into_centered_line());

            let content = ed
                .entries
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let mut header = String::new();
                    if ed.config.show_entry_number {
                        header.push_str(&format!(
                            "{:w$}",
                            i,
                            w = (ed.entries.len() - 1).to_string().chars().count()
                        ))
                    }
                    if ed.config.show_entry_type {
                        let entry_type = {
                            if ed.entries[i].is_file() {
                                &ed.config.file_text
                            } else if ed.entries[i].is_dir() {
                                &ed.config.directory_text
                            } else if ed.entries[i].is_symlink() {
                                &ed.config.symlink_text
                            } else {
                                &ed.config.other_text
                            }
                        };
                        if ed.config.show_entry_number {
                            header.push(':');
                        }
                        header.push_str(entry_type);
                    }
                    if let Ok(metadata) = std::fs::metadata(&ed.entries[i]) {
                        if ed.entries[i].is_file() {
                            let size = bytesize::ByteSize::b(metadata.len());
                            header.push_str(&format!(" {}", size));
                        } else {
                            header.push_str(" - ");
                        }
                    }
                    ed.header_width = (header.chars().count() as u16).max(ed.header_width);
                    let last = {
                        if let Some(l) = p.file_name() {
                            l.to_os_string()
                        } else {
                            std::ffi::OsString::from("..")
                        }
                    };
                    if ed.mode == EditorMode::Insert {
                        if let Some(selected) = table_state.selected() {
                            if selected == i {
                                return Row::new([header, ed.edit_buffer.clone()]);
                            }
                        }
                    }
                    let is_in_selection = {
                        if let Some(selection_start) = ed.selection_start {
                            if let Some(cur) = table_state.selected() {
                                if cur > selection_start {
                                    i < cur && i >= selection_start
                                } else if cur < selection_start {
                                    i > cur && i <= selection_start
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    };
                    let line = last.to_str().unwrap().to_string();
                    Row::new([
                        header.into_line(),
                        if is_in_selection {
                            line.reversed().into_line()
                        } else {
                            line.into_line()
                        },
                    ])
                })
                .collect::<Vec<Row>>();

            if let Some(i) = table_state.selected() {
                let row_offset = {
                    if i < table_state.offset() {
                        0
                    } else if ed.entries.len() > 0 {
                        (i - table_state.offset()).min(
                            (ed.entries.len() - 1).min(view.inner(f.area()).height as usize - 1),
                        ) as u16
                    } else {
                        0
                    }
                };
                f.set_cursor_position((
                    ed.left
                        + ed.header_width
                        + 1
                        + ed.cursor_offset
                        + if ed.mode == EditorMode::Normal {
                            HIGHLIGHT_SYMBOL.chars().count() as u16
                        } else {
                            0
                        },
                    ed.top + 1 + row_offset,
                ));
            }

            match ed.mode {
                EditorMode::Normal => {
                    f.render_stateful_widget(
                        Table::default()
                            .widths([Constraint::Length(ed.header_width), Constraint::Min(0)])
                            .rows(content)
                            .block(view)
                            .row_highlight_style(Style::new().reversed())
                            .column_highlight_style(Style::new().red())
                            .cell_highlight_style(Style::new().blue())
                            .highlight_symbol(HIGHLIGHT_SYMBOL),
                        f.area(),
                        &mut table_state,
                    );
                }
                EditorMode::Insert => {
                    f.render_stateful_widget(
                        Table::default()
                            .widths([Constraint::Length(ed.header_width), Constraint::Min(0)])
                            .rows(content)
                            .block(view)
                            .cell_highlight_style(Style::new().underlined()),
                        f.area(),
                        &mut table_state,
                    );
                }
            }
        })?;
    }
}

pub trait IntoLine<'a> {
    fn into_line(self) -> Line<'a>;
    fn into_centered_line(self) -> Line<'a>;
    fn into_right_aligned_line(self) -> Line<'a>;
    fn into_left_aligned_line(self) -> Line<'a>;
}
impl<'a, T> IntoLine<'a> for T
where
    T: Into<Line<'a>>,
{
    fn into_line(self) -> Line<'a> {
        self.into()
    }

    fn into_centered_line(self) -> Line<'a> {
        self.into_line().centered()
    }

    fn into_right_aligned_line(self) -> Line<'a> {
        self.into_line().right_aligned()
    }

    fn into_left_aligned_line(self) -> Line<'a> {
        self.into_line().left_aligned()
    }
}
