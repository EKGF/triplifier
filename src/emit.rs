//! Turns physical table structure into EKGF Raw quads.
//!
//! Readers only report *what they saw* (views, header cells, data cells) to a
//! [`ViewEmitter`]; this module alone decides how that becomes RDF.

use {
    crate::{
        iri::{ColumnNumber, RowNumber, SourceIris, ViewIris, ViewNumber},
        profile::{ColumnProfile, ColumnProfiler},
        vocab::{raw, rdf, rdfs, xsd},
    },
    oxrdf::{
        GraphName,
        Literal,
        NamedNode,
        NamedNodeRef,
        NamedOrBlankNode,
        Quad,
        Term,
    },
    std::collections::BTreeMap,
    tokio::sync::mpsc,
};

pub type QuadBatch = Vec<Quad>;
pub type QuadSender = mpsc::Sender<QuadBatch>;

const BATCH_SIZE: usize = 8 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("quad writer has shut down")]
pub struct WriterGone;

/// A cell exactly as the source format typed it. Formats without types
/// (CSV) only ever produce [`CellValue::Text`]; we never infer a type.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Text(String),
    Integer(i64),
    Double(f64),
    Boolean(bool),
    /// ISO 8601 date-time without zone, as stored by the source.
    DateTime(chrono::NaiveDateTime),
    /// ISO 8601 duration lexical form.
    Duration(String),
    /// A calendar date, as typed by the source (Parquet `DATE`).
    Date(chrono::NaiveDate),
    /// An instant, from a source type with a timezone (normalised to UTC).
    DateTimeUtc(chrono::DateTime<chrono::Utc>),
    /// An exact decimal in `xsd:decimal` lexical form.
    Decimal(String),
    /// A time of day without zone.
    Time(chrono::NaiveTime),
    /// Raw bytes, emitted as `xsd:hexBinary`.
    Binary(Vec<u8>),
    /// The source explicitly reported "no value" (e.g. `#N/A`), with its text.
    Null {
        source_text: Option<String>,
    },
}

impl CellValue {
    pub(crate) fn literal(&self) -> Option<Literal> {
        Some(match self {
            Self::Text(s) => Literal::new_simple_literal(s),
            Self::Integer(i) => Literal::from(*i),
            Self::Double(d) => Literal::from(*d),
            Self::Boolean(b) => Literal::from(*b),
            Self::DateTime(dt) => {
                Literal::new_typed_literal(
                    dt.format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
                    xsd::DATE_TIME,
                )
            },
            Self::Duration(d) => Literal::new_typed_literal(d, xsd::DURATION),
            Self::Date(d) => {
                Literal::new_typed_literal(
                    d.format("%Y-%m-%d").to_string(),
                    xsd::DATE,
                )
            },
            Self::DateTimeUtc(dt) => {
                Literal::new_typed_literal(
                    dt.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
                    xsd::DATE_TIME,
                )
            },
            Self::Decimal(s) => Literal::new_typed_literal(s, xsd::DECIMAL),
            Self::Time(t) => {
                Literal::new_typed_literal(
                    t.format("%H:%M:%S%.f").to_string(),
                    xsd::TIME,
                )
            },
            Self::Binary(b) => {
                Literal::new_typed_literal(
                    hex::encode_upper(b),
                    xsd::HEX_BINARY,
                )
            },
            Self::Null { .. } => return None,
        })
    }
}

/// Buffers quads for one named graph and ships them in batches.
#[derive(Debug)]
pub struct GraphEmitter {
    graph:  GraphName,
    buffer: QuadBatch,
    sender: QuadSender,
}

impl GraphEmitter {
    pub fn new(graph: NamedNode, sender: QuadSender) -> Self {
        Self {
            graph: graph.into(),
            buffer: Vec::with_capacity(BATCH_SIZE),
            sender,
        }
    }

    pub fn quad(
        &mut self,
        s: impl Into<NamedOrBlankNode>,
        p: NamedNodeRef<'_>,
        o: impl Into<Term>,
    ) {
        self.buffer.push(Quad::new(
            s,
            p.into_owned(),
            o,
            self.graph.clone(),
        ));
    }

    pub fn is_full(&self) -> bool { self.buffer.len() >= BATCH_SIZE }

    fn take(&mut self) -> QuadBatch {
        std::mem::replace(&mut self.buffer, Vec::with_capacity(BATCH_SIZE))
    }

    /// Flush from a blocking (non-runtime) thread if the batch is full.
    pub fn flush_if_full_blocking(&mut self) -> Result<(), WriterGone> {
        if self.is_full() {
            self.flush_blocking()
        } else {
            Ok(())
        }
    }

