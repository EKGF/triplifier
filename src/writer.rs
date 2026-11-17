//! The single writer task: serializes quad batches as they arrive.
//!
//! Format follows the output file name: `.nq`, `.nt`, `.trig`, each
//! optionally followed by `.gz`. `.nt` writes the union of all graphs.
//! With `canonical`, output is RDFC-1.0 canonical N-Quads/N-Triples; that
//! needs the whole dataset in memory, everything else streams.

use {
    crate::emit::QuadBatch,
    anyhow::Context,
    async_compression::tokio::write::GzipEncoder,
    oxrdf::{
        Dataset,
        GraphName,
        Quad,
        graph::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm},
    },
    oxrdfio::{RdfFormat, RdfSerializer},
    std::{path::PathBuf, pin::Pin},
    tokio::{
        io::{AsyncWrite, AsyncWriteExt, BufWriter},
        sync::mpsc,
    },
};

const PREFIXES: &[(&str, &str)] = &[
    ("raw", crate::vocab::raw::NS),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("prov", "http://www.w3.org/ns/prov#"),
    ("dcat", "http://www.w3.org/ns/dcat#"),
    ("dcterms", "http://purl.org/dc/terms/"),
    ("spdx", "http://spdx.org/rdf/terms#"),
];

type Sink = Pin<Box<dyn AsyncWrite + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    NTriples,
    NQuads,
    TriG,
}

impl Syntax {
    fn format(self) -> RdfFormat {
        match self {
            Self::NTriples => RdfFormat::NTriples,
            Self::NQuads => RdfFormat::NQuads,
            Self::TriG => RdfFormat::TriG,
        }
    }

    fn keeps_graphs(self) -> bool { self != Self::NTriples }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Stdout,
    File(PathBuf),
}

/// Fully resolved output: where, which syntax, compressed, canonical.
#[derive(Debug, Clone)]
pub struct Output {
    target:    Target,
    syntax:    Syntax,
    gzip:      bool,
    canonical: bool,
}

impl Output {
    pub fn stdout(canonical: bool) -> Self {
        Self {
            target: Target::Stdout,
            syntax: Syntax::NQuads,
            gzip: false,
            canonical,
        }
    }

    pub fn file(path: PathBuf, canonical: bool) -> anyhow::Result<Self> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("output path has no file name")?
            .to_ascii_lowercase();
        let (stem, gzip) = match name.strip_suffix(".gz") {
            Some(stem) => (stem.to_owned(), true),
            None => (name, false),
        };
        let syntax = match stem.rsplit_once('.').map(|(_, e)| e) {
            Some("nq") => Syntax::NQuads,
            Some("nt") => Syntax::NTriples,
            Some("trig") => Syntax::TriG,
            other => {
                anyhow::bail!(
                    "unsupported output extension {other:?}; use .nq, .nt or \
                     .trig (optionally .gz)"
                )
            },
        };
        anyhow::ensure!(
            !canonical || syntax != Syntax::TriG,
            "RDFC-1.0 canonical output is N-Quads or N-Triples, not TriG"
        );
        Ok(Self {
            target: Target::File(path),
            syntax,
            gzip,
            canonical,
        })
    }

    async fn sink(&self) -> anyhow::Result<Sink> {
        let raw: Sink = match &self.target {
            Target::Stdout => Box::pin(tokio::io::stdout()),
            Target::File(p) => {
                Box::pin(BufWriter::with_capacity(
                    1 << 20,
                    tokio::fs::File::create(p).await.with_context(|| {
                        format!("cannot create {}", p.display())
                    })?,
                ))
            },
        };
        Ok(if self.gzip {
            Box::pin(GzipEncoder::new(raw))
        } else {
            raw
        })
    }

    fn adapt(&self, quad: Quad) -> Quad {
        if self.syntax.keeps_graphs() {
            quad
        } else {
            Quad { graph_name: GraphName::DefaultGraph, ..quad }
        }
    }

    /// Consume all batches; returns the number of quads received.
    pub async fn run(
        self,
        rx: mpsc::Receiver<QuadBatch>,
    ) -> anyhow::Result<u64> {
        let mut sink = self.sink().await?;
        let count = if self.canonical {
            self.write_canonical(&mut sink, rx).await?
        } else {
            self.write_streaming(&mut sink, rx).await?
        };
        sink.shutdown().await?;
        Ok(count)
    }

    async fn write_streaming(
        &self,
        sink: &mut Sink,
        mut rx: mpsc::Receiver<QuadBatch>,
    ) -> anyhow::Result<u64> {
        let mut ser = RdfSerializer::from_format(self.syntax.format());
        for (prefix, ns) in PREFIXES {
            ser = ser.with_prefix(*prefix, *ns)?;
        }
        let mut ser = ser.for_tokio_async_writer(sink);
        let mut count = 0u64;
        while let Some(batch) = rx.recv().await {
            count += batch.len() as u64;
            for quad in batch {
                ser.serialize_quad(&self.adapt(quad)).await?;
            }
        }
        ser.finish().await?;
        Ok(count)
    }

    async fn write_canonical(
        &self,
        sink: &mut Sink,
        mut rx: mpsc::Receiver<QuadBatch>,
    ) -> anyhow::Result<u64> {
        let mut dataset = Dataset::new();
        let mut count = 0u64;
        while let Some(batch) = rx.recv().await {
            count += batch.len() as u64;
            for quad in batch {
                dataset.insert(&self.adapt(quad));
            }
        }
        dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
            hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
        });
        let mut lines: Vec<String> = dataset
            .iter()
            .map(|q| {
                let mut s = q.to_string();
                s.push_str(" .\n");
                s
            })
            .collect();
        lines.sort_unstable();
        for line in lines {
            sink.write_all(line.as_bytes()).await?;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions() {
        let o = Output::file("x/out.nt.gz".into(), false).unwrap();
        assert_eq!((o.syntax, o.gzip), (Syntax::NTriples, true));
        let o = Output::file("out.NQ".into(), true).unwrap();
        assert_eq!((o.syntax, o.gzip), (Syntax::NQuads, false));
        assert!(Output::file("out.trig".into(), true).is_err());
        assert!(Output::file("out.ttl".into(), false).is_err());
    }
}
