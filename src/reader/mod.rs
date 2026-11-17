//! Format readers. Each reader walks its input once, in physical order, and
//! feeds cells into a [`TableBuilder`]; nothing is mapped or interpreted.

mod csv;
mod spreadsheet;

use {
    crate::{
        emit::{CellValue, SourceEmitter, ViewEmitter},
        iri::{ColumnNumber, RowNumber},
        source::{Source, SourceKind},
    },
    std::collections::BTreeMap,
};

/// Read `source` completely, emitting into `emitter`.
pub async fn read(
    source: &Source,
    emitter: SourceEmitter,
) -> anyhow::Result<SourceEmitter> {
    match source.kind() {
        SourceKind::Csv => csv::read(source.path(), b',', emitter).await,
        SourceKind::Tsv => csv::read(source.path(), b'\t', emitter).await,
        SourceKind::Xlsx | SourceKind::Xls | SourceKind::Ods => {
            spreadsheet::read(source.path().to_owned(), emitter).await
        },
    }
}

/// Turns a row-major stream of `(physical row, physical column, cell)` into
/// raw columns, rows and values. The first physical row seen is the header.
pub(crate) struct TableBuilder<'v, 'e> {
    view:        &'v mut ViewEmitter<'e>,
    header_row:  Option<u64>,
    header:      BTreeMap<u32, String>,
    header_done: bool,
    current_row: Option<u64>,
}

impl<'v, 'e> TableBuilder<'v, 'e> {
    pub fn new(view: &'v mut ViewEmitter<'e>) -> Self {
        Self {
            view,
            header_row: None,
            header: BTreeMap::new(),
            header_done: false,
            current_row: None,
        }
    }

    pub fn view(&mut self) -> &mut ViewEmitter<'e> { self.view }

    /// `row` and `column` are 0-based physical positions.
    pub fn cell(&mut self, row: u64, column: u32, cell: CellValue) {
        let header_row = *self.header_row.get_or_insert(row);
        let column = ColumnNumber(column + 1);
        if row == header_row {
            self.header.insert(column.0, cell.header_text());
            return;
        }
        self.finish_header();
        let row = RowNumber(row - header_row - 1);
        if self.current_row != Some(row.0) {
            self.current_row = Some(row.0);
            self.view.row(row);
        }
        self.view.value(row, column, &cell);
    }

    fn finish_header(&mut self) {
        if self.header_done {
            return;
        }
        self.header_done = true;
        let max = self.header.keys().next_back().copied().unwrap_or(0);
        for c in 1..=max {
            self.view.column(
                ColumnNumber(c),
                self.header.get(&c).map(String::as_str),
            );
        }
    }

    pub fn finish(mut self) { self.finish_header(); }
}

impl CellValue {
    fn header_text(&self) -> String {
        match self {
            Self::Text(s) | Self::Duration(s) => s.trim().to_owned(),
            Self::Integer(i) => i.to_string(),
            Self::Double(d) => d.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::DateTime(dt) => dt.to_string(),
            Self::Null { source_text } => {
                source_text.clone().unwrap_or_default()
            },
        }
    }
}
