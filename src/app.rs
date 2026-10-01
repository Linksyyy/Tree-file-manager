use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;

use crate::filesystem::{FileNode, NodeType};
use crate::layout::{Column, ColumnEntry, ColumnLayout, SearchMatch, make_column};

#[derive(Debug)]
pub struct App {
    pub root: FileNode,
    pub selected_path: PathBuf,
    pub layout: Option<ColumnLayout>,
    pub viewport: Rect,
    pub should_quit: bool,
    pub status: Option<String>,
    pub info_modal: Option<Vec<(String, String)>>,
    pub search_mode: bool,
    pub search_query: String,
    pub show_hidden: bool,
    delete_armed: bool,
    search_matches: Vec<PathBuf>,
    search_index: usize,
    current_directory: PathBuf,
    directory_history: Vec<PathBuf>,
}

impl App {
    pub fn new(root_path: PathBuf) -> io::Result<Self> {
        Self::new_with_context(root_path.clone(), root_path)
    }

    fn new_with_context(root_path: PathBuf, current_directory: PathBuf) -> io::Result<Self> {
        let mut root = FileNode::from_directory(root_path)?;
        if current_directory != root.path {
            let Some(current_node) = root.find_mut(&current_directory) else {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Could not load {}", current_directory.display()),
                ));
            };
            current_node.expand()?;
        }
        let selected_path = root
            .find(&current_directory)
            .map(|directory| {
                directory
                    .children
                    .iter()
                    .find(|child| !child.name.starts_with('.'))
                    .map(|child| child.path.clone())
                    .unwrap_or_else(|| current_directory.clone())
            })
            .unwrap_or_else(|| current_directory.clone());
        let mut app = Self {
            root,
            selected_path,
            layout: None,
            viewport: Rect::default(),
            should_quit: false,
            status: None,
            info_modal: None,
            search_mode: false,
            search_query: String::new(),
            show_hidden: false,
            delete_armed: false,
            search_matches: Vec::new(),
            search_index: 0,
            current_directory,
            directory_history: Vec::new(),
        };
        app.load_preview_children();
        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.info_modal.is_some() {
            self.info_modal = None;
            return;
        }
        if self.search_mode {
            self.handle_search_key(key);
            return;
        }
        self.status = None;
        if !matches!(key.code, KeyCode::Char('d')) {
            self.delete_armed = false;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') => self.move_in_column(1),
            KeyCode::Char('k') => self.move_in_column(-1),
            KeyCode::Char('l') => self.open_selected(),
            KeyCode::Char('o') | KeyCode::Enter => self.open_selected(),
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Backspace => self.select_parent(),
            KeyCode::Char('d') => self.delete_selected(),
            KeyCode::Char('r') => self.reload_current_directory(),
            KeyCode::Char('i') => self.show_selected_info(),
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query.clear();
                self.refresh_search();
            }
            KeyCode::Char('n') => self.next_search_match(1),
            KeyCode::Char('N') => self.next_search_match(-1),
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                self.ensure_selection_visible();
                self.load_preview_children();
            }
            _ => {}
        }
    }

    pub fn prepare_layout(&mut self, area: Rect) {
        self.viewport = area;
        let (columns, focus_column, preview) = self.build_columns();
        self.layout = Some(ColumnLayout::new(columns, focus_column, preview, area));
    }

    pub fn selected_node(&self) -> Option<&FileNode> {
        self.root.find(&self.selected_path)
    }

    pub fn exit_path(&self) -> &Path {
        &self.current_directory
    }

    fn build_columns(&self) -> (Vec<Column>, usize, Vec<ColumnEntry>) {
        let mut chain = vec![self.root.path.clone()];
        let mut ancestors = Vec::new();
        let mut current = Some(self.current_directory.clone());
        while let Some(path) = current {
            if path == self.root.path {
                break;
            }
            ancestors.push(path.clone());
            current = path.parent().map(Path::to_path_buf);
        }
        ancestors.reverse();
        chain.extend(ancestors);

        let mut columns = Vec::new();
        for (column_index, directory_path) in chain.iter().enumerate() {
            let Some(directory) = self.root.find(directory_path) else {
                continue;
            };
            if self.selected_path == self.root.path && column_index == 0 {
                columns.push(make_column(
                    vec![ColumnEntry {
                        name: directory.name.clone(),
                        node_type: directory.node_type,
                        expanded: directory.expanded,
                        selected: true,
                        search_match: SearchMatch::None,
                    }],
                    0,
                ));
                continue;
            }
            let selected_child_path = chain.get(column_index + 1).cloned().or_else(|| {
                (directory_path == &self.current_directory).then(|| self.selected_path.clone())
            });
            let entries = directory
                .children
                .iter()
                .filter(|child| self.show_hidden || !child.name.starts_with('.'))
                .map(|child| ColumnEntry {
                    name: child.name.clone(),
                    node_type: child.node_type,
                    expanded: child.expanded,
                    selected: selected_child_path.as_ref() == Some(&child.path),
                    search_match: self.search_match_for(&child.path),
                })
                .collect::<Vec<_>>();
            let selected_row = entries.iter().position(|entry| entry.selected).unwrap_or(0);
            columns.push(make_column(entries, selected_row));
        }

        if columns.is_empty() {
            columns.push(make_column(Vec::new(), 0));
        }
        let focus_column = columns.len() - 1;
        let preview = self
            .selected_node()
            .filter(|node| node.node_type == NodeType::Directory)
            .map(|node| {
                node.children
                    .iter()
                    .filter(|child| self.show_hidden || !child.name.starts_with('.'))
                    .map(|child| ColumnEntry {
                        name: child.name.clone(),
                        node_type: child.node_type,
                        expanded: child.expanded,
                        selected: false,
                        search_match: SearchMatch::None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        (columns, focus_column, preview)
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.search_mode = false,
            KeyCode::Backspace => {
                self.search_query.pop();
                self.refresh_search();
            }
            KeyCode::Char(character) => {
                self.search_query.push(character);
                self.refresh_search();
            }
            _ => {}
        }
    }

    fn refresh_search(&mut self) {
        self.search_matches = self
            .root
            .find(&self.current_directory)
            .map(|directory| {
                let query = self.search_query.to_lowercase();
                directory
                    .children
                    .iter()
                    .filter(|child| {
                        !query.is_empty()
                            && (self.show_hidden || !child.name.starts_with('.'))
                            && child.name.to_lowercase().contains(&query)
                    })
                    .map(|child| child.path.clone())
                    .collect()
            })
            .unwrap_or_default();
        self.search_index = 0;
        if let Some(path) = self.search_matches.first() {
            self.selected_path = path.clone();
            self.load_preview_children();
        }
    }

    fn clear_search(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_index = 0;
    }

    fn next_search_match(&mut self, direction: i32) {
        if self.search_matches.is_empty() {
            return;
        }
        let count = self.search_matches.len();
        self.search_index = if direction > 0 {
            (self.search_index + 1) % count
        } else {
            (self.search_index + count - 1) % count
        };
        self.selected_path = self.search_matches[self.search_index].clone();
        self.load_preview_children();
    }

    fn search_match_for(&self, path: &Path) -> SearchMatch {
        if self.search_matches.is_empty()
            || !self
                .search_matches
                .iter()
                .any(|match_path| match_path == path)
        {
            SearchMatch::None
        } else if self.selected_path == path {
            SearchMatch::Current
        } else {
            SearchMatch::Other
        }
    }

    fn move_in_column(&mut self, direction: i32) {
        let Some(directory) = self.root.find(&self.current_directory) else {
            return;
        };
        let visible_children = directory
            .children
            .iter()
            .filter(|child| self.show_hidden || !child.name.starts_with('.'))
            .collect::<Vec<_>>();
        if visible_children.is_empty() {
            return;
        }
        let Some(index) = visible_children
            .iter()
            .position(|child| child.path == self.selected_path)
        else {
            self.selected_path = visible_children[0].path.clone();
            self.load_preview_children();
            return;
        };
        let next = if direction < 0 {
            index.saturating_sub(1)
        } else {
            (index + 1).min(visible_children.len().saturating_sub(1))
        };
        if let Some(child) = visible_children.get(next) {
            self.selected_path = child.path.clone();
            self.load_preview_children();
        }
    }

    fn open_directory(&mut self) {
        let Some(path) = self.selected_node().map(|node| node.path.clone()) else {
            return;
        };
        let Some(node) = self.root.find_mut(&path) else {
            return;
        };
        if node.node_type != NodeType::Directory {
            return;
        }
        if let Err(error) = node.expand() {
            self.status = Some(format!("Could not enter directory: {error}"));
            return;
        }
        let child_path = node
            .children
            .iter()
            .find(|child| self.show_hidden || !child.name.starts_with('.'))
            .map(|child| child.path.clone());
        self.directory_history.push(self.current_directory.clone());
        self.current_directory = path;
        self.selected_path = child_path.unwrap_or_else(|| self.current_directory.clone());
        self.clear_search();
        self.load_preview_children();
    }

    fn open_selected(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };

        if node.node_type == NodeType::Directory {
            self.open_directory();
            return;
        }

        let path = node.path.clone();
        match Command::new("xdg-open").arg(&path).spawn() {
            Ok(_) => {
                self.status = Some(format!("Opening {}", path.display()));
            }
            Err(error) => {
                self.status = Some(format!("Could not open file: {error}"));
            }
        }
    }

    fn select_parent(&mut self) {
        if self.current_directory == self.root.path {
            let current_directory = self.current_directory.clone();
            let Some(parent_directory) = current_directory.parent().map(Path::to_path_buf) else {
                self.selected_path = self.root.path.clone();
                return;
            };

            match FileNode::from_directory(parent_directory.clone()) {
                Ok(root) => {
                    self.root = root;
                    self.current_directory = parent_directory;
                    self.selected_path = current_directory;
                    self.directory_history.clear();
                    self.clear_search();
                    self.load_preview_children();
                    self.status = Some(format!("Returned to {}", self.current_directory.display()));
                }
                Err(error) => {
                    self.status = Some(format!("Could not read parent directory: {error}"));
                }
            }
            return;
        }
        let leaving_directory = self.current_directory.clone();
        self.current_directory = self
            .directory_history
            .pop()
            .or_else(|| leaving_directory.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| self.root.path.clone());
        self.selected_path = leaving_directory;
        self.clear_search();
        self.load_preview_children();
        self.status = Some(format!("Returned to {}", self.selected_path.display()));
    }

    fn load_preview_children(&mut self) {
        let path = self.selected_path.clone();
        let Some(node) = self.root.find_mut(&path) else {
            return;
        };
        if node.node_type == NodeType::Directory
            && let Err(error) = node.load_children()
        {
            self.status = Some(format!("Could not load preview: {error}"));
        }
    }

    fn show_selected_info(&mut self) {
        let Some(node) = self.selected_node() else {
            self.status = Some("No item is selected".to_string());
            return;
        };

        let metadata = match std::fs::metadata(&node.path) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.status = Some(format!("Could not read file information: {error}"));
                return;
            }
        };
        let item_type = if node.node_type == NodeType::Directory {
            "directory"
        } else {
            "file"
        };
        let size = if node.node_type == NodeType::Directory {
            "n/a".to_string()
        } else {
            Self::format_bytes(metadata.len())
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| format!("{}s since epoch", duration.as_secs()))
            .unwrap_or_else(|| "unavailable".to_string());

        self.info_modal = Some(vec![
            ("Name".to_string(), node.name.clone()),
            ("Path".to_string(), node.path.display().to_string()),
            ("Type".to_string(), item_type.to_string()),
            ("Size".to_string(), size),
            (
                "Read-only".to_string(),
                metadata.permissions().readonly().to_string(),
            ),
            ("Modified".to_string(), modified),
        ]);
    }

    fn format_bytes(bytes: u64) -> String {
        const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
        let mut value = bytes as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        if unit == 0 {
            format!("{bytes} {}", UNITS[unit])
        } else {
            format!("{value:.1} {}", UNITS[unit])
        }
    }

    fn reload_current_directory(&mut self) {
        let directory_path = self
            .selected_node()
            .filter(|node| node.node_type == NodeType::Directory)
            .map(|node| node.path.clone())
            .or_else(|| self.selected_path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| self.root.path.clone());

        let reload_result = self
            .root
            .find_mut(&directory_path)
            .map(|node| node.reload_children());
        match reload_result {
            Some(Ok(())) => {
                self.ensure_selection_visible();
                self.load_preview_children();
                self.status = Some("Directory reloaded".to_string());
            }
            Some(Err(error)) => {
                self.status = Some(format!("Could not reload directory: {error}"));
            }
            None => {}
        }
    }

    fn ensure_selection_visible(&mut self) {
        let selected_is_visible = self
            .root
            .find(&self.selected_path)
            .is_some_and(|node| self.show_hidden || !node.name.starts_with('.'));
        if selected_is_visible {
            return;
        }

        let parent_path = self
            .selected_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.path.clone());
        if let Some(parent) = self.root.find(&parent_path) {
            self.selected_path = parent
                .children
                .iter()
                .find(|child| self.show_hidden || !child.name.starts_with('.'))
                .map(|child| child.path.clone())
                .unwrap_or(parent_path);
        } else {
            self.selected_path = self.root.path.clone();
        }
    }

    fn delete_selected(&mut self) {
        if self.selected_path == self.root.path {
            self.status = Some("The root directory cannot be deleted".to_string());
            return;
        }

        if !self.delete_armed {
            self.delete_armed = true;
            self.status = Some("Press d again to confirm deletion".to_string());
            return;
        }

        let selected_path = self.selected_path.clone();
        let Some(node) = self.root.find(&selected_path) else {
            self.delete_armed = false;
            return;
        };
        let Some(parent_path) = selected_path.parent().map(Path::to_path_buf) else {
            self.delete_armed = false;
            return;
        };
        let Some(parent) = self.root.find(&parent_path) else {
            self.delete_armed = false;
            return;
        };
        let selected_index = parent
            .children
            .iter()
            .position(|child| child.path == selected_path)
            .unwrap_or(0);

        if let Err(error) = node.delete_from_disk() {
            self.delete_armed = false;
            self.status = Some(format!("Could not delete item: {error}"));
            return;
        }
        if self.root.remove_child(&selected_path).is_none() {
            self.delete_armed = false;
            self.status = Some("The item was deleted from disk but not from the tree".to_string());
            return;
        }

        self.delete_armed = false;
        if let Some(parent) = self.root.find(&parent_path) {
            if let Some(next) = parent
                .children
                .get(selected_index.min(parent.children.len().saturating_sub(1)))
            {
                self.selected_path = next.path.clone();
            } else {
                self.selected_path = parent_path;
            }
        }
        self.load_preview_children();
        self.status = Some("Item deleted".to_string());
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    use super::App;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new_with_kind(code, KeyModifiers::NONE, KeyEventKind::Press)
    }

    #[test]
    fn h_returns_through_each_visited_directory() {
        let root = std::env::temp_dir().join(format!(
            "treenav-navigation-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        let nested = root.join("a").join("b").join("c");
        fs::create_dir_all(&nested).expect("test tree must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        app.handle_key(key(KeyCode::Char('l')));
        app.handle_key(key(KeyCode::Char('l')));
        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.selected_path, nested);

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.selected_path, nested);
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.selected_path, root.join("a").join("b"));
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.selected_path, root.join("a"));

        fs::remove_dir_all(root).expect("test tree must be removed");
    }

    #[test]
    fn j_and_k_move_before_and_after_entering_directories() {
        let root = std::env::temp_dir().join(format!(
            "treenav-siblings-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("a").join("a1")).expect("test tree must be created");
        fs::create_dir_all(root.join("a").join("a2")).expect("test tree must be created");
        fs::create_dir_all(root.join("b")).expect("test tree must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        assert_eq!(app.selected_path, root.join("a"));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected_path, root.join("b"));
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.selected_path, root.join("a"));

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.selected_path, root.join("a").join("a1"));
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected_path, root.join("a").join("a2"));
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.selected_path, root.join("a").join("a1"));
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.selected_path, root.join("a"));
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected_path, root.join("b"));

        fs::remove_dir_all(root).expect("test tree must be removed");
    }

    #[test]
    fn l_and_enter_open_files_with_the_same_behavior() {
        let root = std::env::temp_dir().join(format!(
            "treenav-open-file-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("test tree must be created");
        let file = root.join("file.txt");
        fs::write(&file, "content").expect("test file must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        assert_eq!(app.selected_path, file);
        let expected_status = format!("Opening {}", file.display());

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.status.as_deref(), Some(expected_status.as_str()));

        app.status = None;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.status.as_deref(), Some(expected_status.as_str()));

        fs::remove_dir_all(root).expect("test tree must be removed");
    }

    #[test]
    fn q_does_not_enter_selected_directory_without_l() {
        let root = std::env::temp_dir().join(format!(
            "treenav-quit-directory-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("repo")).expect("test tree must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        assert_eq!(app.selected_path, root.join("repo"));
        assert_eq!(app.exit_path(), root);

        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
        assert_eq!(app.exit_path(), root);

        fs::remove_dir_all(root).expect("test tree must be removed");
    }

    #[test]
    fn h_lazily_loads_each_parent_directory() {
        let parent = std::env::temp_dir().join(format!(
            "treenav-parent-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        let launch_directory = parent.join("project");
        fs::create_dir_all(launch_directory.join("src")).expect("test tree must be created");

        let mut app = App::new(launch_directory.clone()).expect("app must load launch tree");
        assert_eq!(app.exit_path(), launch_directory);
        assert_eq!(app.selected_path, launch_directory.join("src"));
        assert_eq!(app.root.path, launch_directory);

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.exit_path(), parent);
        assert_eq!(app.selected_path, launch_directory);
        assert_eq!(app.root.path, parent);

        let (columns, _, _) = app.build_columns();
        assert_eq!(columns.len(), 1);
        assert!(
            columns[0]
                .entries
                .iter()
                .any(|entry| entry.name == "project")
        );

        let grandparent = parent.parent().expect("temporary directory has a parent");
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.exit_path(), grandparent);
        assert_eq!(app.selected_path, parent);
        assert_eq!(app.root.path, grandparent);

        fs::remove_dir_all(parent).expect("test tree must be removed");
    }

    #[test]
    fn l_then_q_exits_in_entered_directory() {
        let root = std::env::temp_dir().join(format!(
            "treenav-enter-quit-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("repo")).expect("test tree must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.exit_path(), root.join("repo"));

        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
        assert_eq!(app.exit_path(), root.join("repo"));

        fs::remove_dir_all(root).expect("test tree must be removed");
    }

    #[test]
    fn i_shows_information_for_the_selected_file() {
        let root = std::env::temp_dir().join(format!(
            "treenav-file-info-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("test tree must be created");
        let file = root.join("notes.txt");
        fs::write(&file, "hello").expect("test file must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        app.handle_key(key(KeyCode::Char('i')));
        let info = app.info_modal.as_ref().expect("info modal must be shown");
        assert!(
            info.iter()
                .any(|(label, value)| label == "Name" && value == "notes.txt")
        );
        assert!(
            info.iter()
                .any(|(label, value)| label == "Type" && value == "file")
        );
        assert!(
            info.iter()
                .any(|(label, value)| label == "Size" && value == "5 B")
        );

        app.handle_key(key(KeyCode::Char('j')));
        assert!(app.info_modal.is_none());

        fs::remove_dir_all(root).expect("test tree must be removed");
    }

    #[test]
    fn slash_searches_incrementally_and_n_cycles_matches() {
        let root = std::env::temp_dir().join(format!(
            "treenav-search-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("test tree must be created");
        fs::write(root.join("apple.txt"), "a").expect("test file must be created");
        fs::write(root.join("apricot.txt"), "b").expect("test file must be created");
        fs::write(root.join("banana.txt"), "c").expect("test file must be created");

        let mut app = App::new(root.clone()).expect("app must load test tree");
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('p')));
        assert!(app.search_mode);
        assert_eq!(app.selected_path, root.join("apple.txt"));

        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.selected_path, root.join("apricot.txt"));
        app.handle_key(key(KeyCode::Char('N')));
        assert_eq!(app.selected_path, root.join("apple.txt"));

        fs::remove_dir_all(root).expect("test tree must be removed");
    }
}
