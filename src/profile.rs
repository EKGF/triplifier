//! Opt-in column profiling (`--profile`).
//!
//! Every measure is exact or absent: distinct-value tracking and percentile
//! samples are capped, and a measure whose cap was exceeded is not emitted.
//! The inferred datatype is column-level evidence only; captured values keep
//! their source literals.
//!
//! Capture policy:
//! - candidates, most specific first: `xsd:boolean`, `xsd:integer`,
//!   `xsd:decimal`, `xsd:double`, `xsd:date`, `xsd:dateTime`;
//! - the first candidate to which at least [`THRESHOLD_PERCENT`] of the
//!   non-null values conform is the inferred datatype;
//! - text conforms by strict XSD lexical form (surrounding whitespace ignored),
//!   except that integers with leading zeros (`00123`, an identifier pattern)
//!   do not conform to numeric types and only `true`/`false` are booleans;
//!   `INF`/`NaN` are not numbers here;
//! - spreadsheet cells conform by their stored type; a date-time at midnight
//!   conforms to `xsd:date` as well as `xsd:dateTime`.

use {
    crate::{emit::CellValue, vocab::xsd},
    chrono::{NaiveDate, NaiveDateTime, NaiveTime},
    oxrdf::{Literal, NamedNodeRef},
    std::collections::HashMap,
};

pub const THRESHOLD_PERCENT: u64 = 95;
pub const MAX_DISTINCT: usize = 100_000;
pub const MAX_SAMPLES: usize = 1_000_000;

/// Datatypes a column can be inferred to have, most specific first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Candidate {
    Boolean,
    Integer,
    Decimal,
    Double,
    Date,
    DateTime,
}

impl Candidate {
    const ALL: [Self; 6] = [
        Self::Boolean,
        Self::Integer,
        Self::Decimal,
        Self::Double,
        Self::Date,
        Self::DateTime,
    ];

    fn datatype(self) -> NamedNodeRef<'static> {
        match self {
            Self::Boolean => xsd::BOOLEAN,
            Self::Integer => xsd::INTEGER,
            Self::Decimal => xsd::DECIMAL,
            Self::Double => xsd::DOUBLE,
            Self::Date => xsd::DATE,
            Self::DateTime => xsd::DATE_TIME,
        }
    }

    fn index(self) -> usize { self as usize }
}

/// How one cell conforms to the candidates.
#[derive(Debug, Default)]
struct Reading {
    boolean:   Option<bool>,
    integer:   Option<i128>,
    /// Numeric value and decimal lexical form, when decimal-conforming.
    decimal:   Option<(f64, String)>,
    double:    Option<f64>,
    date:      Option<NaiveDate>,
    date_time: Option<NaiveDateTime>,
}

impl Reading {
    fn of(cell: &CellValue) -> Self {
        let mut r = Self::default();
        match cell {
            CellValue::Text(s) => r.read_text(s.trim()),
            CellValue::Integer(i) => {
                r.integer = Some(i128::from(*i));
                r.decimal = Some((*i as f64, i.to_string()));
                r.double = Some(*i as f64);
            },
            CellValue::Double(f) if f.is_finite() => {
                if f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0 {
                    r.integer = Some(*f as i128);
                }
                r.decimal = Some((*f, format!("{f}")));
                r.double = Some(*f);
            },
            CellValue::Boolean(b) => r.boolean = Some(*b),
            CellValue::DateTime(dt) => {
                r.date_time = Some(*dt);
                if dt.time() == NaiveTime::MIN {
                    r.date = Some(dt.date());
                }
            },
            CellValue::Decimal(s) => r.read_numeric_text(s),
            CellValue::Date(d) => r.date = Some(*d),
            CellValue::DateTimeUtc(dt) => r.date_time = Some(dt.naive_utc()),
            CellValue::Double(_) |
            CellValue::Duration(_) |
            CellValue::Time(_) |
            CellValue::Binary(_) |
            CellValue::Null { .. } => {},
        }
        r
    }

    /// Numeric conformance of a decimal lexical form.
    fn read_numeric_text(&mut self, s: &str) {
        if let Some(digits) = decimal_lexical(s) {
            let f: f64 = s.parse().unwrap_or(f64::NAN);
            if f.is_finite() {
                if digits.integer_only {
                    self.integer = s.parse::<i128>().ok();
                }
                self.decimal = Some((f, s.to_owned()));
                self.double = Some(f);
            }
        }
    }

