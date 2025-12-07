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
