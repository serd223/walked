use crate::{PathKind, WalkedError, config::Config};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::TableState;
use std::{
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
    str::FromStr,
};

pub const TABLE_HEADER_MIN_WIDTH: u16 = 8;
// pub const NEW_DIRECTORY_TEXT: &'static str = ".#NEWDIR";
// pub const NEW_FILE_TEXT: &'static str = ".#NEWFILE";

#[derive(Clone)]
pub enum CommandKind {
    NewFile,
    NewDirectory,
    IncrementalSearch,
    #[allow(dead_code)]
    Custom(String), // NOTE: For future if we need plugins or such
}

impl ToString for CommandKind {
    fn to_string(&self) -> String {
        match self {
            CommandKind::NewFile => "new-file".to_string(),
            CommandKind::NewDirectory => "new-directory".to_string(),
            CommandKind::IncrementalSearch => "incremental-search".to_string(),
            CommandKind::Custom(s) => s.clone(),
        }
    }
}

pub struct Command {
    pub kind: CommandKind,
    pub arg: String,
}

pub struct Window {
    pub panel: Panel,
    pub clipboard: Vec<PathBuf>,
    pub config: Config,
}

#[derive(PartialEq, Eq)]
pub enum PanelMode {
    Normal,
    Prompt,
    Search(Option<String>),
    Insert,
}

#[derive(Default)]
pub struct LsEntry {
    pub file: PathBuf,
    pub perms: String,
    pub file_count: String,
    pub owner: String,
    pub group: String,
    pub size: String,
    pub date: String,
    pub name: String,
}

impl LsEntry {
    fn from_ls_line(working_directory: &Path, s: &str) -> Result<Self, ()> {
        let mut result = Self::default();
        enum State {
            ParsingName(usize),
            SkippingWhiteSpace,
        }
        let mut current_column = 0;
        let mut sub_column = 0;
        let mut state = State::SkippingWhiteSpace;
        for c in s.chars() {
            match state {
                State::ParsingName(n) => {
                    if c.is_whitespace() {
                        state = State::SkippingWhiteSpace;
                        current_column = n + 1;
                        if n == 5 {
                            if sub_column == 0 || sub_column == 1 {
                                current_column = n;
                                sub_column += 1;
                            }
                        }
                        if n == 6 {
                            current_column = n;
                        }
                    } else {
                        result[current_column].push(c)
                    }
                }
                State::SkippingWhiteSpace => {
                    if c.is_whitespace() {
                        if current_column == 5 && sub_column <= 2 {
                            result[current_column].push(c)
                        }
                        if current_column == 6 {
                            result[current_column].push(c)
                        }
                        continue;
                    } else {
                        state = State::ParsingName(current_column);
                        result[current_column].push(c)
                    }
                }
            }
        }
        result.file = working_directory.join(Path::new(&result.name));
        if current_column < 6 {
            return Err(());
        }
        Ok(result)
    }
}

impl Index<usize> for LsEntry {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.perms,
            1 => &self.file_count,
            2 => &self.owner,
            3 => &self.group,
            4 => &self.size,
            5 => &self.date,
            6 => &self.name,
            _ => {
                panic!("Index out of bounds for LsEntry")
            }
        }
    }
}

impl IndexMut<usize> for LsEntry {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.perms,
            1 => &mut self.file_count,
            2 => &mut self.owner,
            3 => &mut self.group,
            4 => &mut self.size,
            5 => &mut self.date,
            6 => &mut self.name,
            _ => {
                panic!("Index out of bounds for LsEntry")
            }
        }
    }
}

pub struct Panel {
    pub errors: Vec<WalkedError>,
    pub table_state: TableState,
    pub mode: PanelMode,
    pub left: u16,
    pub top: u16,
    pub bottom: u16,
    pub entries: Vec<LsEntry>,
    pub incremental_search_results: Vec<usize>,
    pub current_incremental_search_result: usize,
    pub working_directory: PathBuf,
    pub edit_buffer: String,
    pub cursor_offset: u16,
    pub current_entry_length: usize,
    pub header_width: u16,
    pub selection_start: Option<usize>,
    pub queue: Vec<Command>,
    pub command_prompt: Option<CommandKind>,
}

