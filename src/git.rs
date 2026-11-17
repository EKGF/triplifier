//! Git provenance of a source file: which repo, which commit, which path, and
//! whether the bytes we read are what that commit holds.

use {
    crate::vocab::{prov, raw},
    oxrdf::{Literal, NamedNode},
    std::path::PathBuf,
    tracing::debug,
};

#[derive(Debug, Clone)]
pub struct GitOrigin {
    /// Fetch URL of the default remote, verbatim (scp-style URLs aren't IRIs).
    remote:    Option<String>,
    commit:    String,
    /// Path relative to the work tree, `/`-separated.
    path:      String,
    /// Blob id of `path` in `HEAD`, `None` when untracked.
    head_blob: Option<String>,
}

impl GitOrigin {
    /// Best effort: a file outside any repository simply has no git origin.
    pub async fn discover(file: PathBuf) -> Option<Self> {
        tokio::task::spawn_blocking(move || {
            match Self::discover_blocking(&file) {
                Ok(origin) => origin,
                Err(e) => {
                    debug!("no git origin for {}: {e:#}", file.display());
                    None
                },
            }
        })
        .await
        .ok()
        .flatten()
    }

    fn discover_blocking(
        file: &std::path::Path,
    ) -> anyhow::Result<Option<Self>> {
        let Some(dir) = file.parent() else {
            return Ok(None)
        };
        let repo = gix::discover(dir)?;
        let Some(workdir) = repo.workdir() else {
            return Ok(None)
        };
        let workdir = std::fs::canonicalize(workdir)?;
        let rel = file.strip_prefix(&workdir)?;
        let path = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let commit = repo.head_commit()?;
        let head_blob = commit
            .tree()?
            .lookup_entry_by_path(rel)?
            .map(|e| e.object_id().to_string());
        let remote = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .and_then(Result::ok)
            .and_then(|r| {
                r.url(gix::remote::Direction::Fetch)
                    .map(|u| u.to_bstring().to_string())
            });
        Ok(Some(Self {
            remote,
            commit: commit.id.to_string(),
            path,
            head_blob,
        }))
    }

    pub fn describe(
        &self,
        source: &NamedNode,
        working_blob: &str,
        out: &mut crate::emit::GraphEmitter,
    ) {
        let lit = Literal::new_simple_literal;
        if let Some(remote) = &self.remote {
            out.quad(source.clone(), raw::GIT_REPOSITORY, lit(remote));
            if let Ok(iri) = NamedNode::new(remote) {
                out.quad(source.clone(), prov::WAS_DERIVED_FROM, iri);
            }
        }
        out.quad(source.clone(), raw::GIT_COMMIT, lit(&self.commit));
        out.quad(source.clone(), raw::GIT_PATH, lit(&self.path));
        let modified = self.head_blob.as_deref() != Some(working_blob);
        out.quad(
            source.clone(),
            raw::GIT_MODIFIED,
            Literal::from(modified),
        );
    }
}
