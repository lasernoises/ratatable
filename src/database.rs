use std::ops::Range;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Database {
    tables: Vec<ToplevelTable>,
    next_table_id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ToplevelTable {
    id: u32,
    name: String,
    table: Table,
}

#[derive(Serialize, Deserialize)]
struct Subtable {
    table: Table,

    /// Each row corresponds to a row of the outer table. For nested subtables, the ranges of the
    /// outer subtable reference the `rows` field of the inner subtable. The ranges aren't absolute,
    /// but relative to the current level of nesting.
    rows: Vec<Range<usize>>,
}

#[derive(Clone)]
pub struct SubtableIdx {
    pub column_idx: usize,
    pub row_idx: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Table {
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
    Subtable(Subtable),
}

#[derive(PartialEq, Eq)]
pub enum ColumnType {
    Bool,
    Int,
    Text,
    Subtable,
}

pub enum CellContent<'a> {
    Bool(bool),
    Int(i64),
    Text(&'a str),
    Subtable,
}

pub enum CellContentUpdate {
    Bool(bool),
    Int(i64),
    Text(String),
}

impl Database {
    pub fn add_table(&mut self) {
        self.tables.push(ToplevelTable {
            id: self.next_table_id,
            name: String::new(),
            table: Table {
                row_ids: Vec::new(),
                next_row_id: 0,
                columns: Vec::new(),
                sort_index: Vec::new(),
            },
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

    fn resolve_table(&self, table_idx: usize, subtable_path: &[usize]) -> &Table {
        let mut table = &self.tables[table_idx].table;

        for &column_idx in subtable_path {
            table = match &table.columns[column_idx].content {
                ColumnContent::Subtable(subtable) => &subtable.table,
                _ => panic!("column is not a subtable"),
            }
        }

        table
    }

    fn resolve_table_mut(&mut self, table_idx: usize, subtable_path: &[usize]) -> &mut Table {
        let mut table = &mut self.tables[table_idx].table;

        for &column_idx in subtable_path {
            table = match &mut table.columns[column_idx].content {
                ColumnContent::Subtable(subtable) => &mut subtable.table,
                _ => panic!("column is not a subtable"),
            }
        }

        table
    }

    fn resolve_table_with_rows(
        &self,
        table_idx: usize,
        subtable_path: &[SubtableIdx],
    ) -> (&Table, Range<usize>) {
        let mut table = &self.tables[table_idx].table;
        let mut range = 0..table.row_ids.len();

        for &SubtableIdx {
            column_idx,
            row_idx,
        } in subtable_path
        {
            let subtable = match &table.columns[column_idx].content {
                ColumnContent::Subtable(subtable) => subtable,
                _ => panic!("column is not a subtable"),
            };

            table = &subtable.table;
            range = subtable.rows[range][row_idx].clone();
        }

        (table, range)
    }

    fn resolve_table_with_rows_mut(
        &mut self,
        table_idx: usize,
        subtable_path: &[SubtableIdx],
    ) -> (&mut Table, Range<usize>) {
        let mut table = &mut self.tables[table_idx].table;
        let mut range = 0..table.row_ids.len();

        for &SubtableIdx {
            column_idx,
            row_idx,
        } in subtable_path
        {
            let subtable = match &mut table.columns[column_idx].content {
                ColumnContent::Subtable(subtable) => subtable,
                _ => panic!("column is not a subtable"),
            };

            table = &mut subtable.table;
            range = subtable.rows[range][row_idx].clone();
        }

        (table, range)
    }

    pub fn table_columns_len(&self, table_idx: usize, subtable_path: &[usize]) -> usize {
        self.resolve_table(table_idx, subtable_path).columns.len()
    }

    pub fn table_column_name(
        &self,
        table_idx: usize,
        subtable_path: &[usize],
        column_idx: usize,
    ) -> &str {
        &self.resolve_table(table_idx, subtable_path).columns[column_idx].name
    }

    pub fn set_table_column_name(
        &mut self,
        table_idx: usize,
        subtable_path: &[usize],
        column_idx: usize,
        name: String,
    ) {
        self.resolve_table_mut(table_idx, subtable_path).columns[column_idx].name = name;
    }

    pub fn table_column_type(
        &self,
        table_idx: usize,
        subtable_path: &[usize],
        column_idx: usize,
    ) -> ColumnType {
        match self.resolve_table(table_idx, subtable_path).columns[column_idx].content {
            ColumnContent::Bool(_) => ColumnType::Bool,
            ColumnContent::Int(_) => ColumnType::Int,
            ColumnContent::Text(_) => ColumnType::Text,
            ColumnContent::Subtable(_) => ColumnType::Subtable,
        }
    }

    pub fn set_table_column_type(
        &mut self,
        table_idx: usize,
        subtable_path: &[usize],
        column_idx: usize,
        column_type: ColumnType,
    ) {
        let column_content =
            &mut self.resolve_table_mut(table_idx, subtable_path).columns[column_idx].content;

        match column_type {
            ColumnType::Bool => column_content.change_to_bool(),
            ColumnType::Int => column_content.change_to_int(),
            ColumnType::Text => column_content.change_to_text(),
            ColumnType::Subtable => column_content.change_to_subtable(),
        };
    }

    pub fn add_table_column(&mut self, table_idx: usize, subtable_path: &[usize]) {
        let table = self.resolve_table_mut(table_idx, subtable_path);
        table.columns.push(Column {
            name: String::new(),
            content: ColumnContent::Bool(vec![false; table.row_ids.len()]),
        });
    }

    pub fn table_column_names(
        &self,
        table_idx: usize,
        subtable_path: &[SubtableIdx],
    ) -> impl Iterator<Item = &str> {
        self.resolve_table_with_rows(table_idx, subtable_path)
            .0
            .columns
            .iter()
            .map(|c| &*c.name)
    }

    pub fn table_row_count(&self, table_idx: usize, subtable_path: &[SubtableIdx]) -> usize {
        let (_, range) = self.resolve_table_with_rows(table_idx, subtable_path);
        range.end - range.start
    }

    pub fn table_cell_content(
        &self,
        table_idx: usize,
        subtable_path: &[SubtableIdx],
        column_idx: usize,
        row_idx: usize,
    ) -> CellContent<'_> {
        let (table, range) = self.resolve_table_with_rows(table_idx, subtable_path);

        match &table.columns[column_idx].content {
            ColumnContent::Bool(items) => CellContent::Bool(items[range][row_idx]),
            ColumnContent::Int(items) => CellContent::Int(items[range][row_idx]),
            ColumnContent::Text(items) => CellContent::Text(&items[range][row_idx]),
            ColumnContent::Subtable(_) => CellContent::Subtable,
        }
    }

    pub fn set_table_cell_content(
        &mut self,
        table_idx: usize,
        subtable_path: &[SubtableIdx],
        column_idx: usize,
        row_idx: usize,
        update: CellContentUpdate,
    ) {
        let (table, range) = self.resolve_table_with_rows_mut(table_idx, subtable_path);

        match (&mut table.columns[column_idx].content, update) {
            (ColumnContent::Bool(items), CellContentUpdate::Bool(value)) => {
                items[range][row_idx] = value;
            }
            (ColumnContent::Int(items), CellContentUpdate::Int(value)) => {
                items[range][row_idx] = value;
            }
            (ColumnContent::Text(items), CellContentUpdate::Text(value)) => {
                items[range][row_idx] = value;
            }
            _ => panic!("update does not match column type"),
        }
    }

    pub fn add_table_row(&mut self, table_idx: usize, subtable_path: &[SubtableIdx]) {
        let mut table = &mut self.tables[table_idx].table;
        let mut range = 0..table.row_ids.len();
        let mut ranges = None;

        for &SubtableIdx {
            column_idx,
            row_idx,
        } in subtable_path
        {
            let subtable = match &mut table.columns[column_idx].content {
                ColumnContent::Subtable(subtable) => subtable,
                _ => panic!("column is not a subtable"),
            };

            table = &mut subtable.table;
            let idx = range.start + row_idx;
            range = subtable.rows[range][row_idx].clone();
            ranges = Some((idx, &mut subtable.rows));
        }

        table.row_ids.push(table.next_row_id);
        table.next_row_id += 1;

        for column in &mut table.columns {
            match &mut column.content {
                ColumnContent::Bool(items) => items.insert(range.end, false),
                ColumnContent::Int(items) => items.insert(range.end, 0),
                ColumnContent::Text(items) => items.insert(range.end, String::new()),
                ColumnContent::Subtable(subtable) => subtable.rows.insert(
                    range.end,
                    subtable.table.row_ids.len()..subtable.table.row_ids.len(),
                ),
            }
        }

        if let Some((row_idx, ranges)) = ranges {
            ranges[row_idx].end += 1;

            for range in &mut ranges[row_idx + 1..] {
                range.start += 1;
                range.end += 1;
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
            ColumnContent::Subtable(subtable) => subtable.table.row_ids.len(),
        }
    }

    fn change_to_bool(&mut self) {
        if !matches!(self, ColumnContent::Bool(_)) {
            *self = ColumnContent::Bool(vec![false; self.len()]);
        }
    }

    fn change_to_int(&mut self) {
        if !matches!(self, ColumnContent::Int(_)) {
            *self = ColumnContent::Int(vec![0; self.len()]);
        }
    }

    fn change_to_text(&mut self) {
        if !matches!(self, ColumnContent::Text(_)) {
            *self = ColumnContent::Text(vec![String::new(); self.len()]);
        }
    }

    fn change_to_subtable(&mut self) {
        if !matches!(self, ColumnContent::Subtable(_)) {
            *self = ColumnContent::Subtable(Subtable {
                table: Table {
                    row_ids: Vec::new(),
                    next_row_id: 0,
                    columns: Vec::new(),
                    sort_index: Vec::new(),
                },
                rows: vec![0..0; self.len()],
            });
        }
    }
}
