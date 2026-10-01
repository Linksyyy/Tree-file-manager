use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;

use crate::filesystem::{FileNode, NodeType};
use crate::layout::{Column, ColumnEntry, ColumnLayout, make_column};

#[derive(Debug)]
pub struct App {
    pub root: FileNode,
    pub selected_path: PathBuf,
    pub layout: Option<ColumnLayout>,
    pub viewport: Rect,
    pub should_quit: bool,
    pub status: Option<String>,
    pub show_hidden: bool,
    delete_armed: bool,
    current_directory: PathBuf,
    directory_history: Vec<PathBuf>,
}

impl App {
    pub fn new(root_path: PathBuf) -> io::Result<Self> {
        let root = FileNode::from_directory(root_path)?;
        let root_directory = root.path.clone();
        let selected_path = root
            .children
            .iter()
            .find(|child| !child.name.starts_with('.'))
            .map(|child| child.path.clone())
            .unwrap_or_else(|| root.path.clone());
        let mut app = Self {
            root,
            selected_path,
            layout: None,
            viewport: Rect::default(),
            should_quit: false,
            status: None,
            show_hidden: false,
            delete_armed: false,
            current_directory: root_directory,
            directory_history: Vec::new(),
        };
        app.load_preview_children();
        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
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
                    })
                    .collect()
            })
            .unwrap_or_default();
        (columns, focus_column, preview)
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
            self.selected_path = self.root.path.clone();
            return;
        }
        let leaving_directory = self.current_directory.clone();
        self.current_directory = self
            .directory_history
            .pop()
            .or_else(|| leaving_directory.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| self.root.path.clone());
        self.selected_path = leaving_directory;
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
}