    fn read_text(&mut self, s: &str) {
        match s {
            "true" => self.boolean = Some(true),
            "false" => self.boolean = Some(false),
            _ => {},
        }
        if let Some(digits) = decimal_lexical(s) {
            let f: f64 = s.parse().unwrap_or(f64::NAN);
            if f.is_finite() {
                if digits.integer_only {
                    self.integer = s.parse::<i128>().ok();
                }
                self.decimal = Some((f, s.to_owned()));
                self.double = Some(f);
            }
        } else if double_lexical(s) {
            self.double = s.parse::<f64>().ok().filter(|f| f.is_finite());
        }
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") &&
            s.len() == 10
        {
            self.date = Some(d);
        }
        self.date_time = parse_date_time(s);
    }

    fn conforms(&self, c: Candidate) -> bool {
        match c {
            Candidate::Boolean => self.boolean.is_some(),
            Candidate::Integer => self.integer.is_some(),
            Candidate::Decimal => self.decimal.is_some(),
            Candidate::Double => self.double.is_some(),
            Candidate::Date => self.date.is_some(),
            Candidate::DateTime => self.date_time.is_some(),
        }
    }
}

struct DecimalLexical {
    integer_only: bool,
}

/// `[+-]? digits ('.' digits?)? | [+-]? '.' digits`, rejecting leading zeros
/// in the integer part (other than a single `0`).
fn decimal_lexical(s: &str) -> Option<DecimalLexical> {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int, frac) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    let digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    if !digits(int) || !frac.is_none_or(digits) {
        return None;
    }
    if int.is_empty() && frac.is_none_or(str::is_empty) {
        return None;
    }
    if int.len() > 1 && int.starts_with('0') {
        return None;
    }
    Some(DecimalLexical { integer_only: frac.is_none() })
}

/// A decimal mantissa with an exponent, e.g. `1.5E3`.
fn double_lexical(s: &str) -> bool {
    match s.split_once(['e', 'E']) {
        Some((mantissa, exp)) => {
            let exp = exp.strip_prefix(['+', '-']).unwrap_or(exp);
            decimal_lexical(mantissa).is_some() &&
                !exp.is_empty() &&
                exp.bytes().all(|b| b.is_ascii_digit())
        },
        None => false,
    }
}

/// `YYYY-MM-DDThh:mm:ss[.f]` with optional `Z`/offset (normalised to UTC);
/// a space separator, as in SQL dumps, is accepted too.
fn parse_date_time(s: &str) -> Option<NaiveDateTime> {
    if s.len() < 19 {
        return None;
    }
    let normalised = if s.as_bytes()[10] == b' ' {
        let mut t = s.to_owned();
        t.replace_range(10..11, "T");
        t
    } else {
        s.to_owned()
    };
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&normalised) {
        return Some(dt.naive_utc());
    }
    NaiveDateTime::parse_from_str(&normalised, "%Y-%m-%dT%H:%M:%S%.f").ok()
}

/// Streaming statistics over one numeric candidate.
#[derive(Debug, Default, Clone)]
struct Moments {
    n:    u64,
    mean: f64,
    m2:   f64,
}

impl Moments {
    fn push(&mut self, x: f64) {
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / self.n as f64;
        self.m2 += delta * (x - self.mean);
    }

    fn std(&self) -> Option<f64> {
        (self.n >= 2).then(|| (self.m2 / (self.n - 1) as f64).sqrt())
    }
}

/// Least and greatest value seen, each with its literal.
#[derive(Debug, Clone)]
struct Extremes<K: PartialOrd> {
    min: Option<(K, Literal)>,
    max: Option<(K, Literal)>,
}

impl<K: PartialOrd + Clone> Default for Extremes<K> {
    fn default() -> Self { Self { min: None, max: None } }
}

impl<K: PartialOrd + Clone> Extremes<K> {
    fn push(&mut self, key: K, literal: impl FnOnce() -> Literal) {
        let below = self.min.as_ref().is_none_or(|(m, _)| key < *m);
        let above = self.max.as_ref().is_none_or(|(m, _)| key > *m);
        if below || above {
            let lit = literal();
            if below {
                self.min = Some((key.clone(), lit.clone()));
            }
            if above {
                self.max = Some((key, lit));
            }
        }
    }

    fn literals(&self) -> Option<(Literal, Literal)> {
        Some((
            self.min.as_ref()?.1.clone(),
            self.max.as_ref()?.1.clone(),
        ))
    }
}

/// One numeric sample, tagged with the candidates it conforms to.
#[derive(Debug, Clone, Copy)]
struct Sample {
    value:   f64,
    integer: bool,
    decimal: bool,
}

/// The profile of one column, fed one cell at a time.
#[derive(Debug, Default)]
pub struct ColumnProfiler {
    count:       u64,
    null_count:  u64,
    distinct:    Option<HashMap<Literal, u64>>,
    overflow:    bool,
    conforming:  [u64; 6],
    moments:     [Moments; 3],
    boolean:     Extremes<bool>,
    integer:     Extremes<i128>,
    decimal:     Extremes<f64>,
    double:      Extremes<f64>,
    date:        Extremes<NaiveDate>,
    date_time:   Extremes<NaiveDateTime>,
    samples:     Option<Vec<Sample>>,
    samples_cut: bool,
}

