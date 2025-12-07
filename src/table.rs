use ratatui::{
    layout::{Constraint, Layout},
    widgets::Widget,
};
use wraptatui::{Pass, PassReturn};

use crate::{Column, TableView};

pub struct SelectedCell {
    row: usize,
    column: usize,
}

pub struct State<S> {
    view: Box<dyn TableView<State = S>>,
    columns: Vec<Column>,
    scroll_offset: usize,
    selected_cell: Option<SelectedCell>,
}

pub fn table<'a, S: 'static>(
    pass: Pass<'a>,
    state: &mut S,
    init: impl Fn() -> Box<dyn TableView<State = S>>,
) -> PassReturn<'a, State<S>> {
    pass.apply(
        state,
        |state| {
            let view = init();
            let columns = view.columns(state);

            State {
                view,
                columns,
                scroll_offset: 0,
                selected_cell: None,
            }
        },
        |view_state, state, area, buffer| {
            let layout = Layout::horizontal(state.columns.iter().map(|_| Constraint::Fill(1)));

            let areas = layout.split(area);

            let row_count = state.view.row_count(view_state);
            // TODO: handle overscroll when resizing and scroll offset being larger than row count
            let visible_rows = (row_count - state.scroll_offset).min(area.height as usize);

            for (column, area) in areas.iter().enumerate() {
                for i in 0..visible_rows {
                    let row = i + state.scroll_offset;
                    let mut area = *area;

                    area.height = 1;
                    area.y += i as u16;

                    match state.view.cell(view_state, row, column) {
                        crate::Cell::Text(text) => text.render(area, buffer),
                    }
                }
            }

            None
        },
        |view_state, state, event| false,
    )
}
