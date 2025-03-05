use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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
    pub down: KeyEvent,
    pub left: KeyEvent,
    pub right: KeyEvent,
    pub dir_walk: KeyEvent,
    pub dir_up: KeyEvent,
    pub insert_mode: KeyEvent,
    pub normal_mode: KeyEvent,
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
                code: KeyCode::Char('a'),
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
        }
    }
}
