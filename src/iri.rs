//! Deterministic `urn:uuid:` identifiers.
//!
//! Every resource has a canonical [`ResourceName`]. Its IRI is the
//! name-based **UUIDv8 with SHA-256** of RFC 9562 (§6.5, Appendix B.2):
//!
//! ```text
//! digest = SHA-256( NAMESPACE.bytes ‖ UTF-8(name) )
//! uuid   = digest[0..16], version nibble := 8, variant bits := 0b10
//! iri    = "urn:uuid:" ‖ lowercase hyphenated uuid
//! ```
//!
//! `NAMESPACE` is itself derived with the same function:
//! `uuidv8_sha256(NameSpace_URL, "https://ekgf.org/ontology/raw#")`.
//!
//! Name grammar (ASCII, decimal integers without leading zeros, `<sha256>` =
//! 64 lowercase hex digits of the SHA-256 of the source bytes):
//!
//! ```text
//! source/<sha256>
//! source/<sha256>/graph
//! source/<sha256>/checksum
//! source/<sha256>/view/<v>                      v ≥ 1
//! source/<sha256>/view/<v>/column/<c>           c ≥ 1
//! source/<sha256>/view/<v>/row/<r>              r ≥ 0
//! source/<sha256>/view/<v>/row/<r>/column/<c>
//! agent/triplifier/<semver>
//! ```
//!
//! Anyone can recompute any IRI from the source bytes and a position. Runs
//! are events, not content, so they get a time-ordered UUIDv7 instead.

use {
    oxrdf::NamedNode,
    sha2::{Digest, Sha256},
    std::{fmt, sync::LazyLock},
    uuid::Uuid,
};

/// Name-based UUIDv8 using SHA-256, per RFC 9562 Appendix B.2.
pub fn uuid_v8_sha256(namespace: Uuid, name: &str) -> Uuid {
    let digest = Sha256::new()
        .chain_update(namespace.as_bytes())
        .chain_update(name.as_bytes())
        .finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::new_v8(bytes)
}

/// The triplifier's UUID namespace.
pub static NAMESPACE: LazyLock<Uuid> = LazyLock::new(|| {
    uuid_v8_sha256(Uuid::NAMESPACE_URL, crate::vocab::raw::NS)
});

/// Canonical name of a resource; see the module docs for the grammar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceName(String);

impl ResourceName {
    pub fn as_str(&self) -> &str { &self.0 }

    fn child(&self, segment: impl fmt::Display) -> Self {
        Self(format!("{}/{segment}", self.0))
    }

    pub fn uuid(&self) -> Uuid { uuid_v8_sha256(*NAMESPACE, &self.0) }

    pub fn iri(&self) -> NamedNode { urn(self.uuid()) }
}

fn urn(uuid: Uuid) -> NamedNode {
    NamedNode::new_unchecked(uuid.urn().to_string())
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
    name: ResourceName,
}

impl SourceIris {
    pub fn new(digest: ContentDigest) -> Self {
        Self { name: ResourceName(format!("source/{digest}")) }
    }

    pub fn source(&self) -> NamedNode { self.name.iri() }

    /// Named graph holding everything triplified from this source.
    pub fn graph(&self) -> NamedNode { self.name.child("graph").iri() }

    pub fn checksum(&self) -> NamedNode { self.name.child("checksum").iri() }

    pub fn view(&self, v: ViewNumber) -> ViewIris {
        ViewIris { name: self.name.child("view").child(v.0) }
    }
}

/// Mints the IRIs inside one view.
#[derive(Debug, Clone)]
pub struct ViewIris {
    name: ResourceName,
}

impl ViewIris {
    pub fn view(&self) -> NamedNode { self.name.iri() }

    fn row_name(&self, r: RowNumber) -> ResourceName {
        self.name.child("row").child(r.0)
    }

    pub fn column(&self, c: ColumnNumber) -> NamedNode {
        self.name.child("column").child(c.0).iri()
    }

    pub fn row(&self, r: RowNumber) -> NamedNode { self.row_name(r).iri() }

    pub fn value(&self, r: RowNumber, c: ColumnNumber) -> NamedNode {
        self.row_name(r).child("column").child(c.0).iri()
    }
}

/// One triplifier run (a `prov:Activity`), identified by a UUIDv7.
#[derive(Debug, Clone)]
pub struct RunIri {
    node:  NamedNode,
    graph: NamedNode,
}

impl RunIri {
    pub fn new() -> Self {
        let id = Uuid::now_v7();
        Self {
            node:  urn(id),
            graph: urn(uuid_v8_sha256(id, "graph")),
        }
    }

    pub fn node(&self) -> &NamedNode { &self.node }

    pub fn graph(&self) -> &NamedNode { &self.graph }

    pub fn agent() -> NamedNode {
        ResourceName(format!(
            "agent/triplifier/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .iri()
    }
}

impl Default for RunIri {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc9562_appendix_b2_test_vector() {
        assert_eq!(
            uuid_v8_sha256(Uuid::NAMESPACE_DNS, "www.example.com").to_string(),
            "5c146b14-3c52-8afd-938a-375d0df1fbf6"
        );
    }

    #[test]
    fn names_follow_the_grammar() {
        let s = SourceIris::new(ContentDigest::new([0xab; 32]));
        let v = s.view(ViewNumber(2));
        let name = v.row_name(RowNumber(7)).child("column").child(3);
        assert_eq!(
            name.as_str(),
            format!("source/{}/view/2/row/7/column/3", "ab".repeat(32))
        );
        assert_eq!(v.value(RowNumber(7), ColumnNumber(3)), name.iri());
        assert!(name.iri().as_str().starts_with("urn:uuid:"));
        assert_eq!(name.uuid().get_version_num(), 8);
    }
}