pub struct PanelFrameData {
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
            incremental_search_results: vec![],
            current_incremental_search_result: 0,
            edit_buffer: String::new(),
            cursor_offset: 0,
            current_entry_length: 0,
            header_width: TABLE_HEADER_MIN_WIDTH,
            selection_start: None,
            queue: Vec::new(),
            command_prompt: None,
        };
        panel.read_working_dir();
        panel.table_state.select_first();
        panel.refresh_cursor();
        panel
    }

    pub fn prompt(&mut self, cmd: CommandKind) {
        self.mode = PanelMode::Prompt;
        self.command_prompt = Some(cmd);
        self.edit_buffer.clear();
    }

    pub fn process_command_queue(&mut self) {
        if self.queue.len() > 0 {
            let queue = self.queue.drain(..).collect::<Vec<_>>();
            for cmd in queue {
                match cmd.kind {
                    CommandKind::NewFile => {
                        let new_file = new_path(self.working_directory.join(cmd.arg));
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

                            for (i, entry) in self.entries.iter().enumerate() {
                                if entry.file == new_file {
                                    self.table_state.select(Some(i));
                                    self.cursor_offset = 0;
                                    self.table_state.select_column(Some(1));
                                }
                            }
                        }
                    }
                    CommandKind::NewDirectory => {
                        let new_dir = new_path(self.working_directory.join(cmd.arg));
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

                            for (i, entry) in self.entries.iter().enumerate() {
                                if entry.file == new_dir {
                                    self.table_state.select(Some(i));
                                    self.cursor_offset = 0;
                                    self.table_state.select_column(Some(1));
                                }
                            }
                        }
                    }
                    CommandKind::IncrementalSearch => {
                        self.incremental_search_results.clear();
                        for (i, entry) in self.entries.iter().enumerate() {
                            if entry.name.to_lowercase().contains(&cmd.arg.to_lowercase()) {
                                self.incremental_search_results.push(i);
                            }
                        }
                        if self.incremental_search_results.len() > 0 {
                            if let Some(selected) = self.table_state.selected() {
                                if let Some((result_index, &entry_index)) = self
                                    .incremental_search_results
                                    .iter()
                                    .enumerate()
                                    .find(|(_, i)| **i >= selected)
                                {
                                    self.table_state.select(Some(entry_index));
                                    self.current_incremental_search_result = result_index;
                                    self.cursor_offset = 0;
                                    self.table_state.select_column(Some(1));
                                } else {
                                    self.current_incremental_search_result = 0;
                                    self.table_state
                                        .select(Some(self.incremental_search_results[0]));
                                    self.cursor_offset = 0;
                                    self.table_state.select_column(Some(1));
                                }
                            }
                            if self.incremental_search_results.len() > 1 {
                                self.mode = PanelMode::Search(Some(format!(" ({})", cmd.arg)));
                            }
                        } else {
                            self.mode = PanelMode::Search(Some(" <no mathces found>".to_string()));
                        }
                    }
                    CommandKind::Custom(_) => todo!(),
                }
            }
        }
    }

    /// Returns false if quit was pressed
    pub fn update(
        &mut self,
        key_event: KeyEvent,
        clipboard: &mut Vec<PathBuf>,
        config: &Config,
    ) -> PanelFrameData {
        let mut result = PanelFrameData { quit: false };

        if self.errors.len() > 0 {
            if key_event.kind == KeyEventKind::Press {
                self.errors.clear();
            }
        } else {
            match self.mode {
                PanelMode::Prompt => {
                    if key_event == config.quit {
                        result.quit = true;
                        return result;
                    } else if key_event.code == KeyCode::Enter && key_event.is_press() {
                        self.queue.push(Command {
                            kind: self.command_prompt.clone().unwrap(),
                            arg: self.edit_buffer.clone(),
                        });
                        self.edit_buffer.clear();
                        self.command_prompt = None;
                        self.mode = PanelMode::Normal;
                    } else if key_event.code == KeyCode::Esc && key_event.is_press() {
                        self.edit_buffer.clear();
                        self.command_prompt = None;
                        self.mode = PanelMode::Normal;
                    } else if key_event.code == KeyCode::Backspace && key_event.is_press() {
                        self.edit_buffer.pop();
                    } else if let KeyCode::Char(c) = key_event.code
                        && key_event.is_press()
                    {
                        self.edit_buffer.push(c);
                    }
                }
                PanelMode::Search(_) => {
                    if key_event == config.quit {
                        result.quit = true;
                        return result;
                    } else if key_event.code == KeyCode::Esc {
                        self.mode = PanelMode::Normal;
                    } else if key_event == config.dir_walk {
                        if self.walk(
                            self.incremental_search_results[self.current_incremental_search_result],
                        ) {
                            self.table_state.select_first();
                            self.refresh_cursor();
                            self.mode = PanelMode::Normal;
                        }
                    } else if key_event == config.next_search_result
                        && self.incremental_search_results.len() > 0
                    {
                        if self.current_incremental_search_result + 1
                            >= self.incremental_search_results.len()
                        {
                            self.current_incremental_search_result = 0;
                        } else {
                            self.current_incremental_search_result += 1;
                        }
                        self.table_state.select(Some(
                            self.incremental_search_results[self.current_incremental_search_result],
                        ));
                        self.cursor_offset = 0;
                        self.table_state.select_column(Some(1));
                    } else if key_event == config.prev_search_result
                        && self.incremental_search_results.len() > 0
                    {
                        if self.current_incremental_search_result <= 0 {
                            self.current_incremental_search_result =
                                self.incremental_search_results.len() - 1;
                        } else {
                            self.current_incremental_search_result -= 1;
                        }
                        self.table_state.select(Some(
                            self.incremental_search_results[self.current_incremental_search_result],
                        ));
                        self.cursor_offset = 0;
                        self.table_state.select_column(Some(1));
                    }
                }
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
                    } else if key_event == config.incremental_search {
                        self.prompt(CommandKind::IncrementalSearch);
                    } else if key_event == config.new_file {
                        self.prompt(CommandKind::NewFile);
                    } else if key_event == config.new_directory {
                        self.prompt(CommandKind::NewDirectory);
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
                                let entry = &self.entries[i];
                                let new_entry_path = new_path(&entry.file);

                                if entry.file.is_file() {
                                    if let Err(err) = std::fs::copy(&entry.file, &new_entry_path) {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                self.errors.push(WalkedError::PathNotFound {
                                                    path: entry.file.clone(),
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
                                                entry.name,
                                                new_entry_path.display()
                                            ))),
                                        }
                                    }

                                    refresh = true;
                                } else if entry.file.is_dir() {
                                    let new_dir = new_path(&entry.file);
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
                                        copy_recursively(&entry.file, &new_dir, &mut self.errors);
                                    }
                                    refresh = true;
                                }
                            }
                            if refresh {
                                self.read_working_dir();
                            }
                        }
                    } else if key_event == config.copy && self.entries.len() > 0 {
                        if let Some(current_entry) = self.table_state.selected() {
                            clipboard.clear();
                            if let Some(selection_start) = self.selection_start {
                                for i in current_entry.min(selection_start)
                                    ..=current_entry.max(selection_start)
                                {
                                    if self.entries[i].name != ".." && self.entries[i].name != "." {
                                        clipboard.push(self.entries[i].file.clone());
                                    }
                                }
                            } else {
                                if self.entries[current_entry].name != ".."
                                    && self.entries[current_entry].name != "."
                                {
                                    clipboard.push(self.entries[current_entry].file.clone());
                                }
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
                                if entry.name == "." || entry.name == ".." {
                                    continue;
                                }
                                if entry.file.is_file() {
                                    if let Err(err) = std::fs::remove_file(&entry.file) {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                self.errors.push(WalkedError::PathNotFound {
                                                    path: entry.file.clone(),
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            std::io::ErrorKind::PermissionDenied => {
                                                self.errors.push(WalkedError::PermissionDenied {
                                                    path: entry.file.clone(),
                                                    path_kind: PathKind::File,
                                                })
                                            }
                                            _ => self.errors.push(WalkedError::Message(format!(
                                                "Couldn't remove file '{}'",
                                                entry.name
                                            ))),
                                        }
                                    }
                                    refresh = true;
                                } else if entry.file.is_dir() {
                                    if let Ok(dir) = std::fs::read_dir(&entry.file) {
                                        if let Err(err) = if dir.count() > 0 {
                                            std::fs::remove_dir_all(&entry.file)
                                        } else {
                                            std::fs::remove_dir(&entry.file)
                                        } {
                                            match err.kind() {
                                                std::io::ErrorKind::NotFound => {
                                                    self.errors.push(WalkedError::PathNotFound {
                                                        path: entry.file.clone(),
                                                        path_kind: PathKind::Dir,
                                                    })
                                                }
                                                std::io::ErrorKind::PermissionDenied => self
                                                    .errors
                                                    .push(WalkedError::PermissionDenied {
                                                        path: entry.file.clone(),
                                                        path_kind: PathKind::Dir,
                                                    }),
                                                _ => {
                                                    self.errors.push(WalkedError::Message(format!(
                                                        "Couldn't remove directory '{}'",
                                                        entry.name
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
                            }
                        }
                    } else if key_event == config.insert_mode {
                        if self.entries.len() > 0 {
                            self.mode = PanelMode::Insert;
                            if let Some(i) = self.table_state.selected() {
                                self.edit_buffer = self.entries[i].name.clone();
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
                                    if dist != self.entries[i].file {
                                        self.mode = PanelMode::Insert;
                                        denied = true;
                                        self.errors.push(WalkedError::Message(format!(
                                            "'{}' already exists",
                                            dist.display()
                                        )));
                                    }
                                } else if self.entries[i].name != "."
                                    || self.entries[i].name != ".."
                                {
                                    if let Err(err) = std::fs::rename(&self.entries[i].file, &dist)
                                    {
                                        match err.kind() {
                                            std::io::ErrorKind::NotFound => {
                                                self.errors.push(WalkedError::PathNotFound {
                                                    path: self.entries[i].file.clone(),
                                                    path_kind: PathKind::Ambigious,
                                                })
                                            }
                                            std::io::ErrorKind::PermissionDenied => {
                                                self.errors.push(WalkedError::PermissionDenied {
                                                    path: self.entries[i].file.clone(),
                                                    path_kind: PathKind::Ambigious,
                                                })
                                            }
                                            _ => self.errors.push(WalkedError::Message(format!(
                                                "Couldn't rename '{}' to '{}'",
                                                self.entries[i].name,
                                                dist.display()
                                            ))),
                                        }
                                    } else {
                                        self.read_working_dir();
                                    }
                                }
                            }
                        }
                        if !denied {
                            self.mode = PanelMode::Normal;
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
                self.current_entry_length = self.entries[i].name.chars().count();
                self.cursor_offset = self.cursor_offset.min(self.current_entry_length as u16)
            }
        }
    }
    pub fn walk(&mut self, current_entry: usize) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        if self.entries[current_entry].name == "." {
            return true;
        }
        if self.entries[current_entry].name == ".." {
            return self.parent();
        }
        let selected = &self.entries[current_entry];
        if selected.file.is_dir() {
            self.working_directory = selected.file.clone();
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
        if let Ok(ls_out) = std::process::Command::new("/bin/ls")
            .arg("-lah")
            .arg(&self.working_directory)
            .output()
        {
            if let Ok(ls_out) = str::from_utf8(&ls_out.stdout) {
                self.entries.clear();
                for l in ls_out.lines() {
                    if let Ok(e) = LsEntry::from_ls_line(&self.working_directory, l) {
                        self.entries.push(e);
                    }
                }
            }
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
