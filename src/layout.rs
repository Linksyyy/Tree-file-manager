use ratatui::layout::Rect;

use crate::filesystem::NodeType;

const COLUMN_GAP: i32 = 7;

#[derive(Debug, Clone)]
pub struct ColumnEntry {
    pub name: String,
    pub node_type: NodeType,
    pub expanded: bool,
    pub selected: bool,
    pub search_match: SearchMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMatch {
    None,
    Current,
    Other,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub entries: Vec<ColumnEntry>,
    pub selected_row: usize,
    pub width: i32,
}

#[derive(Debug, Clone)]
pub struct ColumnLayout {
    pub columns: Vec<Column>,
    pub focus_column: usize,
    pub preview: Vec<ColumnEntry>,
    pub focus_x: i32,
    pub focus_width: i32,
    pub center_y: i32,
}

impl ColumnLayout {
    pub fn new(
        columns: Vec<Column>,
        focus_column: usize,
        preview: Vec<ColumnEntry>,
        area: Rect,
    ) -> Self {
        let focus = columns.get(focus_column);
        let focus_width = focus.map_or(1, |column| column.width);
        let center_x = area.width as i32 / 2;
        let focus_x = center_x - focus_width / 2;
        let center_y = area.height as i32 / 2;
        Self {
            columns,
            focus_column,
            preview,
            focus_x,
            focus_width,
            center_y,
        }
    }

    pub fn column_x(&self, index: usize) -> i32 {
        let mut x = self.focus_x;
        if index < self.focus_column {
            for current in (index..self.focus_column).rev() {
                x -= self.columns[current].width + COLUMN_GAP;
            }
        } else {
            for current in self.focus_column..index {
                x += self.columns[current].width + COLUMN_GAP;
            }
        }
        x
    }

    pub fn preview_x(&self) -> i32 {
        self.focus_x + self.focus_width + COLUMN_GAP
    }

    pub fn row_y(&self, column_index: usize, row: usize) -> i32 {
        self.center_y + (row as i32 - self.columns[column_index].selected_row as i32)
    }

    pub fn preview_row_y(&self, row: usize) -> i32 {
        let start = self.center_y - self.preview.len() as i32 / 2;
        start + row as i32
    }

    pub fn label(entry: &ColumnEntry) -> String {
        match entry.node_type {
            NodeType::Directory => {
                format!(
                    "◆ {}{}",
                    entry.name,
                    if entry.expanded { "/" } else { " ▸" }
                )
            }
            NodeType::File => format!("· {}", entry.name),
        }
    }
}

pub fn make_column(entries: Vec<ColumnEntry>, selected_row: usize) -> Column {
    let width = entries
        .iter()
        .map(|entry| ColumnLayout::label(entry).chars().count() as i32)
        .max()
        .unwrap_or(1);
    Column {
        entries,
        selected_row,
        width,
    }
}

pub fn connector_cells(layout: &ColumnLayout) -> Vec<(i32, i32, &'static str)> {
    let mut cells = std::collections::HashMap::<(i32, i32), u8>::new();

    // Keep the path already traversed visible. These connectors are derived
    // from the selected row in each pair of persistent columns.
    for column_index in 0..layout.focus_column {
        let left = &layout.columns[column_index];
        let right = &layout.columns[column_index + 1];
        let left_x = layout.column_x(column_index) + left.width;
        let right_x = layout.column_x(column_index + 1);
        let left_y = layout.row_y(column_index, left.selected_row);
        let right_y = layout.row_y(column_index + 1, right.selected_row);
        connect_path(&mut cells, left_x, left_y, right_x, right_y);
    }

    if !layout.preview.is_empty() {
        let left = &layout.columns[layout.focus_column];
        let left_x = layout.column_x(layout.focus_column) + left.width;
        let left_y = layout.center_y;
        let right_x = layout.preview_x();
        let first_y = layout.preview_row_y(0);
        let last_y = layout.preview_row_y(layout.preview.len() - 1);
        let branch_x = left_x + (right_x - left_x) / 2;

        connect_path(&mut cells, left_x, left_y, branch_x, left_y);
        for y in first_y.min(last_y)..=last_y.max(first_y) {
            if y != first_y {
                connect_path(&mut cells, branch_x, y - 1, branch_x, y);
            }
        }
        for row in 0..layout.preview.len() {
            let y = layout.preview_row_y(row);
            connect_path(&mut cells, branch_x, y, right_x, y);
        }
    }

    cells
        .into_iter()
        .map(|((x, y), directions)| (x, y, connector_symbol(directions)))
        .collect()
}

fn connect_path(
    cells: &mut std::collections::HashMap<(i32, i32), u8>,
    left_x: i32,
    left_y: i32,
    right_x: i32,
    right_y: i32,
) {
    let branch_x = left_x + (right_x - left_x) / 2;
    connect_horizontal(cells, left_x, branch_x, left_y);
    connect_vertical(cells, branch_x, left_y, right_y);
    connect_horizontal(cells, branch_x, right_x, right_y);
}

fn connect_horizontal(
    cells: &mut std::collections::HashMap<(i32, i32), u8>,
    start_x: i32,
    end_x: i32,
    y: i32,
) {
    let step = if end_x >= start_x { 1 } else { -1 };
    let mut x = start_x;
    loop {
        if x != end_x {
            add_direction(cells, (x, y), if step > 0 { 2 } else { 8 });
            add_direction(cells, (x + step, y), if step > 0 { 8 } else { 2 });
        }
        if x == end_x {
            break;
        }
        x += step;
    }
}

fn connect_vertical(
    cells: &mut std::collections::HashMap<(i32, i32), u8>,
    x: i32,
    start_y: i32,
    end_y: i32,
) {
    let step = if end_y >= start_y { 1 } else { -1 };
    let mut y = start_y;
    loop {
        if y != end_y {
            add_direction(cells, (x, y), if step > 0 { 4 } else { 1 });
            add_direction(cells, (x, y + step), if step > 0 { 1 } else { 4 });
        }
        if y == end_y {
            break;
        }
        y += step;
    }
}

fn add_direction(
    cells: &mut std::collections::HashMap<(i32, i32), u8>,
    position: (i32, i32),
    direction: u8,
) {
    *cells.entry(position).or_default() |= direction;
}

fn connector_symbol(directions: u8) -> &'static str {
    match directions {
        10 => "─",
        5 => "│",
        6 => "┌",
        12 => "┐",
        3 => "└",
        9 => "┘",
        11 => "┴",
        14 => "┬",
        7 => "├",
        13 => "┤",
        15 => "┼",
        _ => "·",
    }
}