    pub fn flush_blocking(&mut self) -> Result<(), WriterGone> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let batch = self.take();
        self.sender.blocking_send(batch).map_err(|_| WriterGone)
    }

    pub async fn flush_if_full(&mut self) -> Result<(), WriterGone> {
        if self.is_full() {
            self.flush().await
        } else {
            Ok(())
        }
    }

    pub async fn flush(&mut self) -> Result<(), WriterGone> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let batch = self.take();
        self.sender.send(batch).await.map_err(|_| WriterGone)
    }
}

/// Emits one source graph: the source, its views, columns, rows and values.
#[derive(Debug)]
pub struct SourceEmitter {
    iris:    SourceIris,
    out:     GraphEmitter,
    profile: bool,
}

impl SourceEmitter {
    pub fn new(iris: SourceIris, out: GraphEmitter) -> Self {
        Self { iris, out, profile: false }
    }

    /// Profile every column (`--profile`).
    pub fn with_profile(mut self, profile: bool) -> Self {
        self.profile = profile;
        self
    }

    pub fn iris(&self) -> &SourceIris { &self.iris }

    pub fn out(&mut self) -> &mut GraphEmitter { &mut self.out }

    pub fn into_inner(self) -> GraphEmitter { self.out }

    /// Start a view (worksheet, CSV body, table ...).
    pub fn view(
        &mut self,
        number: ViewNumber,
        label: Option<&str>,
    ) -> ViewEmitter<'_> {
        let iris = self.iris.view(number);
        let view = iris.view();
        let source = self.iris.source();
        let out = &mut self.out;
        out.quad(view.clone(), rdf::TYPE, raw::VIEW.into_owned());
        out.quad(view.clone(), raw::IS_VIEW_IN, source.clone());
        out.quad(source, raw::HAS_VIEW, view.clone());
        out.quad(
            view.clone(),
            raw::VIEW_NUMBER,
            Literal::from(i64::from(number.0)),
        );
        if let Some(label) = label {
            out.quad(
                view,
                rdfs::LABEL,
                Literal::new_simple_literal(label),
            );
        }
        let profiles = self.profile.then(BTreeMap::new);
        ViewEmitter { iris, out, columns: 0, profiles }
    }
}

/// Emits columns, rows and values of one view.
#[derive(Debug)]
pub struct ViewEmitter<'a> {
    iris:     ViewIris,
    out:      &'a mut GraphEmitter,
    columns:  u32,
    profiles: Option<BTreeMap<u32, ColumnProfiler>>,
}

