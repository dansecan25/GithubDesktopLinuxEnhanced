//! Keyboard shortcuts, matching GitHub Desktop's where it has one. `COMMAND` is Ctrl on Linux.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

const CTRL_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

pub const COMMIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter);
pub const REFRESH: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F5);
pub const SHOW_CHANGES: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num1);
pub const SHOW_HISTORY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num2);
pub const PUSH: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::P);
pub const PULL: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::P);
pub const FETCH: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::T);
pub const NEW_BRANCH: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::N);
pub const NEW_REPOSITORY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
pub const ADD_REPOSITORY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
pub const CLONE_REPOSITORY: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::O);
