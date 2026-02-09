use wraptatui::{run, widgets::state::state};

use crate::{database::Database, table::table};

mod database;
mod table;

pub enum Cell<'a> {
    Checkbox(bool),
    Int(i64),
    Text(&'a str),
    Select(&'a str),
    Link,
}

pub enum CellUpdate {
    Checkbox(bool),
    Int(i64),
    Text(String),
    Select(usize),
}

impl CellUpdate {
    pub fn as_checkbox(self) -> bool {
        match self {
            CellUpdate::Checkbox(checked) => checked,
            _ => panic!(),
        }
    }

    fn as_int(self) -> i64 {
        match self {
            CellUpdate::Int(int) => int,
            _ => panic!(),
        }
    }

    pub fn as_text(self) -> String {
        match self {
            CellUpdate::Text(text) => text,
            _ => panic!(),
        }
    }

    pub fn as_select(self) -> usize {
        match self {
            CellUpdate::Select(option) => option,
            _ => panic!(),
        }
    }
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

    #[allow(unused_variables)]
    fn select_options(&self, state: &Self::State, row: usize, column: usize) -> Vec<String> {
        unreachable!()
    }

    #[allow(unused_variables)]
    fn open_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State>> {
        unreachable!()
    }

    #[allow(unused_variables)]
    fn back(&mut self, state: &mut Self::State) -> Option<Box<dyn TableView<State = Self::State>>> {
        None
    }
}

fn main() {
    run(&mut |p| {
        state(p, |p, data: &mut Database| {
            table(p, data, || Box::new(database::MainView {}))
        })
    })
    .unwrap();
}