/// The finished measures of a column, ready to be emitted.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnProfile {
    pub count:       u64,
    pub null_count:  u64,
    pub unique:      Option<u64>,
    pub top:         Option<(Literal, u64)>,
    pub datatype:    Option<(NamedNodeRef<'static>, u64)>,
    pub extremes:    Option<(Literal, Literal)>,
    pub mean:        Option<f64>,
    pub std:         Option<f64>,
    pub percentiles: Option<[f64; 3]>,
}

impl ColumnProfiler {
    pub fn observe(&mut self, cell: &CellValue, literal: Option<&Literal>) {
        let Some(literal) = literal else {
            self.null_count += 1;
            return;
        };
        self.count += 1;
        self.track_distinct(literal);
        let r = Reading::of(cell);
        for c in Candidate::ALL {
            if r.conforms(c) {
                self.conforming[c.index()] += 1;
            }
        }
        if let Some(b) = r.boolean {
            self.boolean.push(b, || Literal::from(b));
        }
        if let Some(i) = r.integer {
            self.moments[0].push(i as f64);
            self.integer.push(i, || {
                Literal::new_typed_literal(i.to_string(), xsd::INTEGER)
            });
        }
        if let Some((f, lexical)) = &r.decimal {
            self.moments[1].push(*f);
            self.decimal.push(*f, || {
                Literal::new_typed_literal(lexical, xsd::DECIMAL)
            });
        }
        if let Some(f) = r.double {
            self.moments[2].push(f);
            self.double.push(f, || Literal::from(f));
            self.sample(Sample {
                value:   f,
                integer: r.integer.is_some(),
                decimal: r.decimal.is_some(),
            });
        }
        if let Some(d) = r.date {
            self.date.push(d, || {
                Literal::new_typed_literal(
                    d.format("%Y-%m-%d").to_string(),
                    xsd::DATE,
                )
            });
        }
        if let Some(dt) = r.date_time {
            self.date_time.push(dt, || {
                Literal::new_typed_literal(
                    dt.format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
                    xsd::DATE_TIME,
                )
            });
        }
    }

    fn track_distinct(&mut self, literal: &Literal) {
        if self.overflow {
            return;
        }
        let map = self.distinct.get_or_insert_with(HashMap::new);
        if let Some(n) = map.get_mut(literal) {
            *n += 1;
        } else if map.len() < MAX_DISTINCT {
            map.insert(literal.clone(), 1);
        } else {
            self.distinct = None;
            self.overflow = true;
        }
    }

    fn sample(&mut self, s: Sample) {
        if self.samples_cut {
            return;
        }
        let v = self.samples.get_or_insert_with(Vec::new);
        if v.len() < MAX_SAMPLES {
            v.push(s);
        } else {
            self.samples = None;
            self.samples_cut = true;
        }
    }

    fn inferred(&self) -> Option<Candidate> {
        if self.count == 0 {
            return None;
        }
        Candidate::ALL.into_iter().find(|c| {
            let n = self.conforming[c.index()];
            n > 0 && n * 100 >= self.count * THRESHOLD_PERCENT
        })
    }

    pub fn finish(self) -> ColumnProfile {
        let (unique, top) = match &self.distinct {
            Some(map) if !self.overflow => {
                let top = map
                    .iter()
                    .max_by(|(a, na), (b, nb)| {
                        na.cmp(nb)
                            .then_with(|| b.to_string().cmp(&a.to_string()))
                    })
                    .map(|(l, n)| (l.clone(), *n));
                (Some(map.len() as u64), top)
            },
            _ => (None, None),
        };
        let inferred = self.inferred();
        let datatype =
            inferred.map(|c| (c.datatype(), self.conforming[c.index()]));
        let extremes = inferred.and_then(|c| {
            match c {
                Candidate::Boolean => self.boolean.literals(),
                Candidate::Integer => self.integer.literals(),
                Candidate::Decimal => self.decimal.literals(),
                Candidate::Double => self.double.literals(),
                Candidate::Date => self.date.literals(),
                Candidate::DateTime => self.date_time.literals(),
            }
        });
        let numeric = match inferred {
            Some(Candidate::Integer) => Some(0),
            Some(Candidate::Decimal) => Some(1),
            Some(Candidate::Double) => Some(2),
            _ => None,
        };
        let moments = numeric.map(|i| &self.moments[i]);
        let percentiles = numeric.and_then(|i| {
            let mut v: Vec<f64> = self
                .samples
                .as_ref()?
                .iter()
                .filter(|s| {
                    match i {
                        0 => s.integer,
                        1 => s.decimal,
                        _ => true,
                    }
                })
                .map(|s| s.value)
                .collect();
            v.sort_by(f64::total_cmp);
            Some([0.25, 0.5, 0.75].map(|p| percentile(&v, p)))
        });
        ColumnProfile {
            count: self.count,
            null_count: self.null_count,
            unique,
            top,
            datatype,
            extremes,
            mean: moments.filter(|m| m.n > 0).map(|m| m.mean),
            std: moments.and_then(Moments::std),
            percentiles,
        }
    }
}

