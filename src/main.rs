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

impl CellUpdate {
    pub fn as_text(self) -> Input {
        match self {
            CellUpdate::Text(input) => input,
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
    fn open_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State>> {
        unreachable!()
    }
}

fn main() {
    run(&mut |p| {
        state(p, &mut |p, data: &mut Database| {
            table(p, data, || Box::new(database::MainView {}))
        })
    })
    .unwrap();
}
