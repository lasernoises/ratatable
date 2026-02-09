use std::any::Any;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Margin},
    style::{Color, Style},
    widgets::{Block, Clear, List, ListState, Widget},
};
use wraptatui::{
    Focus, Focusable, Pass, PassReturn, WidgetState, draw, handle_key_event,
    widgets::textbox::{Input, textbox},
};

use crate::{Cell, CellUpdate, Column, TableView};

enum Editing {
    Text {
        input: Input,
        state: Box<dyn Any>,
    },
    Select {
        options: Vec<String>,
        state: ListState,
    },
}

pub struct SelectedCell {
    row: usize,
    column: usize,
    editing: Option<Editing>,
}

pub struct State<S> {
    view: Box<dyn TableView<State = S>>,
    columns: Vec<Column>,
    scroll_offset: usize,
    selected_cell: Option<SelectedCell>,
}

impl<S: 'static> WidgetState for State<S> {
    fn reset_focus(&mut self) -> Focusable {
        self.selected_cell = None;
        Focusable::Yes
    }
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
        |view_state, state, _focus, area, buffer| {
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

                        if let Some(editing) = &mut selected.editing {
                            cursor_position = match editing {
                                Editing::Text { input, state } => draw(
                                    &mut |p| textbox(p, input),
                                    state.downcast_mut().unwrap(),
                                    Focus::Focused,
                                    area,
                                    buffer,
                                ),
                                Editing::Select { .. } => None,
                            };
                            continue;
                        }
                    }

                    match state.view.cell(view_state, row, column) {
                        crate::Cell::Checkbox(checked) => {
                            if checked { "✓" } else { "" }.render(area, buffer)
                        }
                        crate::Cell::Text(text) => text.render(area, buffer),
                        crate::Cell::Select(text) => text.render(area, buffer),
                        crate::Cell::Link => "Open".render(area, buffer),
                    }
                }
            }

            match state.selected_cell {
                Some(SelectedCell {
                    editing:
                        Some(Editing::Select {
                            ref options,
                            ref mut state,
                        }),
                    ..
                }) => {
                    let area = area.inner(Margin::new(10, 10));

                    Clear.render(area, buffer);

                    ratatui::widgets::StatefulWidget::render(
                        List::new(options.iter().map(|s| s as &str))
                            .highlight_style(Style::new().bg(Color::Blue))
                            .block(Block::bordered()),
                        area,
                        buffer,
                        state,
                    );
                }
                _ => (),
            }

            cursor_position
        },
        |view_state, state, event| {
            if let Some(ref mut selected_cell) = state.selected_cell
                && let Some(ref mut editing) = selected_cell.editing
            {
                match editing {
                    Editing::Text {
                        input,
                        state: widget_state,
                    } => {
                        if !handle_key_event(
                            &mut |p| textbox(p, input),
                            widget_state.downcast_mut().unwrap(),
                            event,
                        ) {
                            match event.code {
                                KeyCode::Esc => {
                                    state.view.save_cell(
                                        view_state,
                                        selected_cell.row,
                                        selected_cell.column,
                                        match selected_cell.editing.take().unwrap() {
                                            Editing::Text { mut input, .. } => {
                                                CellUpdate::Text(input.value_and_reset())
                                            }
                                            _ => unreachable!(),
                                        },
                                    );

                                    true
                                }
                                _ => false,
                            }
                        } else {
                            false
                        }
                    }
                    Editing::Select {
                        state: list_state, ..
                    } => match event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            list_state.select_previous();
                            true
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            list_state.select_next();
                            true
                        }
                        KeyCode::Esc => {
                            selected_cell.editing = None;
                            true
                        }
                        KeyCode::Enter => {
                            if let Some(option) = list_state.selected() {
                                state.view.save_cell(
                                    view_state,
                                    selected_cell.row,
                                    selected_cell.column,
                                    CellUpdate::Select(option),
                                );
                                selected_cell.editing = None;
                                true
                            } else {
                                false
                            }
                        }
                        _ => false,
                    },
                }
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
                    KeyCode::Char('i') | KeyCode::Enter => {
                        if let Some(selected) = &mut state.selected_cell {
                            match state.view.cell(view_state, selected.row, selected.column) {
                                Cell::Checkbox(checked) => {
                                    state.view.save_cell(
                                        view_state,
                                        selected.row,
                                        selected.column,
                                        CellUpdate::Checkbox(!checked),
                                    );
                                }
                                Cell::Text(text) => {
                                    let mut input = Input::new(text.to_string());
                                    let state =
                                        Box::new(wraptatui::init(&mut |p| textbox(p, &mut input)));

                                    selected.editing = Some(Editing::Text { input, state });
                                }
                                Cell::Select(_) => {
                                    selected.editing = Some(Editing::Select {
                                        options: state.view.select_options(
                                            view_state,
                                            selected.row,
                                            selected.column,
                                        ),
                                        state: ListState::default(),
                                    });
                                }
                                Cell::Link => {
                                    state.view = state.view.open_cell(
                                        view_state,
                                        selected.row,
                                        selected.column,
                                    );
                                    state.columns = state.view.columns(view_state);
                                    state.selected_cell = None;
                                }
                            }
                        }
                        true
                    }
                    KeyCode::Backspace => {
                        if let Some(view) = state.view.back(view_state) {
                            state.view = view;
                            state.columns = state.view.columns(view_state);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
        },
    )
}
