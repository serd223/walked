use std::{path::PathBuf, str::FromStr};

pub const TABLE_HEADER_MIN_WIDTH: u16 = 8;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::TableState;

use crate::{config::Config, PathKind, WalkedError};

pub struct Window {
    pub panels: Vec<Vec<Panel>>,
    pub panel_focus_i: usize,
    pub panel_focus_j: usize,
    pub clipboard: Vec<PathBuf>,
    pub config: Config,
}

impl Window {
    pub fn panel(&mut self) -> &Panel {
        &self.panels[self.panel_focus_i][self.panel_focus_j]
    }
}

#[derive(PartialEq, Eq)]
pub enum PanelMode {
    Normal,
    Insert,
}

pub struct Panel {
    pub errors: Vec<WalkedError>,
    pub table_state: TableState,
    pub mode: PanelMode,
    pub left: u16,
    pub top: u16,
    pub bottom: u16,
    pub entries: Vec<PathBuf>,
    pub working_directory: PathBuf,
    pub edit_buffer: String,
    pub cursor_offset: u16,
    pub current_entry_length: usize,
    pub header_width: u16,
    pub selection_start: Option<usize>,
}

pub struct PanelFrameData {
    pub should_refresh: bool,
    pub quit: bool,
}

impl Panel {
    pub fn new(current_dir: PathBuf) -> Self {
        let mut panel = Self {
            errors: Vec::new(),
            table_state: TableState::default(),
            mode: PanelMode::Normal,
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
        panel.read_working_dir();
        panel.table_state.select_first();
        panel.refresh_cursor();
        panel
    }

    /// Returns false if quit was pressed
    pub fn process_key_event(
        &mut self,
        key_event: KeyEvent,
        clipboard: &mut Vec<PathBuf>,
        config: &Config,
    ) -> PanelFrameData {
        let mut result = PanelFrameData {
            quit: false,
            should_refresh: false,
        };

        if self.errors.len() > 0 {
            if key_event.kind == KeyEventKind::Press {
                self.errors.clear();
            }
        } else {
            match self.mode {
                PanelMode::Normal => {
                    if key_event == config.dir_walk {
                        if let Some(i) = self.table_state.selected() {
                            if self.walk(i) {
                                self.table_state.select_first();
                                self.refresh_cursor();
                            }
                        }
                    } else if key_event == config.dir_up {
                        if self.parent() {
                            self.table_state.select_first();
                            self.refresh_cursor();
                        }
                    } else if key_event == config.up {
                        self.selection_start = None;
                        self.table_state.scroll_up_by(1);
                        self.refresh_cursor();
                    } else if key_event == config.select_up {
                        if let None = self.selection_start {
                            self.selection_start = self.table_state.selected();
                        }
                        self.table_state.scroll_up_by(1);
                        self.refresh_cursor();
                    } else if key_event == config.down {
                        self.selection_start = None;
                        self.table_state.scroll_down_by(1);
                        self.refresh_cursor();
                    } else if key_event == config.select_down {
                        if let None = self.selection_start {
                            self.selection_start = self.table_state.selected();
                        }
                        self.table_state.scroll_down_by(1);
                        self.refresh_cursor();
                    } else if key_event == config.left {
                        if self.cursor_offset > 0 {
                            self.cursor_offset -= 1;
                        }
                    } else if key_event == config.right {
                        if self.cursor_offset < self.current_entry_length as u16 {
                            self.cursor_offset += 1;
                        }
                    } else if key_event == config.new_file {
                        let new_file = new_path(self.working_directory.join("NEWFILE"));
                        if let Err(err) = std::fs::File::create(&new_file) {
                            match err.kind() {
                                std::io::ErrorKind::PermissionDenied => {
                                    self.errors.push(WalkedError::PermissionDenied {
                                        path: new_file.clone(),
                                        path_kind: PathKind::File,
                                    })
                                }
                                _ => self.errors.push(WalkedError::Message(format!(
                                    "Couldn't create file '{}'",
                                    new_file.display()
                                ))),
                            }
                        } else {
                            self.read_working_dir();
                            result.should_refresh = true;

                            for (i, entry) in self.entries.iter().enumerate() {
                                if *entry == new_file {
                                    self.table_state.select(Some(i));
                                    self.mode = PanelMode::Insert;
                                    self.edit_buffer.clear();
                                    self.cursor_offset = 0;
                                    self.table_state.select_column(Some(1));
                                }
                            }
                        }
                    } else if key_event == config.new_directory {
                        let new_dir = new_path(self.working_directory.join("NEWDIR"));
                        if let Err(err) = std::fs::create_dir(&new_dir) {
                            match err.kind() {
                                std::io::ErrorKind::PermissionDenied => {
                                    self.errors.push(WalkedError::PermissionDenied {
                                        path: new_dir.clone(),
                                        path_kind: PathKind::Dir,
                                    })
                                }
                                _ => self.errors.push(WalkedError::Message(format!(
                                    "Couldn't create directory '{}'",
                                    new_dir.display()
                                ))),
                            }
                        } else {
                            self.read_working_dir();
                            result.should_refresh = true;

                            for (i, entry) in self.entries.iter().enumerate() {
                                if *entry == new_dir {
                                    self.table_state.select(Some(i));
                                    self.mode = PanelMode::Insert;
                                    self.edit_buffer.clear();
                                    self.cursor_offset = 0;
                                    self.table_state.select_column(Some(1));
                                }
                            }
                        }
                    } else if key_event == config.duplicate && self.entries.len() > 0 {
                        if let Some(current_entry) = self.table_state.selected() {
                            let selection_start =
                                if let Some(selection_start) = self.selection_start {
                                    self.selection_start = None;
                                    selection_start
                                } else {
                                    current_entry
                                };
                            let mut refresh = false;

                            for i in current_entry.min(selection_start)
                                ..=current_entry.max(selection_start)
                            {
                                let entry_path = &self.entries[i];
                                let new_entry_path = new_path(entry_path);

                                if entry_path.is_file() {
                                    if let Err(err) = std::fs::copy(entry_path, &new_entry_path) {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                self.errors.push(WalkedError::PathNotFound {
                                                    path: entry_path.clone(),
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            std::io::ErrorKind::PermissionDenied => {
                                                self.errors.push(WalkedError::PermissionDenied {
                                                    path: new_entry_path,
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            _ => self.errors.push(WalkedError::Message(format!(
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
                                            std::io::ErrorKind::PermissionDenied => {
                                                self.errors.push(WalkedError::PermissionDenied {
                                                    path: new_dir,
                                                    path_kind: PathKind::Dir,
                                                })
                                            }
                                            _ => self.errors.push(WalkedError::Message(format!(
                                                "Couldn't create directory '{}'",
                                                new_dir.display()
                                            ))),
                                        }
                                    } else {
                                        copy_recursively(entry_path, &new_dir, &mut self.errors);
                                    }
                                    refresh = true;
                                }
                            }
                            if refresh {
                                self.read_working_dir();
                                result.should_refresh = true;
                            }
                        }
                    } else if key_event == config.copy && self.entries.len() > 0 {
                        if let Some(current_entry) = self.table_state.selected() {
                            clipboard.clear();
                            if let Some(selection_start) = self.selection_start {
                                for i in current_entry.min(selection_start)
                                    ..=current_entry.max(selection_start)
                                {
                                    clipboard.push(self.entries[i].clone());
                                }
                            } else {
                                clipboard.push(self.entries[current_entry].clone());
                            }
                        }
                    } else if key_event == config.paste {
                        let mut refresh = false;
                        for entry_path in clipboard.iter() {
                            let new_entry_path = new_path(
                                self.working_directory.join(entry_path.file_name().unwrap()),
                            );

                            if entry_path.is_file() {
                                if let Err(err) = std::fs::copy(entry_path, &new_entry_path) {
                                    match err.kind() {
                                        std::io::ErrorKind::NotFound => {
                                            self.errors.push(WalkedError::PathNotFound {
                                                path: entry_path.clone(),
                                                path_kind: PathKind::File,
                                            })
                                        }
                                        std::io::ErrorKind::PermissionDenied => {
                                            self.errors.push(WalkedError::PermissionDenied {
                                                path: new_entry_path,
                                                path_kind: PathKind::File,
                                            })
                                        }
                                        _ => self.errors.push(WalkedError::Message(format!(
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
                                            self.errors.push(WalkedError::PermissionDenied {
                                                path: new_entry_path,
                                                path_kind: PathKind::Dir,
                                            })
                                        }
                                        _ => self.errors.push(WalkedError::Message(format!(
                                            "Couldn't create directory '{}'",
                                            new_entry_path.display()
                                        ))),
                                    }
                                } else {
                                    copy_recursively(entry_path, &new_entry_path, &mut self.errors);
                                }
                                refresh = true;
                            }
                        }
                        if refresh {
                            self.read_working_dir();
                            result.should_refresh = true;
                        }
                    } else if key_event == config.remove && self.entries.len() > 0 {
                        if let Some(current_entry) = self.table_state.selected() {
                            let selection_start =
                                if let Some(selection_start) = self.selection_start {
                                    self.selection_start = None;
                                    selection_start
                                } else {
                                    current_entry
                                };
                            let mut refresh = false;

                            for i in current_entry.min(selection_start)
                                ..=current_entry.max(selection_start)
                            {
                                let entry = &self.entries[i];
                                if entry.is_file() {
                                    if let Err(err) = std::fs::remove_file(entry) {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                self.errors.push(WalkedError::PathNotFound {
                                                    path: entry.clone(),
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            std::io::ErrorKind::PermissionDenied => {
                                                self.errors.push(WalkedError::PermissionDenied {
                                                    path: entry.clone(),
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            _ => self.errors.push(WalkedError::Message(format!(
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
                                                    self.errors.push(WalkedError::PathNotFound {
                                                        path: entry.clone(),
                                                        path_kind: PathKind::Dir,
                                                    })
                                                }
                                                std::io::ErrorKind::PermissionDenied => self
                                                    .errors
                                                    .push(WalkedError::PermissionDenied {
                                                        path: entry.clone(),
                                                        path_kind: PathKind::Dir,
                                                    }),
                                                _ => {
                                                    self.errors.push(WalkedError::Message(format!(
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
                                self.read_working_dir();
                                result.should_refresh = true;
                            }
                        }
                    } else if key_event == config.insert_mode {
                        if self.entries.len() > 0 {
                            self.mode = PanelMode::Insert;
                            if let Some(i) = self.table_state.selected() {
                                self.edit_buffer = {
                                    if let Some(p) = self.entries[i].file_name() {
                                        p.to_str().unwrap().to_string()
                                    } else {
                                        "".to_string()
                                    }
                                };
                            }
                            self.table_state.select_column(Some(1));
                        }
                    } else if key_event == config.quit {
                        result.quit = true;
                        return result;
                    }
                    self.refresh_cursor();
                }
                PanelMode::Insert => {
                    if key_event == config.normal_mode
                        || (key_event.code == KeyCode::Enter
                            && key_event.kind == KeyEventKind::Press)
                    {
                        let mut denied = false;
                        if let Some(i) = self.table_state.selected() {
                            if self.edit_buffer.len() > 0 && self.entries.len() > 0 {
                                let mut dist = self.working_directory.clone();
                                dist.push(&self.edit_buffer);
                                let disallowed_chars =
                                    ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
                                if self.edit_buffer.contains(&disallowed_chars) {
                                    self.mode = PanelMode::Insert;
                                    denied = true;
                                    self.errors.push(WalkedError::Message(format!("Paths can't contain the following characters: {disallowed_chars:?}")));
                                } else if dist.exists() {
                                    if dist != self.entries[i] {
                                        self.mode = PanelMode::Insert;
                                        denied = true;
                                        self.errors.push(WalkedError::Message(format!(
                                            "'{}' already exists",
                                            dist.display()
                                        )));
                                    }
                                } else {
                                    if let Err(err) = std::fs::rename(&self.entries[i], &dist) {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                self.errors.push(WalkedError::PathNotFound {
                                                    path: self.entries[i].clone(),
                                                    path_kind: PathKind::Ambigious,
                                                })
                                            }
                                            std::io::ErrorKind::PermissionDenied => {
                                                self.errors.push(WalkedError::PermissionDenied {
                                                    path: self.entries[i].clone(),
                                                    path_kind: PathKind::Ambigious,
                                                })
                                            }
                                            _ => self.errors.push(WalkedError::Message(format!(
                                                "Couldn't rename '{}' to '{}'",
                                                self.entries[i].display(),
                                                dist.display()
                                            ))),
                                        }
                                    } else {
                                        self.entries[i] = dist;
                                        result.should_refresh = true;
                                    }
                                }
                            }
                        }
                        if !denied {
                            self.mode = PanelMode::Normal;
                            self.table_state.select_column(None);
                            self.edit_buffer.clear();
                        }
                    } else if key_event.kind == KeyEventKind::Press {
                        if key_event.code == KeyCode::Backspace {
                            if self.cursor_offset > 0 {
                                let mut idx = self.edit_buffer.len() - 1;
                                for (i, (len, _)) in self.edit_buffer.char_indices().enumerate() {
                                    if i >= self.cursor_offset as usize {
                                        break;
                                    } else {
                                        idx = len;
                                    }
                                }
                                self.edit_buffer.remove(idx);
                                self.cursor_offset -= 1;
                            }
                        } else if let KeyCode::Char(c) = key_event.code {
                            let mut idx = self.edit_buffer.len();
                            for (i, (len, _)) in self.edit_buffer.char_indices().enumerate() {
                                if i == self.cursor_offset as usize {
                                    idx = len;
                                    break;
                                }
                            }
                            self.edit_buffer.insert(idx, c);
                            self.cursor_offset += 1;
                        }
                    }
                }
            }
        }

        result
    }

    pub fn refresh_cursor(&mut self) {
        if let Some(i) = self.table_state.selected() {
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
    pub fn walk(&mut self, current_entry: usize) -> bool {
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
    pub fn parent(&mut self) -> bool {
        if let Some(p) = self.working_directory.parent() {
            self.working_directory = p.to_path_buf();
            self.read_working_dir();
            return true;
        }
        false
    }
    pub fn read_working_dir(&mut self) {
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
