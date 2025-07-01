mod config;
mod window;

use std::{io::BufWriter, path::PathBuf};

use config::Config;
use crossterm::event::{self, Event};
use ratatui::{
    Terminal,
    layout::Constraint,
    prelude::CrosstermBackend,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Padding, Row, Table},
};
use window::{Panel, PanelMode, Window};

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

    let result = run(&mut terminal, config, current_dir);
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    match result {
        Ok(wd) => {
            println!("{}", wd.to_str().unwrap());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

impl PanelMode {
    fn to_string(&self, config: &Config) -> String {
        match *self {
            PanelMode::Normal => config.normal_mode_text.clone(),
            PanelMode::Prompt => config.normal_mode_text.clone(),
            PanelMode::Insert => config.insert_mode_text.clone(),
        }
    }
}

fn run<W: ratatui::prelude::Backend>(
    terminal: &mut Terminal<W>,
    config: Config,
    current_dir: PathBuf,
) -> Result<PathBuf, std::io::Error> {
    let mut window = Window {
        panels: vec![vec![Panel::new(current_dir)]],
        panel_focus_i: 0,
        panel_focus_j: 0,
        clipboard: Vec::new(),
        config,
    };

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

        if let Event::Key(key_event) = event {
            if key_event == window.config.pane_up {
                window.pane_up();
            } else if key_event == window.config.pane_down {
                window.pane_down();
            } else if key_event == window.config.pane_left {
                window.pane_left();
            } else if key_event == window.config.pane_right {
                window.pane_right();
            } else if key_event == window.config.split_pane_up {
                window.split_up();
            } else if key_event == window.config.split_pane_down {
                window.split_down();
            } else if key_event == window.config.split_pane_left {
                window.split_left();
            } else if key_event == window.config.split_pane_right {
                window.split_right();
            } else if key_event == window.config.close_active_pane {
                window.close_active();
            } else {
                let mut res = window.panels[window.panel_focus_i][window.panel_focus_j].update(
                    key_event,
                    &mut window.clipboard,
                    &window.config,
                );

                window.panels[window.panel_focus_i][window.panel_focus_j]
                    .process_command_queue(&mut res);
                if res.quit {
                    return Ok(window.panel().working_directory.clone());
                }
                if res.should_refresh {
                    for i in 0..window.panels.len() {
                        for j in 0..window.panels[i].len() {
                            if i == window.panel_focus_i && j == window.panel_focus_j {
                                continue;
                            }
                            window.panels[i][j].read_working_dir();
                            window.panels[i][j].refresh_cursor();
                        }
                    }
                }
            }
        }

        terminal.draw(|f| {
            let mut area = f.area();
            let width = area.width;
            let height_per_pane = area.height / window.panels.len() as u16; // NOTE: assumes that there are panels
            area.height = height_per_pane;
            let start = area.x;
            for i in 0..window.panels.len() {
                area.x = start;
                let width_per_pane = width / window.panels[i].len() as u16;
                area.width = width_per_pane;
                for j in 0..window.panels[i].len() {
                    let panel = &mut window.panels[i][j];
                    let view = Block::new()
                        .padding(Padding::new(panel.left, 0, panel.top, panel.bottom))
                        .title(if panel.errors.len() > 0 {
                            {
                                let mut res = String::new();
                                for err in panel.errors.iter() {
                                    res.push_str(&format!("{err} "));
                                }
                                res
                            }
                            .into_left_aligned_line()
                            .red()
                        } else {
                            panel
                                .working_directory
                                .to_str()
                                .unwrap()
                                .to_string()
                                .into_centered_line()
                        })
                        .title_bottom(panel.mode.to_string(&window.config).into_centered_line());

                    let content = panel
                        .entries
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let mut header = String::new();
                            if window.config.show_entry_number {
                                header.push_str(&format!(
                                    "{:w$}",
                                    i,
                                    w = (panel.entries.len() - 1).to_string().chars().count()
                                ))
                            }
                            if window.config.show_entry_type {
                                let entry_type = {
                                    if panel.entries[i].is_file() {
                                        &window.config.file_text
                                    } else if panel.entries[i].is_dir() {
                                        &window.config.directory_text
                                    } else if panel.entries[i].is_symlink() {
                                        &window.config.symlink_text
                                    } else {
                                        &window.config.other_text
                                    }
                                };
                                if window.config.show_entry_number {
                                    header.push(':');
                                }
                                header.push_str(entry_type);
                            }
                            if let Ok(metadata) = std::fs::metadata(&panel.entries[i]) {
                                if panel.entries[i].is_file() {
                                    let size = bytesize::ByteSize::b(metadata.len());
                                    header.push_str(&format!(" {}", size));
                                } else {
                                    header.push_str(" - ");
                                }
                            }
                            panel.header_width =
                                (header.chars().count() as u16).max(panel.header_width);
                            let last = {
                                if let Some(l) = p.file_name() {
                                    l.to_os_string()
                                } else {
                                    std::ffi::OsString::from("..")
                                }
                            };
                            if panel.mode == PanelMode::Insert {
                                if let Some(selected) = panel.table_state.selected() {
                                    if selected == i {
                                        return Row::new([header, panel.edit_buffer.clone()]);
                                    }
                                }
                            }
                            let is_in_selection = {
                                if let Some(selection_start) = panel.selection_start {
                                    if let Some(cur) = panel.table_state.selected() {
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
                    if i == window.panel_focus_i && j == window.panel_focus_j {
                        if let Some(i) = panel.table_state.selected() {
                            let row_offset = {
                                if i < panel.table_state.offset() {
                                    0
                                } else if panel.entries.len() > 0 {
                                    (i - panel.table_state.offset()).min(
                                        (panel.entries.len() - 1)
                                            .min(view.inner(area).height as usize - 1),
                                    ) as u16
                                } else {
                                    0
                                }
                            };
                            f.set_cursor_position((
                                area.x
                                    + panel.left
                                    + panel.header_width
                                    + 1
                                    + panel.cursor_offset
                                    + if panel.mode == PanelMode::Normal {
                                        HIGHLIGHT_SYMBOL.chars().count() as u16
                                    } else {
                                        0
                                    },
                                area.y + panel.top + 1 + row_offset,
                            ));
                        }
                    }

                    match panel.mode {
                        PanelMode::Prompt => {
                            let mut top_area = area;
                            top_area.height -= 2;
                            let mut bottom_area = top_area;
                            bottom_area.y += top_area.height;
                            bottom_area.height = 2;
                            f.render_stateful_widget(
                                Table::default()
                                    .widths([
                                        Constraint::Length(panel.header_width),
                                        Constraint::Min(0),
                                    ])
                                    .rows(content)
                                    .block(view)
                                    .row_highlight_style(Style::new().reversed())
                                    .highlight_symbol(HIGHLIGHT_SYMBOL),
                                top_area,
                                &mut panel.table_state,
                            );
                            if let Some(cmd) = &panel.command_prompt {
                                f.render_widget(
                                    format!("({}) >{}_", cmd.to_string(), panel.edit_buffer),
                                    bottom_area,
                                );
                            } else {
                                f.render_widget(format!(">{}_", panel.edit_buffer), bottom_area);
                            }
                        }
                        PanelMode::Normal => {
                            f.render_stateful_widget(
                                Table::default()
                                    .widths([
                                        Constraint::Length(panel.header_width),
                                        Constraint::Min(0),
                                    ])
                                    .rows(content)
                                    .block(view)
                                    .row_highlight_style(Style::new().reversed())
                                    .highlight_symbol(HIGHLIGHT_SYMBOL),
                                area,
                                &mut panel.table_state,
                            );
                        }
                        PanelMode::Insert => {
                            f.render_stateful_widget(
                                Table::default()
                                    .widths([
                                        Constraint::Length(panel.header_width),
                                        Constraint::Min(0),
                                    ])
                                    .rows(content)
                                    .block(view)
                                    .cell_highlight_style(Style::new().underlined()),
                                area,
                                &mut panel.table_state,
                            );
                        }
                    }
                    area.x += width_per_pane;
                }
                area.y += height_per_pane;
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
