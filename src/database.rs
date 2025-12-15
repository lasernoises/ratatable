use crate::TableView;

#[derive(Default)]
pub struct Database {
    tables: Vec<Table>,
    next_table_id: u32,
}

pub struct Table {
    id: u32,
    name: String,
    /// Is always sorted. The index of a row in the columns is the same as the index here.
    row_ids: Vec<u32>,
    next_row_id: u32,
    columns: Vec<Column>,
    sort_index: Vec<u32>,
}

pub struct Column {
    name: String,
    content: ColumnContent,
}

enum ColumnContent {
    Bool(Vec<bool>),
    Int(Vec<i64>),
    Text(Vec<String>),
}

pub struct MainView {}

impl TableView for MainView {
    type State = Database;

    fn columns(&self, state: &Self::State) -> Vec<crate::Column> {
        vec![
            crate::Column {
                label: "Table".to_string(),
            },
            crate::Column {
                label: "Schema".to_string(),
            },
            crate::Column {
                label: "Content".to_string(),
            },
        ]
    }

    fn row_count(&self, state: &Self::State) -> usize {
        state.tables.len()
    }

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> crate::Cell<'a> {
        match column {
            0 => crate::Cell::Text(&state.tables[row].name),
            1 | 2 => crate::Cell::Link,
            _ => unreachable!(),
        }
    }

    fn save_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
        value: crate::CellUpdate,
    ) {
        assert!(column == 0);

        state.tables[row].name = match value {
            crate::CellUpdate::Text(mut input) => input.value_and_reset(),
        }
    }

    fn new_row(&mut self, state: &mut Self::State) {
        state.tables.push(Table {
            id: state.next_table_id,
            name: String::new(),
            row_ids: Vec::new(),
            next_row_id: 0,
            columns: Vec::new(),
            sort_index: Vec::new(),
        });

        state.next_table_id += 1;
    }

    fn open_cell(
        &mut self,
        _state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State>> {
        match column {
            1 => Box::new(TableSchemaView { table_idx: row }),
            2 => Box::new(TableContentView { table_idx: row }),
            _ => unreachable!(),
        }
    }
}

pub struct TableSchemaView {
    table_idx: usize,
}

impl TableView for TableSchemaView {
    type State = Database;

    fn columns(&self, _: &Self::State) -> Vec<crate::Column> {
        vec![
            crate::Column {
                label: "Column".to_string(),
            },
            crate::Column {
                label: "Type".to_string(),
            },
        ]
    }

    fn row_count(&self, state: &Self::State) -> usize {
        state.tables[self.table_idx].columns.len()
    }

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> crate::Cell<'a> {
        let table = &state.tables[self.table_idx];

        match column {
            0 => crate::Cell::Text(&table.columns[row].name),
            1 => crate::Cell::Text("boolean"),
            _ => unreachable!(),
        }
    }

    fn save_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
        value: crate::CellUpdate,
    ) {
        let table = &mut state.tables[self.table_idx];

        match column {
            0 => {
                table.columns[row].name = value.as_text().value_and_reset();
            }
            _ => unreachable!(),
        }
    }

    fn new_row(&mut self, state: &mut Self::State) {
        let table = &mut state.tables[self.table_idx];
        table.columns.push(Column {
            name: String::new(),
            content: ColumnContent::Bool(vec![false; table.row_ids.len()]),
        });
    }
}

pub struct TableContentView {
    table_idx: usize,
}

impl TableView for TableContentView {
    type State = Database;

    fn columns(&self, state: &Self::State) -> Vec<crate::Column> {
        state.tables[self.table_idx]
            .columns
            .iter()
            .map(|col| crate::Column {
                label: col.name.clone(),
            })
            .collect()
    }

    fn row_count(&self, state: &Self::State) -> usize {
        state.tables[self.table_idx].row_ids.len()
    }

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> crate::Cell<'a> {
        todo!()
    }

    fn save_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
        value: crate::CellUpdate,
    ) {
        todo!()
    }

    fn new_row(&mut self, state: &mut Self::State) {
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
