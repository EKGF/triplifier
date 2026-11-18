//! Format readers. Each reader walks its input once, in physical order, and
//! feeds cells into a [`TableBuilder`]; nothing is mapped or interpreted.

mod csv;
mod parquet;
mod spreadsheet;

use {
    crate::{
        emit::{CellValue, SourceEmitter, ViewEmitter},
        formula::{Formula, R1C1},
        iri::{ColumnNumber, RowNumber},
        source::{Source, SourceKind},
    },
    std::collections::{BTreeMap, BTreeSet},
};

/// Read `source` completely, emitting into `emitter`.
pub async fn read(
    source: &Source,
    emitter: SourceEmitter,
) -> anyhow::Result<SourceEmitter> {
    match source.kind() {
        SourceKind::Csv => csv::read(source.path(), b',', emitter).await,
        SourceKind::Tsv => csv::read(source.path(), b'\t', emitter).await,
        SourceKind::Parquet => {
            parquet::read(source.path().to_owned(), emitter).await
        },
        SourceKind::Xlsx | SourceKind::Xls | SourceKind::Ods => {
            spreadsheet::read(source.path().to_owned(), emitter).await
        },
    }
}

/// Turns a row-major stream of `(physical row, physical column, cell)` into
/// raw columns, rows and values. The first physical row seen is the header.
pub(crate) struct TableBuilder<'v, 'e> {
    view:            &'v mut ViewEmitter<'e>,
    header_row:      Option<u64>,
    header:          BTreeMap<u32, String>,
    header_done:     bool,
    current_row:     Option<u64>,
    column_formulas: BTreeMap<u32, BTreeSet<R1C1>>,
}

impl<'v, 'e> TableBuilder<'v, 'e> {
    pub fn new(view: &'v mut ViewEmitter<'e>) -> Self {
        Self {
            view,
            header_row: None,
            header: BTreeMap::new(),
            header_done: false,
            current_row: None,
            column_formulas: BTreeMap::new(),
        }
    }

    pub fn view(&mut self) -> &mut ViewEmitter<'e> { self.view }

    /// `row` and `column` are 0-based physical positions.
    pub fn cell(&mut self, row: u64, column: u32, cell: CellValue) {
        self.put(row, column, cell, None);
    }

    /// A cell computed by `formula`; `cached` is the value the workbook
    /// stored for it, if any.
    pub fn formula_cell(
        &mut self,
        row: u64,
        column: u32,
        cached: Option<CellValue>,
        formula: Formula,
    ) {
        let cell = cached.unwrap_or(CellValue::Null { source_text: None });
        self.put(row, column, cell, Some(formula));
    }

    fn put(
        &mut self,
        row: u64,
        column: u32,
        cell: CellValue,
        formula: Option<Formula>,
    ) {
        let header_row = *self.header_row.get_or_insert(row);
        let physical = (row, column);
        let column = ColumnNumber(column + 1);
        if row <= header_row {
            if row == header_row {
                self.header.insert(column.0, cell.header_text());
            }
            return;
        }
        self.finish_header();
        let row = RowNumber(row - header_row - 1);
        if self.current_row != Some(row.0) {
            self.current_row = Some(row.0);
            self.view.row(row);
        }
        self.view.value(row, column, &cell);
        if let Some(formula) = formula {
            let r1c1 = formula.to_r1c1(physical.0, physical.1);
            self.view.formula(row, column, &formula, &r1c1);
            let forms = self.column_formulas.entry(column.0).or_default();
            if forms.len() < 2 {
                forms.insert(r1c1);
            }
        }
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

    pub fn finish(mut self) {
        self.finish_header();
        self.view.finish_profiles();
        for (column, forms) in std::mem::take(&mut self.column_formulas) {
            if let [single] = forms.into_iter().collect::<Vec<_>>().as_slice() {
                self.view.column_formula(ColumnNumber(column), single);
            }
        }
    }
}

impl CellValue {
    fn header_text(&self) -> String {
        match self {
            Self::Text(s) | Self::Duration(s) => s.trim().to_owned(),
            Self::Integer(i) => i.to_string(),
            Self::Double(d) => d.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::DateTime(dt) => dt.to_string(),
            Self::Date(d) => d.to_string(),
            Self::DateTimeUtc(dt) => dt.to_rfc3339(),
            Self::Decimal(s) => s.clone(),
            Self::Time(t) => t.to_string(),
            Self::Binary(b) => hex::encode_upper(b),
            Self::Null { source_text } => {
                source_text.clone().unwrap_or_default()
            },
        }
    }
}
