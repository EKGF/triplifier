use {
    clap::Parser,
    std::path::PathBuf,
    triplifier::{Triplifier, writer::Output},
};

/// Turn every CSV/TSV/XLSX/XLS/ODS file under the given paths into raw RDF.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Files or directories to triplify (directories are walked,
    /// honouring .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,

    /// Output file: .nq, .nt (graphs merged) or .trig, optionally .gz.
    /// Defaults to N-Quads on stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Emit RDFC-1.0 canonical N-Quads/N-Triples (buffers the dataset in
    /// memory).
    #[arg(long)]
    canonical: bool,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::WarnLevel>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(
                        cli.verbose
                            .tracing_level()
                            .map_or("off", |l| l.as_str()),
                    )
                }),
        )
        .without_time()
        .with_target(false)
        .init();
    let output = match cli.output {
        Some(path) => Output::file(path, cli.canonical)?,
        None => Output::stdout(cli.canonical),
    };
    let summary = Triplifier::new(cli.paths, output).run().await?;
    tracing::info!(?summary, "done");
    anyhow::ensure!(
        summary.failed == 0,
        "{} source(s) failed",
        summary.failed
    );
    Ok(())
}