/// Hyndman and Fan type 7: linear interpolation between closest ranks.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(cells: &[CellValue]) -> ColumnProfile {
        let mut p = ColumnProfiler::default();
        for c in cells {
            p.observe(c, c.literal().as_ref());
        }
        p.finish()
    }

    fn text(s: &str) -> CellValue { CellValue::Text(s.into()) }

    #[test]
    fn mostly_dates_infer_xsd_date_with_evidence() {
        let mut cells: Vec<_> =
            (1..=28).map(|d| text(&format!("2026-02-{d:02}"))).collect();
        cells.push(text("unknown"));
        let mut p = profile(&cells);
        assert_eq!(p.datatype, Some((xsd::DATE, 28)));
        let (min, max) = p.extremes.take().unwrap();
        assert_eq!(
            (min.value(), max.value()),
            ("2026-02-01", "2026-02-28")
        );
        assert_eq!(min.datatype(), xsd::DATE);
        assert_eq!(p.mean, None);
    }

    #[test]
    fn below_threshold_infers_nothing() {
        let p = profile(&[text("1"), text("2"), text("x")]);
        assert_eq!(p.datatype, None);
        assert_eq!(p.extremes, None);
    }

    #[test]
    fn leading_zeros_are_identifiers_not_numbers() {
        let p = profile(&[text("00123"), text("00456")]);
        assert_eq!(p.datatype, None);
    }

    #[test]
    fn integers_get_exact_statistics() {
        let p = profile(&[text("1"), text("2"), text("3"), text("4")]);
        assert_eq!(p.datatype, Some((xsd::INTEGER, 4)));
        assert_eq!(p.mean, Some(2.5));
        assert!((p.std.unwrap() - 1.290_994_448_735_805_6).abs() < 1e-12);
        assert_eq!(p.percentiles, Some([1.75, 2.5, 3.25]));
        let (min, max) = p.extremes.unwrap();
        assert_eq!((min.value(), max.value()), ("1", "4"));
    }

    #[test]
    fn decimals_and_doubles_rank_after_integers() {
        let p = profile(&[text("1.5"), text("2"), text("-0.25")]);
        assert_eq!(p.datatype, Some((xsd::DECIMAL, 3)));
        let p = profile(&[text("1e3"), text("2.5E-1")]);
        assert_eq!(p.datatype, Some((xsd::DOUBLE, 2)));
    }

    #[test]
    fn zero_one_columns_are_integers_not_booleans() {
        let p = profile(&[text("0"), text("1"), text("1")]);
        assert_eq!(p.datatype, Some((xsd::INTEGER, 3)));
        let p = profile(&[text("true"), text("false")]);
        assert_eq!(p.datatype, Some((xsd::BOOLEAN, 2)));
    }

    #[test]
    fn sql_style_date_times_conform() {
        let p = profile(&[
            text("2026-11-17 18:46:00"),
            text("2026-11-18T09:00:00Z"),
        ]);
        assert_eq!(p.datatype, Some((xsd::DATE_TIME, 2)));
    }

    #[test]
    fn nulls_and_distinct_values_are_counted_exactly() {
        let p = profile(
            &[text("a"), text("b"), text("a"), CellValue::Null {
                source_text: Some("#N/A".into()),
            }],
        );
        assert_eq!((p.count, p.null_count, p.unique), (3, 1, Some(2)));
        let (top, freq) = p.top.unwrap();
        assert_eq!((top.value(), freq), ("a", 2));
    }

    #[test]
    fn top_ties_break_on_ntriples_order() {
        let p = profile(&[text("b"), text("a")]);
        assert_eq!(p.top.unwrap().0.value(), "a");
    }

    #[test]
    fn spreadsheet_midnight_conforms_to_date() {
        let dt = NaiveDate::from_ymd_opt(2026, 1, 2)
            .unwrap()
            .and_time(NaiveTime::MIN);
        let p = profile(&[CellValue::DateTime(dt)]);
        assert_eq!(p.datatype, Some((xsd::DATE, 1)));
    }
}
