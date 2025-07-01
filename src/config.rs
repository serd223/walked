use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use toml::Value;

pub struct Config {
    pub normal_mode_text: String,
    pub insert_mode_text: String,
    pub show_entry_number: bool,
    pub show_entry_type: bool,
    pub show_working_directory: bool,
    pub simple_working_directory: bool,
    pub directory_text: String,
    pub file_text: String,
    pub symlink_text: String,
    pub other_text: String,
    pub new_file: KeyEvent,
    pub new_directory: KeyEvent,
    pub duplicate: KeyEvent,
    pub remove: KeyEvent,
    pub copy: KeyEvent,
    pub paste: KeyEvent,
    pub up: KeyEvent,
    pub select_up: KeyEvent,
    pub pane_up: KeyEvent,
    pub split_pane_up: KeyEvent,
    pub down: KeyEvent,
    pub select_down: KeyEvent,
    pub pane_down: KeyEvent,
    pub split_pane_down: KeyEvent,
    pub left: KeyEvent,
    pub pane_left: KeyEvent,
    pub split_pane_left: KeyEvent,
    pub right: KeyEvent,
    pub pane_right: KeyEvent,
    pub split_pane_right: KeyEvent,
    pub dir_walk: KeyEvent,
    pub dir_up: KeyEvent,
    pub insert_mode: KeyEvent,
    pub normal_mode: KeyEvent,
    pub close_active_pane: KeyEvent,
    pub quit: KeyEvent,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            show_entry_number: true,
            show_entry_type: true,
            show_working_directory: true,
            simple_working_directory: false,
            normal_mode_text: String::from("NORMAL"),
            insert_mode_text: String::from("INSERT"),
            directory_text: String::from("D"),
            file_text: String::from("F"),
            symlink_text: String::from("S"),
            other_text: String::from("O"),
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
            select_up: KeyEvent {
                code: KeyCode::Char('K'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            pane_up: KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            split_pane_up: KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            down: KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            select_down: KeyEvent {
                code: KeyCode::Char('J'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            pane_down: KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            split_pane_down: KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            left: KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            pane_left: KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            split_pane_left: KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            right: KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            pane_right: KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
            split_pane_right: KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::ALT,
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
            close_active_pane: KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
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
                            if mod_str.contains('a') || mod_str.contains('S') {
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
            // TODO: maybe support more keycodes?
            None
        }
    }

    pub fn from_toml(&mut self, toml: Value) {
        if let Some(v) = toml.get("normal_mode_text") {
            if let Some(v) = v.as_str() {
                self.normal_mode_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("insert_mode_text") {
            if let Some(v) = v.as_str() {
                self.insert_mode_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("show_entry_number") {
            if let Some(v) = v.as_bool() {
                self.show_entry_number = v;
            }
        }
        if let Some(v) = toml.get("show_entry_type") {
            if let Some(v) = v.as_bool() {
                self.show_entry_type = v;
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
        if let Some(v) = toml.get("directory_text") {
            if let Some(v) = v.as_str() {
                self.directory_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("file_text") {
            if let Some(v) = v.as_str() {
                self.file_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("symlink_text") {
            if let Some(v) = v.as_str() {
                self.symlink_text = v.to_string();
            }
        }
        if let Some(v) = toml.get("other_text") {
            if let Some(v) = v.as_str() {
                self.other_text = v.to_string();
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
        if let Some(v) = toml.get("paste") {
            Self::key_event_from_toml(&mut self.paste, v);
        }
        if let Some(v) = toml.get("up") {
            Self::key_event_from_toml(&mut self.up, v);
        }
        if let Some(v) = toml.get("select_up") {
            Self::key_event_from_toml(&mut self.select_up, v);
        }
        if let Some(v) = toml.get("pane_up") {
            Self::key_event_from_toml(&mut self.pane_up, v);
        }
        if let Some(v) = toml.get("split_pane_up") {
            Self::key_event_from_toml(&mut self.split_pane_up, v);
        }
        if let Some(v) = toml.get("down") {
            Self::key_event_from_toml(&mut self.down, v);
        }
        if let Some(v) = toml.get("select_down") {
            Self::key_event_from_toml(&mut self.select_down, v);
        }
        if let Some(v) = toml.get("pane_down") {
            Self::key_event_from_toml(&mut self.pane_down, v);
        }
        if let Some(v) = toml.get("split_pane_down") {
            Self::key_event_from_toml(&mut self.split_pane_down, v);
        }
        if let Some(v) = toml.get("left") {
            Self::key_event_from_toml(&mut self.left, v);
        }
        if let Some(v) = toml.get("pane_left") {
            Self::key_event_from_toml(&mut self.pane_left, v);
        }
        if let Some(v) = toml.get("split_pane_left") {
            Self::key_event_from_toml(&mut self.split_pane_left, v);
        }
        if let Some(v) = toml.get("right") {
            Self::key_event_from_toml(&mut self.right, v);
        }
        if let Some(v) = toml.get("pane_right") {
            Self::key_event_from_toml(&mut self.pane_right, v);
        }
        if let Some(v) = toml.get("split_pane_right") {
            Self::key_event_from_toml(&mut self.split_pane_right, v);
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
        if let Some(v) = toml.get("close_active_pane") {
            Self::key_event_from_toml(&mut self.close_active_pane, v);
        }
        if let Some(v) = toml.get("quit") {
            Self::key_event_from_toml(&mut self.quit, v);
        }
    }
}
