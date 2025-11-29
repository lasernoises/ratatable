use crossterm::event::KeyCode;
use rat_ftable::{Table, TableData, TableState, TableStyle, selection::CellSelection};
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::Widget,
};
use wraptatui::{
    ratatui_stateful_widget, run,
    widgets::{state::state, with_key_handler::with_key_handler},
};

struct Data {
    data: Vec<[&'static str; 4]>,
}

impl<'a> TableData<'a> for &'a Data {
    fn header(&self) -> Option<rat_ftable::textdata::Row<'a>> {
        Some(rat_ftable::textdata::Row::new(["A", "B", "C", "D"]))
    }

    fn rows(&self) -> usize {
        self.data.len()
    }

    fn render_cell(
        &self,
        ctx: &rat_ftable::TableContext,
        column: usize,
        row: usize,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        self.data[row][column].render(area, buf);
    }
}

impl Default for Data {
    fn default() -> Self {
        Self {
            data: vec![
                ["a", "b", "c", "d"],
                ["e", "f", "g", "h"],
                ["i", "j", "k", "l"],
            ],
        }
    }
}

fn main() {
    run(&mut |p| {
        state(p, &mut |p,
                       (data, state): &mut (
            Data,
            TableState<CellSelection>,
        )| {
            with_key_handler(
                p,
                state,
                |state, event| match event.code {
                    KeyCode::Left | KeyCode::Char('h') => state.selection.move_left(1, 3),
                    KeyCode::Right | KeyCode::Char('l') => state.selection.move_right(1, 3),
                    KeyCode::Up | KeyCode::Char('k') => state.selection.move_up(1, 2),
                    KeyCode::Down | KeyCode::Char('j') => state.selection.move_down(1, 2),
                    _ => false,
                },
                |p, state| {
                    ratatui_stateful_widget(
                        p,
                        Table::new()
                            .data(&*data)
                            .widths([
                                Constraint::Fill(1),
                                Constraint::Fill(1),
                                Constraint::Fill(1),
                                Constraint::Fill(1),
                            ])
                            .styles(TableStyle {
                                show_cell_focus: true,
                                select_cell: Some(Style {
                                    bg: Some(Color::White),
                                    ..Default::default()
                                }),
                                ..TableStyle::default()
                            }),
                        state,
                    )
                },
            )
        })
    })
    .unwrap();
}
