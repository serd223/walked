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
    text::{Line, Span},
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

fn main() -> Result<(), std::io::Error> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stderr(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::SetCursorStyle::SteadyBlock
    )?;
    let mut terminal = Terminal::new(CrosstermBackend::new(BufWriter::new(std::io::stderr())))?;
    let current_dir = std::path::absolute(".").expect("Can't parse current working directory");
    let current_dir2 = current_dir.clone();
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
    crossterm::execute!(
        std::io::stderr(),
        crossterm::cursor::SetCursorStyle::DefaultUserShape,
        crossterm::terminal::LeaveAlternateScreen
    )?;
    match result {
        Ok(Some(wd)) => {
            println!("{}", wd.to_str().unwrap());
            Ok(())
        }
        Ok(None) => {
            println!("{}", current_dir2.to_str().unwrap());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

impl PanelMode {
    fn to_string(&self, config: &Config) -> String {
        match self {
            PanelMode::Normal => config.normal_mode_text.clone(),
            PanelMode::Prompt => config.normal_mode_text.clone(),
            PanelMode::Search(None) => config.search_mode_text.clone(),
            PanelMode::Search(Some(msg)) => format!("{}{msg}", config.search_mode_text),
            PanelMode::Insert => config.insert_mode_text.clone(),
        }
    }
}

fn run<W: ratatui::prelude::Backend>(
    terminal: &mut Terminal<W>,
    config: Config,
    current_dir: PathBuf,
) -> Result<Option<PathBuf>, std::io::Error> {
    let mut window = Window {
        panel: Panel::new(current_dir),
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
            let res = window
                .panel
                .update(key_event, &mut window.clipboard, &window.config);

            window.panel.process_command_queue();
            if res.quit {
                if res.dont_write_stdout {
                    return Ok(None);
                } else {
                    return Ok(Some(window.panel.working_directory.clone()));
                }
            }
        }

        terminal.draw(|f| {
            let area = f.area();
            let panel = &mut window.panel;
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

            let mut col_widths = [0u16; 7];
            for entry in &panel.entries {
                for col in 0..7 {
                    col_widths[col] = col_widths[col].max(entry[col].chars().count() as u16);
                }
            }

            let content = panel
                .entries
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let is_in_selection = {
                        if let Some(selection_start) = panel.selection_start {
                            if let Some(cur) = panel.table_state.selected() {
                                if i == cur {
                                    true
                                } else {
                                    if cur > selection_start {
                                        i < cur && i >= selection_start
                                    } else if cur < selection_start {
                                        i > cur && i <= selection_start
                                    } else {
                                        false
                                    }
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    };

                    let cells: Vec<Line> = (0..7)
                        .map(|col| {
                            if col == 0 && !is_in_selection {
                                let spans: Vec<Span> = p[col]
                                    .chars()
                                    .map(|c| match c {
                                        'r' => "r".blue(),
                                        'w' => "w".green(),
                                        'x' => "x".red(),
                                        'd' => "d".yellow(),
                                        _ => Span::raw(c.to_string()),
                                    })
                                    .collect();
                                let line = Line::from(spans);
                                return line;
                            }
                            let text = if col == 6 && panel.mode == PanelMode::Insert {
                                if let Some(selected) = panel.table_state.selected() {
                                    if selected == i {
                                        &panel.edit_buffer
                                    } else {
                                        &p[col]
                                    }
                                } else {
                                    &p[col]
                                }
                            } else {
                                &p[col]
                            };

                            let text = if is_in_selection {
                                text.as_str().into_line().reversed()
                            } else {
                                text.as_str().into_line()
                            };

                            if col == 6 && p.perms.starts_with('d') {
                                text.blue()
                            } else {
                                text
                            }
                        })
                        .collect();

                    Row::new(cells)
                })
                .collect::<Vec<Row>>();
            if let Some(i) = panel.table_state.selected() {
                let row_offset = {
                    if i < panel.table_state.offset() {
                        0
                    } else if panel.entries.len() > 0 {
                        (i - panel.table_state.offset()).min(
                            (panel.entries.len() - 1).min(view.inner(area).height as usize - 1),
                        ) as u16
                    } else {
                        0
                    }
                };
                let selected_col = panel.table_state.selected_column().unwrap_or(6).min(6);
                let mut col_offset = 0u16;
                for c in 0..selected_col {
                    col_offset += col_widths[c] + 1;
                }
                f.set_cursor_position((
                    area.x + panel.left + col_offset + panel.cursor_offset,
                    area.y + panel.top + 1 + row_offset,
                ));
            }

            let constraints = [
                Constraint::Length(col_widths[0]),
                Constraint::Length(col_widths[1]),
                Constraint::Length(col_widths[2]),
                Constraint::Length(col_widths[3]),
                Constraint::Length(col_widths[4]),
                Constraint::Length(col_widths[5]),
                Constraint::Min(0),
            ];
            let table = Table::default()
                .widths(constraints)
                .rows(content)
                .block(view.clone())
                .cell_highlight_style(Style::new().reversed());
            match panel.mode {
                PanelMode::Prompt => {
                    let mut top_area = area;
                    top_area.height -= 2;
                    let mut bottom_area = top_area;
                    bottom_area.y += top_area.height;
                    bottom_area.height = 2;
                    f.render_stateful_widget(table, top_area, &mut panel.table_state);
                    if let Some(cmd) = &panel.command_prompt {
                        f.render_widget(
                            format!("({}) >{}_", cmd.to_string(), panel.edit_buffer),
                            bottom_area,
                        );
                    } else {
                        f.render_widget(format!(":{}_", panel.edit_buffer), bottom_area);
                    }
                }
                PanelMode::Normal | PanelMode::Search(_) => {
                    f.render_stateful_widget(table, area, &mut panel.table_state);
                }
                PanelMode::Insert => {
                    f.render_stateful_widget(
                        table.cell_highlight_style(Style::new().underlined()),
                        area,
                        &mut panel.table_state,
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
