use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Up,
    Down,
    Left,
    Right,
    Open,
    Parent,
    Quit,
    Delete,
    Reload,
    Info,
    Search,
    NextSearch,
    PreviousSearch,
    Hidden,
    Confirm,
    Cancel,
    Backspace,
}

const ACTIONS: [(Action, &str); 17] = [
    (Action::Up, "up"),
    (Action::Down, "down"),
    (Action::Left, "left"),
    (Action::Right, "right"),
    (Action::Open, "open"),
    (Action::Parent, "parent"),
    (Action::Quit, "quit"),
    (Action::Delete, "delete"),
    (Action::Reload, "reload"),
    (Action::Info, "info"),
    (Action::Search, "search"),
    (Action::NextSearch, "next_search"),
    (Action::PreviousSearch, "previous_search"),
    (Action::Hidden, "hidden"),
    (Action::Confirm, "confirm"),
    (Action::Cancel, "cancel"),
    (Action::Backspace, "backspace"),
];

#[derive(Debug, Clone)]
pub(crate) struct KeyBindings {
    keys: Vec<(Action, KeyCode, String)>,
}

impl KeyBindings {
    pub fn load() -> io::Result<Self> {
        let path = config_path().ok_or_else(|| invalid_config("config.toml was not found"))?;
        let contents = fs::read_to_string(path)?;
        let mut keys = Vec::with_capacity(ACTIONS.len());

        for (action, name) in ACTIONS {
            let value = contents
                .lines()
                .filter_map(|line| line.split_once('='))
                .map(|(key, value)| (key.trim(), value))
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.trim().trim_matches('"').trim_matches('\''))
                .ok_or_else(|| invalid_config(&format!("missing key `{name}`")))?;
            keys.push((action, parse_key(value)?, value.to_string()));
        }

        for (index, (_, key, _)) in keys.iter().enumerate() {
            if keys[..index]
                .iter()
                .any(|(_, other_key, _)| other_key == key)
            {
                return Err(invalid_config(
                    "the same key cannot be assigned to multiple actions",
                ));
            }
        }

        Ok(Self { keys })
    }

    pub(crate) fn matches(&self, action: Action, key: KeyCode) -> bool {
        self.keys
            .iter()
            .any(|(configured_action, configured_key, _)| {
                *configured_action == action && *configured_key == key
            })
    }

    #[cfg(test)]
    pub(crate) fn key(&self, action: Action) -> KeyCode {
        self.keys
            .iter()
            .find(|(configured_action, _, _)| *configured_action == action)
            .map(|(_, key, _)| *key)
            .expect("all actions must be configured")
    }

    pub(crate) fn label(&self, action: Action) -> &str {
        self.keys
            .iter()
            .find(|(configured_action, _, _)| *configured_action == action)
            .map(|(_, _, label)| label.as_str())
            .unwrap_or("?")
    }
}

fn parse_key(value: &str) -> io::Result<KeyCode> {
    match value.to_ascii_lowercase().as_str() {
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "enter" => Ok(KeyCode::Enter),
        "esc" => Ok(KeyCode::Esc),
        "backspace" => Ok(KeyCode::Backspace),
        _ => {
            let mut chars = value.chars();
            match (chars.next(), chars.next()) {
                (Some(character), None) => Ok(KeyCode::Char(character)),
                _ => Err(invalid_config("keys must be one character or a named key")),
            }
        }
    }
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("TREE_VIEW_CONFIG") {
        return Some(PathBuf::from(path));
    }
    let local_path = PathBuf::from("config.toml");
    if local_path.exists() {
        return Some(local_path);
    }
    if let Ok(executable) = env::current_exe() {
        if let Some(project_path) = executable
            .parent()
            .and_then(|target| target.parent())
            .map(|project| project.join("config.toml"))
            && project_path.exists()
        {
            return Some(project_path);
        }
    }
    let directory = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(directory.join("tree-view/config.toml"))
}

fn invalid_config(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
