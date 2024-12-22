use anyhow::{bail, Context};
use clap::{Parser, ValueEnum};
use std::{
    borrow::Cow,
    fmt::{Display, Write as _},
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};
use tracing::{debug, info};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};
use version_control_clean_check::{check_version_control, CheckOptions};

/// Designed to switch between working on the different modes
///
/// - `Root` is expected to point to the root of the repo
/// - Makes changes immediately so might result in partial switch
/// - Splits the line based on the mark and comments out the line if the following text does not start with the mode or uncomments if it does
/// - Comments or Uncomment SQLX_OFFLINE depending on mode
#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
#[command(author, version, about)]
struct Cli {
    #[arg(value_enum)]
    mode: Mode,

    /// Specify the root directory or uses current directory if not provided
    #[arg(value_name = "FOLDER", long, default_value = ".")]
    root: PathBuf,

    #[clap(flatten)]
    check_version_control: CheckOptions,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(fmt::layer().with_span_events(FmtSpan::ACTIVE))
        .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("warn")))
        .init();

    info!("Switching to {}", cli.mode);
    let path = cli.root.canonicalize()?;
    info!(?path);
    check_version_control(&path, &cli.check_version_control)
        .context("failed version control check")?;
    switch_rust_analyzer(&path, &cli.mode).context("failed to switch rust analyzer")?;
    switch_sqlx(&path, &cli.mode).context("failed to switch sqlx")?;
    switch_port(&path, &cli.mode).context("failed to switch db port")?;
    switch_sqlx_prepared_queries(&path, &cli.mode)
        .context("failed to switch sqlx prepared queries")?;
    println!("Switch completed to: {}", cli.mode);
    Ok(())
}

#[derive(ValueEnum, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
enum Mode {
    #[default]
    Standalone,
    Shuttle,
}

enum FileType {
    Json,
    DotEnv,
    Toml,
}

impl FileType {
    fn to_comment_slice(&self) -> &'static str {
        match self {
            FileType::Json => "//",
            FileType::DotEnv | FileType::Toml => "#",
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::Standalone => "Standalone",
                Mode::Shuttle => "Shuttle",
            }
        )
    }
}

fn switch_rust_analyzer(path: &Path, mode: &Mode) -> anyhow::Result<()> {
    do_switch(
        path.join(".vscode/settings.json"),
        mode,
        "Switch to ",
        FileType::Json.to_comment_slice(),
    )
}

fn switch_sqlx(path: &Path, mode: &Mode) -> anyhow::Result<()> {
    do_switch(
        path.join("crates/chat-app-server/.env"),
        mode,
        "Switch to ",
        FileType::DotEnv.to_comment_slice(),
    )
}

fn switch_port(path: &Path, mode: &Mode) -> anyhow::Result<()> {
    do_switch(
        path.join("crates/chat-app-server/configuration/base.toml"),
        mode,
        "Switch to ",
        FileType::Toml.to_comment_slice(),
    )
}

/// Deletes and replaces the base sqlx folder with the appropriate source folder
///
/// NB: Expects only files that start with "query" and have a json extension
fn switch_sqlx_prepared_queries(path: &Path, mode: &Mode) -> anyhow::Result<()> {
    let dest_folder_name = ".sqlx";
    let source_folder_name = format!("{dest_folder_name}_{mode}");
    let dest_path = path
        .join(dest_folder_name)
        .canonicalize()
        .with_context(|| {
            format!("failed to canonicalize destination sqlx folder: {dest_folder_name:?}")
        })?;
    let source_path = path
        .join(&source_folder_name)
        .canonicalize()
        .with_context(|| {
            format!("failed to canonicalize source sqlx folder: {source_folder_name:?}")
        })?;

    // Empty out the base folder
    for file in
        fs::read_dir(&dest_path).with_context(|| format!("failed to read dir: {dest_path:?}"))?
    {
        let path = file.context("failed to read file")?.path();
        check_expected_query_filename(&path)?;
        fs::remove_file(&path).with_context(|| format!("failed to remove file: {path:?}"))?;
    }

    // Copy over the files from the source folder
    for file in fs::read_dir(&source_path)
        .with_context(|| format!("failed to read dir: {source_path:?}"))?
    {
        let path = file.context("failed to read file")?.path();
        check_expected_query_filename(&path)?;
        fs::copy(
            &path,
            dest_path.join(
                path.file_name()
                    .context("no filename? how did it reach here?")?,
            ),
        )
        .with_context(|| format!("failed to remove file: {path:?}"))?;
    }

    Ok(())
}

fn check_expected_query_filename(path: &Path) -> anyhow::Result<()> {
    if !path.is_file() {
        bail!("Only expected files but found something else at {path:?}");
    }
    if !path
        .file_name()
        .with_context(|| format!("found a 'file' with no filename? -> {path:?}"))?
        .to_string_lossy()
        .starts_with("query")
    {
        bail!("expected all the query files to start with 'query' but found: {path:?}");
    }
    if path.extension().map(|x| x.to_str().unwrap_or_default()) != Some("json") {
        bail!("only expected json query files but found: {path:?}");
    }
    Ok(())
}

fn do_switch<P: std::fmt::Debug + AsRef<Path>>(
    path: P,
    mode: &Mode,
    mark: &str,
    comment: &str,
) -> anyhow::Result<()> {
    let mut changed = false;
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read file contents of: {path:?}"))?;
    let mut output = String::with_capacity(contents.len());
    for (i, line) in contents.lines().enumerate() {
        let i = i + 1;
        let mut parts = line.split(mark);
        let new_line = match parts.nth(1) {
            Some(after_marker) => {
                let should_be_uncommented = after_marker.starts_with(&mode.to_string());
                ensure_line_commenting(comment, &mut changed, line, should_be_uncommented, i)
            }
            None => {
                // Leave line unchanged
                Cow::Borrowed(line)
            }
        };
        writeln!(output, "{new_line}").expect("memory for string should already be allocated");
    }
    if changed {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .with_context(|| format!("failed to open file for writing: {path:?}"))?;
        file.write_all(output.as_bytes())
            .with_context(|| format!("failed to write changes to: {path:?}"))?;
        info!("Changes written to: {path:?}");
    } else {
        debug!("No changes to: {path:?}");
    }
    Ok(())
}

fn ensure_line_commenting<'a>(
    comment: &str,
    changed: &mut bool,
    line: &'a str,
    should_be_uncommented: bool,
    line_number: usize,
) -> Cow<'a, str> {
    if should_be_uncommented {
        // Uncomment line (if not already done)
        if line.trim().starts_with(comment) {
            *changed = true;
            debug!("Uncommented line: {line_number}");
            Cow::Owned(line.trim()[comment.len()..].trim().to_string())
        } else {
            // Already uncommented
            Cow::Borrowed(line)
        }
    } else {
        // Comment line (if not already done)
        if line.trim().starts_with(comment) {
            // Already commented
            Cow::Borrowed(line)
        } else {
            *changed = true;
            debug!("Commented out line: {line_number}");
            Cow::Owned(format!("{comment} {}", line.trim()))
        }
    }
}
