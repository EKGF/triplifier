//! A triplifiable input file and its metadata (DCAT, SPDX checksum, git).

use {
    crate::{
        emit::GraphEmitter,
        git::GitOrigin,
        iri::{ContentDigest, SourceIris},
        vocab::{dcat, dcterms, prov, raw, rdf, spdx, xsd},
    },
    anyhow::Context,
    oxrdf::{Literal, NamedNode},
    sha1::Digest as _,
    std::path::{Path, PathBuf},
    tokio::io::AsyncReadExt,
};

/// Formats the triplifier can read. Add a variant to support a new format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Csv,
    Tsv,
    Xlsx,
    Xls,
    Ods,
}

impl SourceKind {
    pub const EXTENSIONS: &[&str] =
        &["csv", "tsv", "tab", "xlsx", "xlsm", "xlsb", "xls", "ods"];

    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "csv" => Self::Csv,
            "tsv" | "tab" => Self::Tsv,
            "xlsx" | "xlsm" | "xlsb" => Self::Xlsx,
            "xls" => Self::Xls,
            "ods" => Self::Ods,
            _ => return None,
        })
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv",
            Self::Tsv => "text/tab-separated-values",
            Self::Xlsx => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.\
                 sheet"
            },
            Self::Xls => "application/vnd.ms-excel",
            Self::Ods => "application/vnd.oasis.opendocument.spreadsheet",
        }
    }

    fn class(self) -> oxrdf::NamedNodeRef<'static> {
        match self {
            Self::Csv => raw::CSV_FILE,
            Self::Tsv => raw::TSV_FILE,
            Self::Xlsx => raw::XLSX_FILE,
            Self::Xls | Self::Ods => raw::FILE,
        }
    }
}

/// A file on disk, fingerprinted and ready to be read.
#[derive(Debug, Clone)]
pub struct Source {
    path:     PathBuf,
    kind:     SourceKind,
    size:     u64,
    modified: Option<chrono::DateTime<chrono::Utc>>,
    digest:   ContentDigest,
    git_blob: String,
    git:      Option<GitOrigin>,
}

impl Source {
    /// Fingerprint `path` in one streaming pass (SHA-256 + git blob SHA-1).
    pub async fn open(path: PathBuf, kind: SourceKind) -> anyhow::Result<Self> {
        let path = tokio::fs::canonicalize(&path)
            .await
            .with_context(|| format!("cannot resolve {}", path.display()))?;
        let meta = tokio::fs::metadata(&path).await?;
        let size = meta.len();
        let modified = meta.modified().ok().map(chrono::DateTime::from);

        let mut file = tokio::fs::File::open(&path).await?;
        let mut sha256 = sha2::Sha256::new();
        let mut blob = sha1::Sha1::new();
        blob.update(format!("blob {size}\0").as_bytes());
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            sha256.update(&buf[..n]);
            blob.update(&buf[..n]);
        }
        let digest = ContentDigest::new(sha256.finalize().into());
        let git_blob = hex::encode(blob.finalize());
        let git = GitOrigin::discover(path.clone()).await;
        Ok(Self { path, kind, size, modified, digest, git_blob, git })
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn kind(&self) -> SourceKind { self.kind }

    pub fn digest(&self) -> ContentDigest { self.digest }

    /// Emit the `raw:File` / `dcat:Distribution` description of this source.
    pub fn describe(&self, iris: &SourceIris, out: &mut GraphEmitter) {
        let s = iris.source();
        out.quad(
            s.clone(),
            rdf::TYPE,
            self.kind.class().into_owned(),
        );
        for class in [raw::FILE, raw::SOURCE, prov::ENTITY, dcat::DISTRIBUTION]
        {
            out.quad(s.clone(), rdf::TYPE, class.into_owned());
        }
        let name = self.path.file_name().map(|n| n.to_string_lossy());
        if let Some(name) = name {
            out.quad(
                s.clone(),
                raw::FILE_NAME,
                Literal::new_simple_literal(name.as_ref()),
            );
            out.quad(
                s.clone(),
                dcterms::TITLE,
                Literal::new_simple_literal(name.as_ref()),
            );
        }
        if let Ok(url) = url::Url::from_file_path(&self.path) {
            let url = NamedNode::new_unchecked(url.as_str());
            out.quad(s.clone(), dcat::DOWNLOAD_URL, url.clone());
            out.quad(s.clone(), prov::AT_LOCATION, url);
        }
        out.quad(
            s.clone(),
            dcat::MEDIA_TYPE,
            NamedNode::new_unchecked(format!(
                "https://www.iana.org/assignments/media-types/{}",
                self.kind.media_type()
            )),
        );
        out.quad(
            s.clone(),
            dcat::BYTE_SIZE,
            Literal::new_typed_literal(
                self.size.to_string(),
                xsd::NON_NEGATIVE_INTEGER,
            ),
        );
        if let Some(m) = self.modified {
            out.quad(
                s.clone(),
                dcterms::MODIFIED,
                Literal::new_typed_literal(m.to_rfc3339(), xsd::DATE_TIME),
            );
        }
        let ck = iris.checksum();
        out.quad(s.clone(), spdx::CHECKSUM, ck.clone());
        out.quad(
            ck.clone(),
            rdf::TYPE,
            spdx::CHECKSUM_CLASS.into_owned(),
        );
        out.quad(
            ck.clone(),
            spdx::ALGORITHM,
            spdx::SHA256.into_owned(),
        );
        out.quad(
            ck,
            spdx::CHECKSUM_VALUE,
            Literal::new_typed_literal(self.digest.hex(), xsd::HEX_BINARY),
        );
        out.quad(
            s.clone(),
            raw::GIT_BLOB,
            Literal::new_simple_literal(&self.git_blob),
        );
        if let Some(git) = &self.git {
            git.describe(&s, &self.git_blob, out);
        }
    }
}
