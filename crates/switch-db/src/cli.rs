use super::Mode;
use clap::Parser;
use std::path::PathBuf;
use version_control_clean_check::CheckOptions;

/// Designed to switch between working on the different modes
///
/// - `Root` is expected to point to the root of the repo
/// - Makes changes immediately so might result in partial switch
/// - Splits the line based on the mark and comments out the line if the following text does not start with the mode or uncomments if it does
/// - Comments or Uncomment SQLX_OFFLINE depending on mode
#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(value_enum)]
    pub mode: Mode,

    /// Specify the root directory or uses current directory if not provided
    #[arg(value_name = "FOLDER", long, default_value = ".")]
    pub root: PathBuf,

    #[clap(flatten)]
    pub check_version_control: CheckOptions,

    /// Causes the program to fail if it needs to edit any files (copying is ok)
    ///
    /// This is intended for use in CI where we do not want to have dirty files
    /// in git but copying is done to an ignored folder and that is allowed and
    /// even desired as it doesn't exist in CI if not copied
    #[arg(long)]
    pub no_edit_only_copy: bool,
}
