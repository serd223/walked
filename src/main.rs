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

const TABLE_HEADER_WIDTH: u16 = 8;
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
            if let Ok(c) = toml::from_str(&config_str) {
                config = c;
            }
        } else {
            let config_str =
                toml::to_string_pretty(&config).expect("Couldn't parse keybinds to config file.");
            std::fs::write(&args[1], config_str)?;
        }
    }

    let mut ed = Editor {
        config,
        clipboard: PathBuf::new(),
        mode: EditorMode::Normal,
        left: 2,
        top: 2,
        bottom: 1,
        working_directory: current_dir,
        entries: vec![],
        edit_buffer: String::new(),
        cursor_offset: 0,
        current_entry_length: 0,
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
    clipboard: PathBuf,
    mode: EditorMode,
    left: u16,
    top: u16,
    bottom: u16,
    entries: Vec<PathBuf>,
    working_directory: PathBuf,
    edit_buffer: String,
    cursor_offset: u16,
    current_entry_length: usize,
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
                self.current_entry_length = name.len();
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

    let mut quit = false;
    while !quit {
        let event = event::read()?;
        terminal.draw(|f| {
            if let Event::Key(key_event) = event {
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
                            table_state.scroll_up_by(1);
                            ed.refresh_cursor(&table_state);
                        } else if key_event == ed.config.down {
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
                            // TODO: Handle Error
                            let _ = std::fs::File::create(new_path(
                                ed.working_directory.join("NEWFILE"),
                            ));
                            ed.read_working_dir();
                        } else if key_event == ed.config.new_directory {
                            // TODO: Handle Error
                            let _ =
                                std::fs::create_dir(new_path(ed.working_directory.join("NEWDIR")));
                            ed.read_working_dir();
                        } else if key_event == ed.config.duplicate && ed.entries.len() > 0 {
                            if let Some(current_entry) = table_state.selected() {
                                let entry_path = &ed.entries[current_entry];
                                let new_entry_path = new_path(entry_path);

                                // TODO: Add recursive directory duplication
                                if entry_path.is_file() {
                                    // TODO: Handle Error
                                    let _ = std::fs::copy(entry_path, new_entry_path);
                                    ed.read_working_dir();
                                }
                            }
                        } else if key_event == ed.config.copy && ed.entries.len() > 0 {
                            if let Some(current_entry) = table_state.selected() {
                                ed.clipboard = ed.entries[current_entry].clone();
                            }
                        } else if key_event == ed.config.paste {
                            let entry_path = &ed.clipboard;

                            if entry_path.is_file() {
                                let new_entry_path = new_path(
                                    ed.working_directory.join(entry_path.file_name().unwrap()),
                                );
                                // TODO: Handle Error
                                let _ = std::fs::copy(entry_path, new_entry_path);
                                ed.read_working_dir();
                            }
                        } else if key_event == ed.config.remove && ed.entries.len() > 0 {
                            if let Some(current_entry) = table_state.selected() {
                                let entry = &ed.entries[current_entry];
                                if entry.is_file() {
                                    // TODO: Handle Error
                                    let _ = std::fs::remove_file(entry);
                                    ed.read_working_dir();
                                } else if entry.is_dir() {
                                    if let Ok(dir) = std::fs::read_dir(entry) {
                                        if dir.count() > 0 {
                                            // TODO: Handle Error
                                            let _ = std::fs::remove_dir_all(entry);
                                            ed.read_working_dir();
                                        } else {
                                            let _ = std::fs::remove_dir(entry);
                                            ed.read_working_dir();
                                        }
                                    }
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
                                    f.set_cursor_position((
                                        ed.left + TABLE_HEADER_WIDTH + 1,
                                        ed.top + 1 + i as u16,
                                    ));
                                }
                                table_state.select_column(Some(1));
                            }
                        } else if key_event == ed.config.quit {
                            quit = true;
                            return;
                        }
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
                                    if dist.exists() {
                                        if dist != ed.entries[i] {
                                            ed.mode = EditorMode::Insert;
                                            denied = true;
                                        }
                                    } else {
                                        // TODO: Handle Error
                                        let _ = std::fs::rename(&ed.entries[i], &dist);
                                        ed.entries[i] = dist;
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
                                    ed.edit_buffer.remove(ed.cursor_offset as usize - 1);
                                    ed.cursor_offset -= 1;
                                }
                            } else if let KeyCode::Char(c) = key_event.code {
                                ed.edit_buffer.insert(ed.cursor_offset as usize, c);
                                ed.cursor_offset += 1;
                            }
                        }
                    }
                }
            }
            let view = Block::new()
                .padding(Padding::new(ed.left, 0, ed.top, ed.bottom))
                .title(ed.working_directory.to_str().unwrap().into_centered_line())
                .title_bottom(vec![ed.mode.to_string(&ed.config).into()].into_centered_line());

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
                        + TABLE_HEADER_WIDTH
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
                    Row::new([header, last.to_str().unwrap().to_string()])
                })
                .collect::<Vec<Row>>();

            match ed.mode {
                EditorMode::Normal => {
                    f.render_stateful_widget(
                        Table::default()
                            .widths([Constraint::Length(TABLE_HEADER_WIDTH), Constraint::Min(0)])
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
                            .widths([Constraint::Length(TABLE_HEADER_WIDTH), Constraint::Min(0)])
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
    Ok(())
}

#[allow(dead_code)]
trait IntoLine<'a> {
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
