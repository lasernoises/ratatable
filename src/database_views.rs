use std::{
    fs::File,
    io::{BufWriter, ErrorKind},
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;

use crate::{TableView, database::*};

#[derive(Default)]
pub struct State {
    db: Database,
    save_path: Option<PathBuf>,
}

impl State {
    pub fn load(path: &Path) -> Result<Self> {
        let db = std::fs::read(path).map_or_else(
            |e| {
                if e.kind() == ErrorKind::NotFound {
                    Ok(Database::default())
                } else {
                    Err(e)
                }
            },
            |data| serde_json::from_slice(&data).map_err(Into::into),
        )?;

        Ok(State {
            db,
            save_path: Some(path.to_path_buf()),
        })
    }

    fn save(&mut self) {
        if let Some(path) = &self.save_path {
            // TODO: We're currently not calling this codepath in the ssh server where we use tokio,
            // so blocking I/O here isn't a problem practically speaking. But given that we'll
            // eventually want persistent storage in some form in the SSH server we should probably
            // make the TableView functions async.
            //
            // Also we'll need to handle I/O errors in some way. Probably we should display a prompt
            // or something like that. This will need to be handled through the TableView interface
            // as well.
            serde_json::to_writer(BufWriter::new(File::create(path).unwrap()), &self.db).unwrap();
        }
    }
}

pub struct MainView {}

impl TableView for MainView {
    type State = State;

    fn columns(&self, _state: &Self::State) -> Vec<crate::Column> {
        vec![
            crate::Column {
                label: "Table".to_string(),
            },
            crate::Column {
                label: "Content".to_string(),
            },
            crate::Column {
                label: "Schema".to_string(),
            },
        ]
    }

    fn row_count(&self, state: &Self::State) -> usize {
        state.db.tables_len()
    }

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> crate::Cell<'a> {
        match column {
            0 => crate::Cell::Text(state.db.table_name(row)),
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

        state.db.set_table_name(row, value.as_text());
        state.save();
    }

    fn new_row(&mut self, state: &mut Self::State) {
        state.db.add_table();
        state.save();
    }

    fn open_cell(
        &mut self,
        _state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State> + Send> {
        match column {
            1 => Box::new(TableContentView {
                table_idx: row,
                subtable_path: Vec::new(),
            }),
            2 => Box::new(TableSchemaView {
                table_idx: row,
                subtable_path: Vec::new(),
            }),
            _ => unreachable!(),
        }
    }
}

pub struct TableSchemaView {
    table_idx: usize,
    subtable_path: Vec<usize>,
}

impl TableView for TableSchemaView {
    type State = State;

    fn columns(&self, _: &Self::State) -> Vec<crate::Column> {
        vec![
            crate::Column {
                label: "Column".to_string(),
            },
            crate::Column {
                label: "Type".to_string(),
            },
            crate::Column {
                label: "Subtable".to_string(),
            },
        ]
    }

    fn row_count(&self, state: &Self::State) -> usize {
        state
            .db
            .table_columns_len(self.table_idx, &self.subtable_path)
    }

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> crate::Cell<'a> {
        let column_type = state
            .db
            .table_column_type(self.table_idx, &self.subtable_path, row);

        match column {
            0 => crate::Cell::Text(state.db.table_column_name(
                self.table_idx,
                &self.subtable_path,
                row,
            )),
            1 => crate::Cell::Select(match column_type {
                ColumnType::Bool => "boolean",
                ColumnType::Int => "int",
                ColumnType::Text => "text",
                ColumnType::Subtable => "subtable",
            }),
            2 => {
                if column_type == ColumnType::Subtable {
                    crate::Cell::Link
                } else {
                    crate::Cell::Empty
                }
            }
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
        match column {
            0 => {
                state.db.set_table_column_name(
                    self.table_idx,
                    &self.subtable_path,
                    row,
                    value.as_text(),
                );
            }
            1 => {
                state.db.set_table_column_type(
                    self.table_idx,
                    &self.subtable_path,
                    row,
                    match value.as_select() {
                        0 => ColumnType::Bool,
                        1 => ColumnType::Int,
                        2 => ColumnType::Text,
                        3 => ColumnType::Subtable,
                        _ => unreachable!(),
                    },
                );
            }
            _ => unreachable!(),
        }

        state.save();
    }

    fn open_cell(
        &mut self,
        _state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State> + Send> {
        assert_eq!(column, 2);

        Box::new(TableSchemaView {
            table_idx: self.table_idx,
            subtable_path: self.subtable_path.iter().cloned().chain([row]).collect(),
        })
    }

    fn new_row(&mut self, state: &mut Self::State) {
        state
            .db
            .add_table_column(self.table_idx, &self.subtable_path);
        state.save();
    }

    fn select_options(&self, _state: &Self::State, _row: usize, column: usize) -> Vec<String> {
        assert_eq!(column, 1);

        vec![
            "boolean".into(),
            "int".into(),
            "text".into(),
            "subtable".into(),
        ]
    }

    fn back(
        &mut self,
        _: &mut Self::State,
    ) -> Option<Box<dyn TableView<State = Self::State> + Send>> {
        Some(if self.subtable_path.is_empty() {
            Box::new(MainView {})
        } else {
            Box::new(TableSchemaView {
                table_idx: self.table_idx,
                subtable_path: self.subtable_path[0..(self.subtable_path.len() - 1)].to_vec(),
            })
        })
    }
}

pub struct TableContentView {
    table_idx: usize,
    subtable_path: Vec<SubtableIdx>,
}

impl TableView for TableContentView {
    type State = State;

    fn columns(&self, state: &Self::State) -> Vec<crate::Column> {
        state
            .db
            .table_column_names(self.table_idx, &self.subtable_path)
            .map(|name| crate::Column {
                label: name.to_string(),
            })
            .collect()
    }

    fn row_count(&self, state: &Self::State) -> usize {
        state
            .db
            .table_row_count(self.table_idx, &self.subtable_path)
    }

    fn cell<'a>(&'a self, state: &'a Self::State, row: usize, column: usize) -> crate::Cell<'a> {
        match state
            .db
            .table_cell_content(self.table_idx, &self.subtable_path, column, row)
        {
            CellContent::Bool(content) => crate::Cell::Checkbox(content),
            CellContent::Int(content) => crate::Cell::Int(content),
            CellContent::Text(content) => crate::Cell::Text(content),
            CellContent::Subtable => crate::Cell::Link,
        }
    }

    fn save_cell(
        &mut self,
        state: &mut Self::State,
        row: usize,
        column: usize,
        value: crate::CellUpdate,
    ) {
        state.db.set_table_cell_content(
            self.table_idx,
            &self.subtable_path,
            column,
            row,
            match value {
                crate::CellUpdate::Checkbox(value) => CellContentUpdate::Bool(value),
                crate::CellUpdate::Int(value) => CellContentUpdate::Int(value),
                crate::CellUpdate::Text(value) => CellContentUpdate::Text(value),
                crate::CellUpdate::Select(_) => todo!(),
            },
        );

        state.save();
    }

    fn new_row(&mut self, state: &mut Self::State) {
        state.db.add_table_row(self.table_idx, &self.subtable_path);
        state.save();
    }

    fn open_cell(
        &mut self,
        _state: &mut Self::State,
        row: usize,
        column: usize,
    ) -> Box<dyn TableView<State = Self::State> + Send> {
        Box::new(TableContentView {
            table_idx: self.table_idx,
            subtable_path: self
                .subtable_path
                .iter()
                .cloned()
                .chain([SubtableIdx {
                    column_idx: column,
                    row_idx: row,
                }])
                .collect(),
        })
    }

    fn back(
        &mut self,
        _: &mut Self::State,
    ) -> Option<Box<dyn TableView<State = Self::State> + Send>> {
        Some(if self.subtable_path.is_empty() {
            Box::new(MainView {})
        } else {
            Box::new(TableContentView {
                table_idx: self.table_idx,
                subtable_path: self.subtable_path[0..(self.subtable_path.len() - 1)].to_vec(),
            })
        })
    }
}
