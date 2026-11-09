use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use toml::Value;

pub enum ClipboardType {
    TemporaryFile,
    File(String),
    InMemory,
}

pub enum TmuxIntegration {
    FromEnv,
    Always,
    Never,
}

pub struct Config {
    pub tmux_integration: TmuxIntegration,
    pub tmux_display_popup_command: Vec<String>,
    pub clipboard_file: ClipboardType,
    pub normal_mode_text: String,
    pub select_mode_text: String,
    pub search_mode_text: String,
    pub insert_mode_text: String,
    pub shell_command_placeholder: String,
    pub show_working_directory: bool,
    pub simple_working_directory: bool,
    pub new_file: KeyEvent,
    pub new_directory: KeyEvent,
    pub duplicate: KeyEvent,
    pub remove: KeyEvent,
    pub copy: KeyEvent,
    pub paste: KeyEvent,
    pub incremental_search: KeyEvent,
    pub next_search_result: KeyEvent,
    pub prev_search_result: KeyEvent,
    pub up: KeyEvent,
    pub down: KeyEvent,
    pub left: KeyEvent,
    pub column_left: KeyEvent,
    pub right: KeyEvent,
    pub column_right: KeyEvent,
    pub dir_walk: KeyEvent,
    pub dir_up: KeyEvent,
    pub insert_mode: KeyEvent,
    pub normal_mode: KeyEvent,
    pub select_mode: KeyEvent,
    pub command_mode: KeyEvent,
    pub quit: KeyEvent,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tmux_integration: TmuxIntegration::FromEnv,
            tmux_display_popup_command: vec![
                "display-popup".to_string(),
                "-w".to_string(),
                "70%".to_string(),
                "-h".to_string(),
                "70%".to_string(),
            ],
            clipboard_file: ClipboardType::TemporaryFile,
            show_working_directory: true,
            simple_working_directory: false,
            normal_mode_text: String::from("NORMAL"),
            select_mode_text: String::from("SELECT"),
            search_mode_text: String::from("SEARCH"),
            insert_mode_text: String::from("INSERT"),
            shell_command_placeholder: String::from("%"),
            new_file: KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            new_directory: KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            duplicate: KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            remove: KeyEvent {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            copy: KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            paste: KeyEvent {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            incremental_search: KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            next_search_result: KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            prev_search_result: KeyEvent {
                code: KeyCode::Char('N'),
                modifiers: KeyModifiers::NONE,
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
            column_left: KeyEvent {
                code: KeyCode::Char('H'),
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
            column_right: KeyEvent {
                code: KeyCode::Char('L'),
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
            select_mode: KeyEvent {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            command_mode: KeyEvent {
                code: KeyCode::Char(':'),
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
        }
    }
}

impl Config {
    fn key_event_from_toml(key_event: &mut KeyEvent, toml: &Value) {
        if let Some(v) = toml.as_str() {
            let split = v.split_once("-");
            let (code, modifiers) = {
                let mut modifiers = KeyModifiers::NONE;
                (
                    {
                        if let Some((mod_str, code)) = split {
                            if mod_str.contains('c') || mod_str.contains('C') {
                                modifiers = modifiers.union(KeyModifiers::CONTROL);
                            }
                            if mod_str.contains('s') || mod_str.contains('S') {
                                modifiers = modifiers.union(KeyModifiers::SHIFT);
                            }
                            if mod_str.contains('a') || mod_str.contains('A') {
                                modifiers = modifiers.union(KeyModifiers::ALT);
                            }
                            Self::key_code_from_str(code)
                        } else {
                            Self::key_code_from_str(v)
                        }
                    },
                    modifiers,
                )
            };
            if let Some(code) = code {
                key_event.code = code;
                key_event.modifiers = modifiers;
            }
        }
    }

    fn key_code_from_str(s: &str) -> Option<KeyCode> {
        if s.chars().count() == 1 {
            return Some(KeyCode::Char(s.chars().nth(0).unwrap()));
        }

        if s.starts_with('F') && s.len() > 1 {
            if let Ok(n) = s[1..].parse() {
                return Some(KeyCode::F(n));
            }
        }

        if s == "Backspace" {
            Some(KeyCode::Backspace)
        } else if s == "Enter" {
            Some(KeyCode::Enter)
        } else if s == "Left" {
            Some(KeyCode::Left)
        } else if s == "Right" {
            Some(KeyCode::Right)
        } else if s == "Up" {
            Some(KeyCode::Up)
        } else if s == "Down" {
            Some(KeyCode::Down)
        } else if s == "Home" {
            Some(KeyCode::Home)
        } else if s == "End" {
            Some(KeyCode::End)
        } else if s == "PageUp" {
            Some(KeyCode::PageUp)
        } else if s == "PageDown" {
            Some(KeyCode::PageDown)
        } else if s == "Tab" {
            Some(KeyCode::Tab)
        } else if s == "BackTab" {
            Some(KeyCode::BackTab)
        } else if s == "Delete" {
            Some(KeyCode::Delete)
        } else if s == "Insert" {
            Some(KeyCode::Insert)
        } else if s == "Esc" {
            Some(KeyCode::Esc)
        } else {
            // TODO: Proper error reporting for malformed config
            // TODO: maybe support more keycodes?
            None
        }
    }

    pub fn from_toml(&mut self, toml: Value) {
        if let Some(v) = toml.get("tmux_integration") {
            if let Some(v) = v.as_str() {
                match v {
                    "env" => self.tmux_integration = TmuxIntegration::FromEnv,
                    "always" => self.tmux_integration = TmuxIntegration::Always,
                    "never" => self.tmux_integration = TmuxIntegration::Never,
                    _ => (), // TODO: Proper error reporting for malformed config
                }
            }
        }
        if let Some(v) = toml.get("tmux_display_popup_command") {
            if let Some(v) = v.as_str() {
                self.tmux_display_popup_command.clear();
                for v in v.split_whitespace() {
                    self.tmux_display_popup_command.push(v.to_string());
                }
            }
        }
        if let Some(v) = toml.get("clipboard_file") {
            if let Some(v) = v.as_str() {
                if v == "none" {
                    self.clipboard_file = ClipboardType::InMemory;
                } else {
                    self.clipboard_file = ClipboardType::File(v.to_string())
                }
            }
        }
        if let Some(v) = toml.get("normal_mode_text") {
            if let Some(v) = v.as_str() {
                self.normal_mode_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("select_mode_text") {
            if let Some(v) = v.as_str() {
                self.select_mode_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("search_mode_text") {
            if let Some(v) = v.as_str() {
                self.search_mode_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("insert_mode_text") {
            if let Some(v) = v.as_str() {
                self.insert_mode_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("shell_command_placeholder") {
            if let Some(v) = v.as_str() {
                self.shell_command_placeholder = v.to_string();
            }
        }
        if let Some(v) = toml.get("show_working_directory") {
            if let Some(v) = v.as_bool() {
                self.show_working_directory = v;
            }
        }
        if let Some(v) = toml.get("simple_working_directory") {
            if let Some(v) = v.as_bool() {
                self.simple_working_directory = v;
            }
        }
        if let Some(v) = toml.get("new_file") {
            Self::key_event_from_toml(&mut self.new_file, v)
        }
        if let Some(v) = toml.get("new_directory") {
            Self::key_event_from_toml(&mut self.new_directory, v);
        }
        if let Some(v) = toml.get("duplicate") {
            Self::key_event_from_toml(&mut self.duplicate, v);
        }
        if let Some(v) = toml.get("remove") {
            Self::key_event_from_toml(&mut self.remove, v);
        }
        if let Some(v) = toml.get("copy") {
            Self::key_event_from_toml(&mut self.copy, v);
        }
        if let Some(v) = toml.get("incremental_search") {
            Self::key_event_from_toml(&mut self.paste, v);
        }
        if let Some(v) = toml.get("next_search_result") {
            Self::key_event_from_toml(&mut self.paste, v);
        }
        if let Some(v) = toml.get("prev_search_result") {
            Self::key_event_from_toml(&mut self.paste, v);
        }
        if let Some(v) = toml.get("paste") {
            Self::key_event_from_toml(&mut self.paste, v);
        }
        if let Some(v) = toml.get("up") {
            Self::key_event_from_toml(&mut self.up, v);
        }
        if let Some(v) = toml.get("down") {
            Self::key_event_from_toml(&mut self.down, v);
        }
        if let Some(v) = toml.get("left") {
            Self::key_event_from_toml(&mut self.left, v);
        }
        if let Some(v) = toml.get("column_left") {
            Self::key_event_from_toml(&mut self.column_left, v);
        }
        if let Some(v) = toml.get("right") {
            Self::key_event_from_toml(&mut self.right, v);
        }
        if let Some(v) = toml.get("column_right") {
            Self::key_event_from_toml(&mut self.column_right, v);
        }
        if let Some(v) = toml.get("dir_walk") {
            Self::key_event_from_toml(&mut self.dir_walk, v);
        }
        if let Some(v) = toml.get("dir_up") {
            Self::key_event_from_toml(&mut self.dir_up, v);
        }
        if let Some(v) = toml.get("insert_mode") {
            Self::key_event_from_toml(&mut self.insert_mode, v);
        }
        if let Some(v) = toml.get("normal_mode") {
            Self::key_event_from_toml(&mut self.normal_mode, v);
        }
        if let Some(v) = toml.get("select_mode") {
            Self::key_event_from_toml(&mut self.select_mode, v);
        }
        if let Some(v) = toml.get("command_mode") {
            Self::key_event_from_toml(&mut self.command_mode, v);
        }
        if let Some(v) = toml.get("quit") {
            Self::key_event_from_toml(&mut self.quit, v);
        }
    }
}
