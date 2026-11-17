//! Deterministic IRI minting.
//!
//! Every IRI is derived from the SHA-256 digest of the source content and
//! physical positions inside it, so re-triplifying the same bytes yields the
//! same graph, wherever the file lives. No key columns, no slugs, no guessing.

use {oxrdf::NamedNode, std::fmt, thiserror::Error};

/// Base IRI under which all resources are minted. Must end in `/`, `#` or
/// `:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseIri(String);

#[derive(Debug, Error)]
#[error("base IRI `{0}` must be an absolute IRI ending in '/', '#' or ':'")]
pub struct InvalidBaseIri(String);

impl BaseIri {
    pub const DEFAULT: &str = "urn:ekgf:raw:";

    pub fn as_str(&self) -> &str { &self.0 }
}

impl Default for BaseIri {
    fn default() -> Self { Self(Self::DEFAULT.to_owned()) }
}

impl std::str::FromStr for BaseIri {
    type Err = InvalidBaseIri;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ok = matches!(s.chars().last(), Some('/' | '#' | ':')) &&
            NamedNode::new(s).is_ok();
        if ok {
            Ok(Self(s.to_owned()))
        } else {
            Err(InvalidBaseIri(s.into()))
        }
    }
}

/// SHA-256 of a source's bytes; the identity of a [`crate::source::Source`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentDigest([u8; 32]);

impl ContentDigest {
    pub fn new(bytes: [u8; 32]) -> Self { Self(bytes) }

    pub fn hex(&self) -> String { hex::encode(self.0) }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

/// 1-based position of a view (worksheet) within a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewNumber(pub u32);

/// 1-based physical column index, as in `raw:columnNumber`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnNumber(pub u32);

/// 0-based data-row sequence number, as in `raw:viewRowNumber`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowNumber(pub u64);

/// Mints the IRIs of one source and everything inside it.
#[derive(Debug, Clone)]
pub struct SourceIris {
    source: String,
}

impl SourceIris {
    pub fn new(base: &BaseIri, digest: ContentDigest) -> Self {
        Self {
            source: format!("{}source:{digest}", base.as_str()),
        }
    }

    fn node(s: String) -> NamedNode { NamedNode::new_unchecked(s) }

    pub fn source(&self) -> NamedNode { Self::node(self.source.clone()) }

    /// Named graph holding everything triplified from this source.
    pub fn graph(&self) -> NamedNode {
        Self::node(format!("{}:graph", self.source))
    }

    pub fn checksum(&self) -> NamedNode {
        Self::node(format!("{}:checksum", self.source))
    }

    pub fn view(&self, v: ViewNumber) -> ViewIris {
        ViewIris { view: format!("{}:view:{}", self.source, v.0) }
    }
}

/// Mints the IRIs inside one view.
#[derive(Debug, Clone)]
pub struct ViewIris {
    view: String,
}

impl ViewIris {
    pub fn view(&self) -> NamedNode { NamedNode::new_unchecked(&self.view) }

    pub fn column(&self, c: ColumnNumber) -> NamedNode {
        NamedNode::new_unchecked(format!("{}:column:{}", self.view, c.0))
    }

    pub fn row(&self, r: RowNumber) -> NamedNode {
        NamedNode::new_unchecked(format!("{}:row:{}", self.view, r.0))
    }

    pub fn value(&self, r: RowNumber, c: ColumnNumber) -> NamedNode {
        NamedNode::new_unchecked(format!(
            "{}:row:{}:column:{}",
            self.view, r.0, c.0
        ))
    }
}

/// IRI of one triplifier run (a `prov:Activity`).
#[derive(Debug, Clone)]
pub struct RunIri(NamedNode);

impl RunIri {
    pub fn new(base: &BaseIri, started: chrono::DateTime<chrono::Utc>) -> Self {
        Self(NamedNode::new_unchecked(format!(
            "{}run:{}",
            base.as_str(),
            started.format("%Y%m%dT%H%M%S%.6fZ")
        )))
    }

    pub fn node(&self) -> &NamedNode { &self.0 }

    pub fn graph(&self) -> NamedNode {
        NamedNode::new_unchecked(format!("{}:graph", self.0.as_str()))
    }

    pub fn agent(base: &BaseIri) -> NamedNode {
        NamedNode::new_unchecked(format!(
            "{}agent:triplifier:{}",
            base.as_str(),
            env!("CARGO_PKG_VERSION")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_must_end_in_separator() {
        assert!("https://example.org/raw".parse::<BaseIri>().is_err());
        assert!("https://example.org/raw/".parse::<BaseIri>().is_ok());
        assert!("not an iri/".parse::<BaseIri>().is_err());
    }

    #[test]
    fn value_iri_is_positional() {
        let s =
            SourceIris::new(&BaseIri::default(), ContentDigest::new([0; 32]));
        let v = s.view(ViewNumber(2));
        assert!(
            v.value(RowNumber(7), ColumnNumber(3))
                .as_str()
                .ends_with(":view:2:row:7:column:3")
        );
    }
}
