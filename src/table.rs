use std::any::Any;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::Widget,
};
use wraptatui::{
    Pass, PassReturn, draw, handle_key_event,
    widgets::textbox::{Input, textbox},
};

use crate::{Cell, CellUpdate, Column, TableView};

pub struct SelectedCell {
    row: usize,
    column: usize,
    editing: Option<(CellUpdate, Box<dyn Any>)>,
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
            let visible_rows = (row_count - state.scroll_offset).min(area.height as usize - 1);

            let mut cursor_position = None;

            for (column, area) in areas.iter().enumerate() {
                let mut label_area = *area;
                label_area.height = 1;

                (&state.columns[column].label).render(label_area, buffer);

                for i in 0..visible_rows {
                    let row = i + state.scroll_offset;
                    let mut area = *area;

                    area.height = 1;
                    area.y += i as u16 + 1;

                    if let Some(selected) = &mut state.selected_cell
                        && selected.row == row
                        && selected.column == column
                    {
                        buffer.set_style(
                            area,
                            Style {
                                bg: Some(Color::Blue),
                                ..Default::default()
                            },
                        );

                        if let Some((update, state)) = &mut selected.editing {
                            cursor_position = match update {
                                CellUpdate::Text(input) => draw(
                                    &mut |p| textbox(p, input),
                                    state.downcast_mut().unwrap(),
                                    area,
                                    buffer,
                                ),
                            };
                            continue;
                        }
                    }

                    match state.view.cell(view_state, row, column) {
                        crate::Cell::Text(text) => text.render(area, buffer),
                        crate::Cell::Link => "Open".render(area, buffer),
                    }
                }
            }

            cursor_position
        },
        |view_state, state, event| {
            if let Some(ref mut selected_cell) = state.selected_cell
                && let Some((ref mut update, ref mut widget_state)) = selected_cell.editing
            {
                (match update {
                    CellUpdate::Text(input) => handle_key_event(
                        &mut |p| textbox(p, input),
                        widget_state.downcast_mut().unwrap(),
                        event,
                    ),
                }) || (match event.code {
                    KeyCode::Esc => {
                        state.view.save_cell(
                            view_state,
                            selected_cell.row,
                            selected_cell.column,
                            selected_cell.editing.take().unwrap().0,
                        );

                        true
                    }
                    _ => false,
                })
            } else {
                let row_count = state.view.row_count(view_state);

                match event.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if let Some(selected) = &mut state.selected_cell {
                            selected.column = selected.column.saturating_sub(1);
                        } else {
                            state.selected_cell = Some(SelectedCell {
                                row: 0,
                                column: state.columns.len() - 1,
                                editing: None,
                            });
                        }
                        true
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if let Some(selected) = &mut state.selected_cell {
                            selected.column =
                                (selected.column + 1).min(state.columns.len().saturating_sub(1));
                        } else {
                            state.selected_cell = Some(SelectedCell {
                                row: 0,
                                column: 0,
                                editing: None,
                            });
                        }
                        true
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(selected) = &mut state.selected_cell {
                            selected.row = selected.row.saturating_sub(1);
                        } else {
                            state.selected_cell = Some(SelectedCell {
                                row: row_count - 1,
                                column: 0,
                                editing: None,
                            });
                        }
                        true
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(selected) = &mut state.selected_cell {
                            selected.row = (selected.row + 1).min(row_count.saturating_sub(1));
                        } else {
                            state.selected_cell = Some(SelectedCell {
                                row: 0,
                                column: 0,
                                editing: None,
                            });
                        }
                        true
                    }
                    KeyCode::Char('n') => {
                        state.view.new_row(view_state);
                        true
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = &mut state.selected_cell {
                            selected.editing = Some(
                                match state.view.cell(view_state, selected.row, selected.column) {
                                    Cell::Text(text) => {
                                        let mut input = Input::new(text.to_string());
                                        let state = Box::new(wraptatui::init(&mut |p| {
                                            textbox(p, &mut input)
                                        }));

                                        (CellUpdate::Text(input), state)
                                    }
                                    _ => todo!(),
                                },
                            )
                        }
                        true
                    }
                    _ => false,
                }
            }
        },
    )
}
