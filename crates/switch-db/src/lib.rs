use anyhow::{bail, Context};
use clap::{Parser, ValueEnum};
use cli::Cli;
use std::{
    borrow::Cow,
    fmt::{Display, Write as _},
    fs,
    io::Write as _,
    path::Path,
};
use tracing::{debug, info};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};
use version_control_clean_check::check_version_control;

mod cli;

pub fn run() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    tracing_subscriber::registry()
        .with(fmt::layer().with_span_events(FmtSpan::ACTIVE))
        .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("warn")))
        .init();

    info!("Switching to {}", cli.mode);
    cli.root = cli.root.canonicalize()?;
    debug!(?cli);
    check_version_control(&cli.root, &cli.check_version_control)
        .context("failed version control check")?;
    switch_rust_analyzer(&cli).context("failed to switch rust analyzer")?;
    switch_sqlx(&cli).context("failed to switch sqlx")?;
    switch_sqlx_prepared_queries(&cli).context("failed to switch sqlx prepared queries")?;
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
}

impl FileType {
    fn to_comment_slice(&self) -> &'static str {
        match self {
            FileType::Json => "//",
            FileType::DotEnv => "#",
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

fn switch_rust_analyzer(cli: &Cli) -> anyhow::Result<()> {
    do_switch(
        cli.root.join(".vscode/settings.json"),
        &cli.mode,
        "Switch to ",
        FileType::Json.to_comment_slice(),
        cli.no_edit_only_copy,
    )
}

fn switch_sqlx(cli: &Cli) -> anyhow::Result<()> {
    do_switch(
        cli.root.join("crates/chat-app-server/.env"),
        &cli.mode,
        "Switch to ",
        FileType::DotEnv.to_comment_slice(),
        cli.no_edit_only_copy,
    )
}

/// Deletes and replaces the base sqlx folder with the appropriate source folder
///
/// NB: Expects only files that start with "query" and have a json extension
fn switch_sqlx_prepared_queries(cli: &Cli) -> anyhow::Result<()> {
    let Cli {
        root: path, mode, ..
    } = cli;
    let dst_folder_name = ".sqlx";
    let src_folder_name = format!("{dst_folder_name}_{mode}");
    let dst_path = path.join(dst_folder_name);
    if !dst_path.exists() {
        fs::create_dir(&dst_path).context("failed to create target folder: {dst_folder_name:?}")?;
    }
    let dst_path = dst_path.canonicalize().with_context(|| {
        format!("failed to canonicalize destination sqlx folder: {dst_folder_name:?}")
    })?;
    let src_path = path
        .join(&src_folder_name)
        .canonicalize()
        .with_context(|| {
            format!("failed to canonicalize source sqlx folder: {src_folder_name:?}")
        })?;

    // Empty out the base folder
    for file in
        fs::read_dir(&dst_path).with_context(|| format!("failed to read dir: {dst_path:?}"))?
    {
        let path = file.context("failed to read file")?.path();
        check_expected_query_filename(&path)?;
        fs::remove_file(&path).with_context(|| format!("failed to remove file: {path:?}"))?;
    }

    // Copy over the files from the src folder
    for file in
        fs::read_dir(&src_path).with_context(|| format!("failed to read dir: {src_path:?}"))?
    {
        let path = file.context("failed to read file")?.path();
        check_expected_query_filename(&path)?;
        fs::copy(
            &path,
            dst_path.join(
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
    should_block_changes: bool,
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
        if should_block_changes {
            bail!("Changes are disallowed but a file would have been edited: {path:?}");
        }
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
