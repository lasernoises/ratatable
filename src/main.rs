use std::{any::Any, cell::RefCell};

use crossterm::event::KeyCode;
use rat_ftable::{Table, TableData, TableState, TableStyle, selection::CellSelection};
use ratatui::{
    layout::{Constraint, Position},
    style::{Color, Style},
    widgets::Widget,
};
use wraptatui::{
    Pass, PassReturn, draw, handle_key_event, init, ratatui_stateful_widget, run,
    widgets::{
        state::state,
        textbox::{Input, textbox},
        with_key_handler::with_key_handler,
    },
};

pub enum Cell<'a> {
    Text(&'a str),
}

pub enum CellUpdate {
    Text(Input),
}

#[derive(Clone)]
pub struct Column {
    pub label: String,
}

pub trait TableView {
    type State;

    /// Called once when the view is opened.
    fn columns(&self, state: &Self::State) -> Vec<Column>;

    fn row_count(&self, state: &Self::State) -> usize;

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> Cell<'a>;

    fn save_cell(&mut self, state: &mut Self::State, row: usize, column: usize, value: CellUpdate);

    fn new_row(&mut self, state: &mut Self::State);

    fn open_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State>>;
}

struct TestData {
    data: Vec<[&'static str; 4]>,
}

impl Default for TestData {
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

impl TableView for TestData {
    type State = ();

    fn columns(&self, _: &Self::State) -> Vec<Column> {
        vec![
            Column {
                label: "A".to_string(),
            },
            Column {
                label: "B".to_string(),
            },
            Column {
                label: "C".to_string(),
            },
            Column {
                label: "D".to_string(),
            },
        ]
    }

    fn row_count(&self, _: &Self::State) -> usize {
        self.data.len()
    }

    fn cell<'a>(&'a self, _: &'a Self::State, row: usize, column: usize) -> Cell<'a> {
        Cell::Text(self.data[row][column])
    }

    fn save_cell(&mut self, _: &mut Self::State, row: usize, column: usize, value: CellUpdate) {
        self.data[row][column] = match value {
            CellUpdate::Text(mut input) => input.value_and_reset().leak(),
        }
    }

    fn new_row(&mut self, _: &mut Self::State) {
        todo!()
    }

    fn open_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State>> {
        todo!()
    }
}

struct Data<S> {
    state: S,
    view: Box<dyn TableView<State = S>>,
    columns: Vec<Column>,
    editing: Option<RefCell<(CellUpdate, Box<dyn Any>)>>,
    cursor_position: std::cell::Cell<Option<Position>>,
}

impl Default for Data<()> {
    fn default() -> Self {
        let view = TestData::default();
        let columns = view.columns(&());

        Self {
            state: (),
            view: Box::new(view),
            columns,
            editing: None,
            cursor_position: Default::default(),
        }
    }
}

impl<'a, S> TableData<'a> for &'a Data<S> {
    fn header(&self) -> Option<rat_ftable::textdata::Row<'a>> {
        Some(rat_ftable::textdata::Row::new(
            self.columns.iter().map(|c| &*c.label),
        ))
    }

    fn rows(&self) -> usize {
        self.view.row_count(&self.state)
    }

    fn render_cell(
        &self,
        ctx: &rat_ftable::TableContext,
        column: usize,
        row: usize,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        if let Some(content) = &self.editing
            && ctx.selected_cell
        {
            let (ref mut update, ref mut state) = *content.borrow_mut();

            self.cursor_position.set(match update {
                CellUpdate::Text(input) => draw(
                    &mut |p| textbox(p, input),
                    state.downcast_mut().unwrap(),
                    area,
                    buf,
                ),
            });

            return;
        }

        match self.view.cell(&self.state, row, column) {
            Cell::Text(text) => text.render(area, buf),
        }
    }
}

fn main() {
    run(&mut |p| {
        state(p, &mut |p,
                       (data, state): &mut (
            Data<()>,
            TableState<CellSelection>,
        )| {
            with_key_handler(
                p,
                &mut (data, state),
                |&mut (&mut ref mut data, &mut ref mut state), event| {
                    if let Some((update, widget_state)) =
                        data.editing.as_mut().map(RefCell::get_mut)
                    {
                        (match update {
                            CellUpdate::Text(input) => handle_key_event(
                                &mut |p| textbox(p, input),
                                widget_state.downcast_mut().unwrap(),
                                event,
                            ),
                        }) || (match event.code {
                            KeyCode::Esc => {
                                data.view.save_cell(
                                    &mut data.state,
                                    state.selection.lead_cell.unwrap().1,
                                    state.selection.lead_cell.unwrap().0,
                                    data.editing.take().unwrap().into_inner().0,
                                );

                                true
                            }
                            _ => false,
                        })
                    } else {
                        match event.code {
                            KeyCode::Left | KeyCode::Char('h') => state.selection.move_left(1, 3),
                            KeyCode::Right | KeyCode::Char('l') => state.selection.move_right(1, 3),
                            KeyCode::Up | KeyCode::Char('k') => state.selection.move_up(1, 2),
                            KeyCode::Down | KeyCode::Char('j') => state.selection.move_down(1, 2),
                            KeyCode::Enter => {
                                data.editing = Some(RefCell::new(
                                    match data.view.cell(
                                        &mut data.state,
                                        state.selection.lead_cell.unwrap().1,
                                        state.selection.lead_cell.unwrap().0,
                                    ) {
                                        Cell::Text(text) => {
                                            let mut input = Input::new(text.to_string());
                                            let state =
                                                Box::new(init(&mut |p| textbox(p, &mut input)));

                                            (CellUpdate::Text(input), state)
                                        }
                                    },
                                ));
                                true
                            }
                            _ => false,
                        }
                    }
                },
                |p, &mut (&mut ref mut data, &mut ref mut state)| {
                    fn widget<'a>(
                        p: Pass<'a>,
                        data: &mut Data<()>,
                        state: &mut TableState<CellSelection>,
                    ) -> PassReturn<'a, impl Sized + 'static + use<>> {
                        ratatui_stateful_widget(
                            p,
                            Table::new()
                                .data(&*data)
                                .widths(data.columns.iter().map(|_| Constraint::Fill(1)))
                                .styles(TableStyle {
                                    show_cell_focus: true,
                                    select_cell: Some(Style {
                                        bg: Some(Color::Red),
                                        ..Default::default()
                                    }),
                                    ..TableStyle::default()
                                }),
                            state,
                        )
                    }

                    p.apply(
                        (data, state),
                        |(data, state)| init(&mut |p| widget(p, data, state)),
                        |(data, state), widget_state, area, buffer| {
                            draw(&mut |p| widget(p, data, state), widget_state, area, buffer);

                            data.cursor_position.take()
                        },
                        |_data, _state, _event| false,
                    )
                },
            )
        })
    })
    .unwrap();
}
