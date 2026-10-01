use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Directory,
    File,
}

#[derive(Debug)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub node_type: NodeType,
    pub expanded: bool,
    pub children: Vec<FileNode>,
    pub children_loaded: bool,
    pub load_error: Option<String>,
}

impl FileNode {
    pub fn from_directory(path: PathBuf) -> io::Result<Self> {
        let children = read_children(&path)?;
        Ok(Self {
            name: display_name(&path),
            path,
            node_type: NodeType::Directory,
            expanded: true,
            children,
            children_loaded: true,
            load_error: None,
        })
    }

    fn from_entry(entry: DirEntry) -> io::Result<Self> {
        let path = entry.path();
        let node_type = if entry.file_type()?.is_dir() {
            NodeType::Directory
        } else {
            NodeType::File
        };
        Ok(Self {
            name: display_name(&path),
            path,
            node_type,
            expanded: false,
            children: Vec::new(),
            children_loaded: node_type == NodeType::File,
            load_error: None,
        })
    }

    pub fn expand(&mut self) -> io::Result<()> {
        if self.node_type != NodeType::Directory {
            return Ok(());
        }

        self.load_children()?;
        self.expanded = true;
        Ok(())
    }

    pub fn load_children(&mut self) -> io::Result<()> {
        if self.node_type != NodeType::Directory {
            return Ok(());
        }

        if !self.children_loaded {
            match read_children(&self.path) {
                Ok(children) => {
                    self.children = children;
                    self.children_loaded = true;
                    self.load_error = None;
                }
                Err(error) => {
                    self.load_error = Some(error.to_string());
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    pub fn reload_children(&mut self) -> io::Result<()> {
        if self.node_type != NodeType::Directory {
            return Ok(());
        }
        self.children = read_children(&self.path)?;
        self.children_loaded = true;
        self.load_error = None;
        Ok(())
    }

    pub fn find_mut(&mut self, path: &Path) -> Option<&mut FileNode> {
        if self.path == path {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(path))
    }

    pub fn find(&self, path: &Path) -> Option<&FileNode> {
        if self.path == path {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(path))
    }

    pub fn remove_child(&mut self, path: &Path) -> Option<FileNode> {
        if let Some(index) = self.children.iter().position(|child| child.path == path) {
            return Some(self.children.remove(index));
        }
        self.children
            .iter_mut()
            .find_map(|child| child.remove_child(path))
    }

    pub fn delete_from_disk(&self) -> io::Result<()> {
        match self.node_type {
            NodeType::Directory => fs::remove_dir_all(&self.path),
            NodeType::File => fs::remove_file(&self.path),
        }
    }
}

fn read_children(path: &Path) -> io::Result<Vec<FileNode>> {
    let mut children: Vec<FileNode> = fs::read_dir(path)?
        .map(|entry_result| entry_result.and_then(FileNode::from_entry))
        .collect::<io::Result<Vec<FileNode>>>()?;

    children.sort_by_key(|child| {
        (
            child.node_type != NodeType::Directory,
            child.name.to_lowercase(),
        )
    });
    Ok(children)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{FileNode, NodeType};

    #[test]
    fn visible_nodes_follow_expansion_state() {
        let mut root = FileNode {
            path: "root".into(),
            name: "root".into(),
            node_type: NodeType::Directory,
            expanded: true,
            children_loaded: true,
            children: vec![FileNode {
                path: "root/src".into(),
                name: "src".into(),
                node_type: NodeType::Directory,
                expanded: false,
                children_loaded: true,
                children: vec![FileNode {
                    path: "root/src/main.rs".into(),
                    name: "main.rs".into(),
                    node_type: NodeType::File,
                    expanded: false,
                    children_loaded: true,
                    children: vec![],
                    load_error: None,
                }],
                load_error: None,
            }],
            load_error: None,
        };

        assert_eq!(root.children.len(), 1);
        assert!(!root.children[0].expanded);
        root.children[0].expanded = true;
        assert_eq!(root.children[0].children.len(), 1);
    }
}
