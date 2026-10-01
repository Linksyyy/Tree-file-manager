use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::filesystem::NodeType;
use crate::layout::{ColumnLayout, connector_cells};

pub fn render(app: &mut App, frame: &mut Frame) {
    let [tree_area, footer_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).areas(frame.area());
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(
                " ◆ TREE-VIEW ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", app.root.path.display()),
                Style::default().fg(Color::DarkGray),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(55, 70, 90)));
    let canvas = block.inner(tree_area);
    app.prepare_layout(canvas);
    frame.render_widget(block, tree_area);

    if let Some(layout) = &app.layout {
        draw_columns(app, frame, canvas, layout);
    }

    let status = app.status.as_deref().unwrap_or("Ready");
    let status_style = if app.status.is_some() {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let footer = vec![
        Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Color::Green)),
            Span::styled(status, status_style),
        ]),
        Line::from(vec![
            Span::styled(
                " j/k ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("move  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "l ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("enter  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "h ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("back  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "r ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("reload  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                ". ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("hidden  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "dd ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("delete  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "q ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("quit", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    frame.render_widget(Paragraph::new(footer), footer_area);
}

fn draw_columns(app: &App, frame: &mut Frame, area: Rect, layout: &ColumnLayout) {
    let buffer = frame.buffer_mut();
    let to_screen_x = |x: i32| area.x as i32 + x;
    let to_screen_y = |y: i32| area.y as i32 + y;

    for (x, y, symbol) in connector_cells(layout) {
        if x >= 0 && x < area.width as i32 && y >= 0 && y < area.height as i32 {
            buffer[(to_screen_x(x) as u16, to_screen_y(y) as u16)]
                .set_symbol(symbol)
                .set_style(Style::default().fg(Color::Rgb(75, 75, 90)));
        }
    }

    for (column_index, column) in layout.columns.iter().enumerate() {
        let x = layout.column_x(column_index);
        for (row, entry) in column.entries.iter().enumerate() {
            let y = layout.row_y(column_index, row);
            draw_entry(app, buffer, area, x, y, entry, row == column.selected_row);
        }
    }
    for (row, entry) in layout.preview.iter().enumerate() {
        draw_entry(
            app,
            buffer,
            area,
            layout.preview_x(),
            layout.preview_row_y(row),
            entry,
            false,
        );
    }
}

fn draw_entry(
    _app: &App,
    buffer: &mut ratatui::buffer::Buffer,
    area: Rect,
    x: i32,
    y: i32,
    entry: &crate::layout::ColumnEntry,
    selected: bool,
) {
    if y < 0 || y >= area.height as i32 || x >= area.width as i32 {
        return;
    }
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        match entry.node_type {
            NodeType::Directory => Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
            NodeType::File => Style::default().fg(Color::Gray),
        }
    };
    let label = ColumnLayout::label(entry);
    let visible_x = x.max(0);
    let skip = (visible_x - x) as usize;
    let clipped = label.chars().skip(skip).collect::<String>();
    buffer.set_string(
        (area.x as i32 + visible_x) as u16,
        (area.y as i32 + y) as u16,
        clipped,
        style,
    );
}
