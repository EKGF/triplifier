//! XLSX / XLSM / XLSB / XLS / ODS via calamine. Cell types are the ones the
//! workbook stores; we do not reinterpret text.

use {
    super::TableBuilder,
    crate::{
        emit::{CellValue, SourceEmitter},
        iri::ViewNumber,
    },
    anyhow::Context,
    calamine::{Data, DataRef, Reader, Sheets, open_workbook_auto},
    std::path::PathBuf,
};

pub async fn read(
    path: PathBuf,
    emitter: SourceEmitter,
) -> anyhow::Result<SourceEmitter> {
    tokio::task::spawn_blocking(move || read_blocking(&path, emitter)).await?
}

fn read_blocking(
    path: &std::path::Path,
    mut emitter: SourceEmitter,
) -> anyhow::Result<SourceEmitter> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    let names = workbook.sheet_names();
    for (index, name) in names.iter().enumerate() {
        let number = ViewNumber(u32::try_from(index + 1)?);
        let mut view = emitter.view(number, Some(name));
        let mut table = TableBuilder::new(&mut view);
        match &mut workbook {
            // XLSX: true streaming, one cell at a time.
            Sheets::Xlsx(xlsx) => {
                let mut cells = xlsx.worksheet_cells_reader(name)?;
                while let Some(cell) = cells.next_cell()? {
                    let (row, col) = cell.get_position();
                    if let Some(value) =
                        CellValue::from_data_ref(cell.get_value())
                    {
                        table.cell(u64::from(row), col, value);
                    }
                    table.view().out().flush_if_full_blocking()?;
                }
            },
            other => {
                let range = other.worksheet_range(name)?;
                let (row0, col0) = range.start().unwrap_or((0, 0));
                for (r, cells) in range.rows().enumerate() {
                    for (c, data) in cells.iter().enumerate() {
                        if let Some(value) = CellValue::from_data(data) {
                            table.cell(
                                u64::from(row0) + r as u64,
                                col0 + u32::try_from(c)?,
                                value,
                            );
                        }
                    }
                    table.view().out().flush_if_full_blocking()?;
                }
            },
        }
        table.finish();
    }
    emitter.out().flush_blocking()?;
    Ok(emitter)
}

impl CellValue {
    fn from_data(d: &Data) -> Option<Self> {
        Some(match d {
            Data::Empty => return None,
            Data::String(s) => Self::Text(s.clone()),
            Data::Int(i) => Self::Integer(*i),
            Data::Float(f) => Self::Double(*f),
            Data::Bool(b) => Self::Boolean(*b),
            Data::DateTime(dt) => Self::excel_date_time(dt),
            Data::DateTimeIso(s) => Self::iso_date_time(s),
            Data::DurationIso(s) => Self::Duration(s.clone()),
            Data::Error(e) => Self::Null { source_text: Some(e.to_string()) },
        })
    }

    fn from_data_ref(d: &DataRef<'_>) -> Option<Self> {
        Some(match d {
            DataRef::Empty => return None,
            DataRef::String(s) => Self::Text(s.clone()),
            DataRef::SharedString(s) => Self::Text((*s).to_owned()),
            DataRef::Int(i) => Self::Integer(*i),
            DataRef::Float(f) => Self::Double(*f),
            DataRef::Bool(b) => Self::Boolean(*b),
            DataRef::DateTime(dt) => Self::excel_date_time(dt),
            DataRef::DateTimeIso(s) => Self::iso_date_time(s),
            DataRef::DurationIso(s) => Self::Duration(s.clone()),
            DataRef::Error(e) => {
                Self::Null { source_text: Some(e.to_string()) }
            },
        })
    }

    fn excel_date_time(dt: &calamine::ExcelDateTime) -> Self {
        if dt.is_duration() {
            // Excel durations are fractional days; keep the serial number.
            return Self::Double(dt.as_f64());
        }
        match dt.as_datetime() {
            Some(ndt) => Self::DateTime(ndt),
            None => Self::Double(dt.as_f64()),
        }
    }

    fn iso_date_time(s: &str) -> Self {
        s.parse::<chrono::NaiveDateTime>()
            .map(Self::DateTime)
            .unwrap_or_else(|_| Self::Text(s.to_owned()))
    }
}
