use wraptatui::{
    run,
    widgets::{state::state, textbox::Input},
};

use crate::{database::Database, table::table};

mod database;
mod table;

pub enum Cell<'a> {
    Text(&'a str),
    Link,
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

fn main() {
    run(&mut |p| {
        state(p, &mut |p, data: &mut Database| {
            // table(p, data, || Box::new(TestData::default()))
            table(p, data, || Box::new(database::MainView {}))
        })
    })
    .unwrap();
}
