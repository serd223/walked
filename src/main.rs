mod config;

use std::path::PathBuf;

use config::Config;
use crossterm::event::{self, Event};
use ratatui::{
    layout::Constraint,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Padding, Row, Table, TableState},
    DefaultTerminal,
};

fn main() -> Result<(), std::io::Error> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

enum EditorMode {
    Normal,
    Insert,
}

impl EditorMode {
    fn to_string(&self, config: &Config) -> String {
        match self {
            &EditorMode::Normal => config.normal_mode_text.clone(),
            &EditorMode::Insert => config.insert_mode_text.clone(),
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
}

impl Editor {
    fn walk(&mut self, current_entry: usize) -> bool {
        if self.entries.len() <= 0 {
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

fn run(mut terminal: DefaultTerminal) -> Result<(), std::io::Error> {
    let current_dir =
        PathBuf::from(std::path::absolute(".").expect("Can't parse current working directory"));
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
    };
    ed.read_working_dir();
    ed.parent();

    let mut table_state = TableState::default();
    table_state.select_first();
    let mut quit = false;
    while !quit {
        let event = event::read()?;
        // Draw
        terminal.draw(|f| {
            // Entire area of the editor
            let view = Block::new()
                .padding(Padding::new(ed.left, 0, ed.top, ed.bottom))
                .title("walkEd".bold().into_centered_line())
                .title_bottom(
                    vec![
                        ed.mode.to_string(&ed.config).into(),
                        " | Quit ".into(),
                        "<Q> ".blue().bold(),
                    ]
                    .into_left_aligned_line(),
                );

            let content = ed
                .entries
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let last = {
                        if let Some(l) = p.file_name() {
                            l.to_os_string()
                        } else {
                            std::ffi::OsString::from("..")
                        }
                    };
                    let mut header = String::new();
                    if ed.config.show_entry_number {
                        header.push_str(&format!(
                            "{:w$}",
                            i,
                            w = ed.entries.len().to_string().chars().count()
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
                        header.push_str(&format!("{entry_type}"));
                    }
                    Row::new([header, last.to_str().unwrap().to_string()])
                })
                .collect::<Vec<Row>>();

            // Update
            if let Event::Key(key_event) = event {
                if key_event == ed.config.dir_walk {
                    if let Some(i) = table_state.selected() {
                        if ed.walk(i) {
                            table_state.select_first();
                        }
                    }
                } else if key_event == ed.config.dir_up {
                    if ed.parent() {
                        table_state.select_first();
                    }
                } else if key_event == ed.config.up {
                    table_state.scroll_up_by(1);
                } else if key_event == ed.config.down {
                    table_state.scroll_down_by(1);
                } else if key_event == ed.config.quit {
                    quit = true;
                    return;
                }
            }
            f.render_stateful_widget(
                Table::default()
                    .widths([Constraint::Length(8), Constraint::Min(0)])
                    .rows(content)
                    .block(view)
                    .row_highlight_style(Style::new().reversed())
                    .column_highlight_style(Style::new().red())
                    .cell_highlight_style(Style::new().blue())
                    .highlight_symbol(">>"),
                f.area(),
                &mut table_state,
            );
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
