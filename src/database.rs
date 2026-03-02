use std::ops::Range;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Database {
    tables: Vec<Table>,
    next_table_id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Table {
    id: u32,
    name: String,
    /// Is always sorted. The index of a row in the columns is the same as the index here.
    row_ids: Vec<u32>,
    next_row_id: u32,
    columns: Vec<Column>,
    sort_index: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct Column {
    name: String,
    content: ColumnContent,
}

#[derive(Serialize, Deserialize)]
enum ColumnContent {
    Bool(Vec<bool>),
    Int(Vec<i64>),
    Text(Vec<String>),
}

pub enum ColumnType {
    Bool,
    Int,
    Text,
}

pub enum CellContent<'a> {
    Bool(bool),
    Int(i64),
    Text(&'a str),
}

pub enum CellContentUpdate {
    Bool(bool),
    Int(i64),
    Text(String),
}

impl Database {
    pub fn add_table(&mut self) {
        self.tables.push(Table {
            id: self.next_table_id,
            name: String::new(),
            row_ids: Vec::new(),
            next_row_id: 0,
            columns: Vec::new(),
            sort_index: Vec::new(),
        });

        self.next_table_id += 1;
    }

    pub fn tables_len(&self) -> usize {
        self.tables.len()
    }

    pub fn table_name(&self, table_idx: usize) -> &str {
        &self.tables[table_idx].name
    }

    pub fn set_table_name(&mut self, table_idx: usize, name: String) {
        self.tables[table_idx].name = name;
    }

    pub fn table_columns_len(&self, table_idx: usize) -> usize {
        self.tables[table_idx].columns.len()
    }

    pub fn table_column_name(&self, table_idx: usize, column_idx: usize) -> &str {
        &self.tables[table_idx].columns[column_idx].name
    }

    pub fn set_table_column_name(&mut self, table_idx: usize, column_idx: usize, name: String) {
        self.tables[table_idx].columns[column_idx].name = name;
    }

    pub fn table_column_type(&self, table_idx: usize, column_idx: usize) -> ColumnType {
        match self.tables[table_idx].columns[column_idx].content {
            ColumnContent::Bool(_) => ColumnType::Bool,
            ColumnContent::Int(_) => ColumnType::Int,
            ColumnContent::Text(_) => ColumnType::Text,
        }
    }

    pub fn set_table_column_type(
        &mut self,
        table_idx: usize,
        column_idx: usize,
        column_type: ColumnType,
    ) {
        let column_content = &mut self.tables[table_idx].columns[column_idx].content;
        match column_type {
            ColumnType::Bool => column_content.change_to_bool(),
            ColumnType::Int => column_content.change_to_int(),
            ColumnType::Text => column_content.change_to_text(),
        };
    }

    pub fn add_table_column(&mut self, table_idx: usize) {
        let table = &mut self.tables[table_idx];
        table.columns.push(Column {
            name: String::new(),
            content: ColumnContent::Bool(vec![false; table.row_ids.len()]),
        });
    }

    pub fn table_column_names(&self, table_idx: usize) -> impl Iterator<Item = &str> {
        self.tables[table_idx].columns.iter().map(|c| &*c.name)
    }

    pub fn table_row_count(&self, table_idx: usize) -> usize {
        self.tables[table_idx].row_ids.len()
    }

    pub fn table_cell_content(
        &self,
        table_idx: usize,
        column_idx: usize,
        row_idx: usize,
    ) -> CellContent<'_> {
        match &self.tables[table_idx].columns[column_idx].content {
            ColumnContent::Bool(items) => CellContent::Bool(items[row_idx]),
            ColumnContent::Int(items) => CellContent::Int(items[row_idx]),
            ColumnContent::Text(items) => CellContent::Text(&items[row_idx]),
        }
    }

    pub fn set_table_cell_content(
        &mut self,
        table_idx: usize,
        column_idx: usize,
        row_idx: usize,
        update: CellContentUpdate,
    ) {
        match (
            &mut self.tables[table_idx].columns[column_idx].content,
            update,
        ) {
            (ColumnContent::Bool(items), CellContentUpdate::Bool(value)) => {
                items[row_idx] = value;
            }
            (ColumnContent::Int(items), CellContentUpdate::Int(value)) => {
                items[row_idx] = value;
            }
            (ColumnContent::Text(items), CellContentUpdate::Text(value)) => {
                items[row_idx] = value;
            }
            _ => panic!("Update does not match column type!"),
        }
    }

    pub fn add_table_row(&mut self, table_idx: usize) {
        let table = &mut self.tables[table_idx];

        table.row_ids.push(table.next_row_id);
        table.next_row_id += 1;

        for column in &mut table.columns {
            match &mut column.content {
                ColumnContent::Bool(items) => items.push(false),
                ColumnContent::Int(items) => items.push(0),
                ColumnContent::Text(items) => items.push(String::new()),
            }
        }
    }
}

impl ColumnContent {
    fn len(&self) -> usize {
        match self {
            ColumnContent::Bool(items) => items.len(),
            ColumnContent::Int(items) => items.len(),
            ColumnContent::Text(items) => items.len(),
        }
    }

    fn change_to_bool(&mut self) {
        if !matches!(self, ColumnContent::Bool(_)) {
            *self = ColumnContent::Bool(vec![false; self.len()])
        }
    }

    fn change_to_int(&mut self) {
        if !matches!(self, ColumnContent::Int(_)) {
            *self = ColumnContent::Int(vec![0; self.len()])
        }
    }

    fn change_to_text(&mut self) {
        if !matches!(self, ColumnContent::Text(_)) {
            *self = ColumnContent::Text(vec![String::new(); self.len()])
        }
    }
}