impl ViewEmitter<'_> {
    pub fn out(&mut self) -> &mut GraphEmitter { self.out }

    /// Declare a column, optionally with its header text as `rdfs:label`.
    pub fn column(&mut self, number: ColumnNumber, header: Option<&str>) {
        let col = self.iris.column(number);
        let view = self.iris.view();
        let out = &mut *self.out;
        out.quad(col.clone(), rdf::TYPE, raw::COLUMN.into_owned());
        out.quad(col.clone(), raw::IS_COLUMN_IN_VIEW, view.clone());
        out.quad(view, raw::HAS_COLUMN, col.clone());
        out.quad(
            col.clone(),
            raw::COLUMN_NUMBER,
            Literal::from(i64::from(number.0)),
        );
        if let Some(h) = header.filter(|h| !h.is_empty()) {
            out.quad(col, rdfs::LABEL, Literal::new_simple_literal(h));
        }
        self.columns = self.columns.max(number.0);
    }

    /// Attach a cell's formula (verbatim and R1C1) to its value.
    pub fn formula(
        &mut self,
        row: RowNumber,
        column: ColumnNumber,
        formula: &crate::formula::Formula,
        r1c1: &crate::formula::R1C1,
    ) {
        let value = self.iris.value(row, column);
        let out = &mut *self.out;
        out.quad(
            value.clone(),
            raw::FORMULA,
            Literal::new_simple_literal(formula.as_str()),
        );
        out.quad(
            value,
            raw::FORMULA_R1C1,
            Literal::new_simple_literal(r1c1.as_str()),
        );
    }

    /// Every formula in `column` has this one R1C1 form.
    pub fn column_formula(
        &mut self,
        column: ColumnNumber,
        r1c1: &crate::formula::R1C1,
    ) {
        let col = self.iris.column(column);
        self.out.quad(
            col,
            raw::FORMULA_R1C1,
            Literal::new_simple_literal(r1c1.as_str()),
        );
    }

    /// Emit the profile of every declared column (no-op without
    /// `--profile`).
    pub fn finish_profiles(&mut self) {
        let Some(mut profiles) = self.profiles.take() else {
            return
        };
        for c in 1..=self.columns {
            let profile = profiles.remove(&c).unwrap_or_default().finish();
            self.emit_profile(ColumnNumber(c), profile);
        }
    }

    fn emit_profile(&mut self, column: ColumnNumber, p: ColumnProfile) {
        let col = self.iris.column(column);
        let out = &mut *self.out;
        let int =
            |n: u64| Literal::new_typed_literal(n.to_string(), xsd::INTEGER);
        out.quad(col.clone(), raw::COUNT, int(p.count));
        out.quad(col.clone(), raw::NULL_COUNT, int(p.null_count));
        if let Some(n) = p.unique {
            out.quad(col.clone(), raw::UNIQUE, int(n));
        }
        if let Some((top, freq)) = p.top {
            out.quad(col.clone(), raw::TOP, top);
            out.quad(col.clone(), raw::FREQ, int(freq));
        }
        if let Some((datatype, n)) = p.datatype {
            out.quad(
                col.clone(),
                raw::INFERRED_DATATYPE,
                datatype.into_owned(),
            );
            out.quad(col.clone(), raw::CONFORMING_COUNT, int(n));
        }
        if let Some((min, max)) = p.extremes {
            out.quad(col.clone(), raw::MIN, min);
            out.quad(col.clone(), raw::MAX, max);
        }
        if let Some(m) = p.mean {
            out.quad(col.clone(), raw::MEAN, Literal::from(m));
        }
        if let Some(s) = p.std {
            out.quad(col.clone(), raw::STD, Literal::from(s));
        }
        if let Some([p25, p50, p75]) = p.percentiles {
            out.quad(col.clone(), raw::P25, Literal::from(p25));
            out.quad(col.clone(), raw::P50, Literal::from(p50));
            out.quad(col, raw::P75, Literal::from(p75));
        }
    }

    pub fn row(&mut self, number: RowNumber) {
        let row = self.iris.row(number);
        let out = &mut *self.out;
        out.quad(row.clone(), rdf::TYPE, raw::ROW.into_owned());
        out.quad(row.clone(), raw::IS_ROW_IN_VIEW, self.iris.view());
        out.quad(
            row,
            raw::VIEW_ROW_NUMBER,
            Literal::new_typed_literal(number.0.to_string(), xsd::INTEGER),
        );
    }

    /// Record one cell. Columns beyond the header are declared on the fly
    /// (ragged rows are data too).
    pub fn value(
        &mut self,
        row: RowNumber,
        column: ColumnNumber,
        cell: &CellValue,
    ) {
        if column.0 > self.columns {
            for c in self.columns + 1..=column.0 {
                self.column(ColumnNumber(c), None);
            }
        }
        let value = self.iris.value(row, column);
        let out = &mut *self.out;
        let class = match cell {
            CellValue::Null { .. } => raw::NULL_VALUE,
            _ => raw::VALUE,
        };
        out.quad(value.clone(), rdf::TYPE, class.into_owned());
        out.quad(value.clone(), raw::VALUE_OF, self.iris.row(row));
        out.quad(
            value.clone(),
            raw::FOR_COLUMN,
            self.iris.column(column),
        );
        let literal = cell.literal();
        if let Some(profiles) = &mut self.profiles {
            profiles
                .entry(column.0)
                .or_default()
                .observe(cell, literal.as_ref());
        }
        if let Some(lit) = literal {
            out.quad(value.clone(), raw::LITERAL_VALUE, lit);
        }
        if let CellValue::Null { source_text: Some(t) } = cell {
            out.quad(
                value,
                raw::SOURCE_TEXT,
                Literal::new_simple_literal(t),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::iri::ContentDigest};

    #[test]
    fn text_cell_stays_a_string() {
        let lit = CellValue::Text("00123".into()).literal().unwrap();
        assert_eq!(lit.value(), "00123");
        assert_eq!(lit.datatype(), xsd::STRING);
    }

    #[test]
    fn null_cell_has_no_literal_but_keeps_text() {
        let (tx, mut rx) = mpsc::channel(1);
        let iris = SourceIris::new(ContentDigest::new([1; 32]));
        let mut src = SourceEmitter::new(
            iris.clone(),
            GraphEmitter::new(iris.graph(), tx),
        );
        let mut v = src.view(ViewNumber(1), None);
        v.value(RowNumber(0), ColumnNumber(1), &CellValue::Null {
            source_text: Some("#N/A".into()),
        });
        src.out().flush_blocking().unwrap();
        let quads = rx.try_recv().unwrap();
        assert!(quads.iter().any(|q| q.predicate == raw::SOURCE_TEXT));
        assert!(!quads.iter().any(|q| q.predicate == raw::LITERAL_VALUE));
        assert!(
            quads
                .iter()
                .any(|q| q.object == raw::NULL_VALUE.into_owned().into())
        );
    }
}
