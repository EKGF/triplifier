//! Spreadsheet formulas as data.
//!
//! A formula is kept verbatim (A1 notation, as the workbook stores it) and
//! also normalised to R1C1 notation relative to its own cell. Filled-down
//! formulas (`=A2*B2`, `=A3*B3`, ...) share one R1C1 form (`RC[-2]*RC[-1]`),
//! which is what turns a column of formulas into a single derivation rule —
//! the starting point for generating SHACL rules/constraints later.

use std::fmt::Write as _;

/// A formula as stored in the workbook, without the leading `=`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula(String);

/// The R1C1 form of a [`Formula`], relative to the cell holding it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct R1C1(String);

impl Formula {
    pub fn new(text: impl Into<String>) -> Option<Self> {
        let text = text.into();
        let text = text.strip_prefix('=').map(str::to_owned).unwrap_or(text);
        (!text.trim().is_empty()).then_some(Self(text))
    }

    pub fn as_str(&self) -> &str { &self.0 }

    /// Rewrite every A1 cell reference to R1C1 relative to the 0-based
    /// physical cell `(row, col)`. String literals and quoted sheet names
    /// are copied untouched; function names (`LOG10(`) are not references.
    pub fn to_r1c1(&self, row: u64, col: u32) -> R1C1 {
        let s = self.0.as_bytes();
        let mut out = String::with_capacity(s.len() + 8);
        let mut i = 0;
        while i < s.len() {
            let b = s[i];
            if b == b'"' || b == b'\'' {
                let end = closing_quote(s, i);
                out.push_str(&self.0[i..end]);
                i = end;
                continue;
            }
            let boundary = i == 0 || !is_word(s[i - 1]);
            if boundary && let Some((len, r)) = cell_ref(&s[i..]) {
                r.write_r1c1(&mut out, row, col);
                i += len;
                continue;
            }
            // Copy one UTF-8 char.
            let ch_len = self.0[i..].chars().next().map_or(1, char::len_utf8);
            out.push_str(&self.0[i..i + ch_len]);
            i += ch_len;
        }
        R1C1(out)
    }
}

impl R1C1 {
    pub fn as_str(&self) -> &str { &self.0 }
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

/// Index just past the quote closing the one at `start` (`""` escapes).
fn closing_quote(s: &[u8], start: usize) -> usize {
    let q = s[start];
    let mut i = start + 1;
    while i < s.len() {
        if s[i] == q {
            if s.get(i + 1) == Some(&q) {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    s.len()
}

struct CellRef {
    col_abs: bool,
    col:     u32, // 0-based
    row_abs: bool,
    row:     u64, // 0-based
}

impl CellRef {
    fn write_r1c1(&self, out: &mut String, row: u64, col: u32) {
        out.push('R');
        if self.row_abs {
            let _ = write!(out, "{}", self.row + 1);
        } else if self.row != row {
            let _ = write!(out, "[{}]", self.row as i64 - row as i64);
        }
        out.push('C');
        if self.col_abs {
            let _ = write!(out, "{}", self.col + 1);
        } else if self.col != col {
            let _ = write!(out, "[{}]", i64::from(self.col) - i64::from(col));
        }
    }
}

/// Parse `$?[A-Z]{1,3}$?[0-9]+` at the start of `s`, not followed by a word
/// character or `(`. Returns the byte length and the reference.
fn cell_ref(s: &[u8]) -> Option<(usize, CellRef)> {
    let mut i = 0;
    let col_abs = s.first() == Some(&b'$');
    i += usize::from(col_abs);
    let letters = s[i..].iter().take_while(|b| b.is_ascii_uppercase()).count();
    if !(1..=3).contains(&letters) {
        return None;
    }
    let col = s[i..i + letters]
        .iter()
        .fold(0u32, |acc, b| acc * 26 + u32::from(b - b'A' + 1)) -
        1;
    i += letters;
    let row_abs = s.get(i) == Some(&b'$');
    i += usize::from(row_abs);
    let digits = s[i..].iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let row: u64 = std::str::from_utf8(&s[i..i + digits]).ok()?.parse().ok()?;
    i += digits;
    if row == 0 || s.get(i).is_some_and(|&b| is_word(b) || b == b'(') {
        return None;
    }
    Some((i, CellRef { col_abs, col, row_abs, row: row - 1 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r1c1(f: &str, row: u64, col: u32) -> String {
        Formula::new(f).unwrap().to_r1c1(row, col).0
    }

    #[test]
    fn filled_down_formulas_share_r1c1() {
        // C2 = A2*B2 and C3 = A3*B3 (0-based rows 1 and 2, column 2)
        assert_eq!(r1c1("=A2*B2", 1, 2), "RC[-2]*RC[-1]");
        assert_eq!(r1c1("A3*B3", 2, 2), "RC[-2]*RC[-1]");
    }

    #[test]
    fn absolute_ranges_strings_and_functions() {
        assert_eq!(r1c1("SUM($A$1:A1)", 4, 1), "SUM(R1C1:R[-4]C[-1])");
        assert_eq!(
            r1c1("IF(A1=\"B2\",LOG10(C1),'Sheet A1'!D4)", 0, 0),
            "IF(RC=\"B2\",LOG10(RC[2]),'Sheet A1'!R[3]C[3])"
        );
    }
}
