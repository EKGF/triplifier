//! Fast, `.gitignore`-aware discovery of everything we can triplify.

use {
    crate::source::SourceKind,
    std::path::PathBuf,
    tokio::sync::mpsc,
    tracing::warn,
};

/// A file worth triplifying.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub kind: SourceKind,
}

/// Walks the given roots on a blocking thread pool and streams candidates.
#[derive(Debug, Clone)]
pub struct Walker {
    roots: Vec<PathBuf>,
}

impl Walker {
    pub fn new(roots: Vec<PathBuf>) -> Self { Self { roots } }

    pub fn spawn(self, capacity: usize) -> mpsc::Receiver<Candidate> {
        let (tx, rx) = mpsc::channel(capacity);
        tokio::task::spawn_blocking(move || self.walk(&tx));
        rx
    }

    fn walk(self, tx: &mpsc::Sender<Candidate>) {
        let Some((first, rest)) = self.roots.split_first() else {
            return
        };
        let mut builder = ignore::WalkBuilder::new(first);
        for root in rest {
            builder.add(root);
        }
        builder
            .hidden(true)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(true)
            .parents(true);
        builder.build_parallel().run(|| {
            let tx = tx.clone();
            Box::new(move |entry| {
                use ignore::WalkState::{Continue, Quit};
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("{e}");
                        return Continue;
                    },
                };
                if !entry.file_type().is_some_and(|t| t.is_file()) {
                    return Continue;
                }
                let path = entry.into_path();
                // Skip Excel lock files like `~$Book.xlsx`.
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("~$"))
                {
                    return Continue;
                }
                let Some(kind) = SourceKind::from_path(&path) else {
                    return Continue
                };
                if tx.blocking_send(Candidate { path, kind }).is_err() {
                    Quit
                } else {
                    Continue
                }
            })
        });
    }
}
