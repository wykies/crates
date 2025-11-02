use clap::Parser;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
#[command(
    author,
    version,
    about,
    long_about = "Monitors a bunyan formatted json log and reports errors"
)]
pub struct Cli {
    /// Print state only and exit
    #[arg(long, short)]
    pub print_state_only: bool,

    /// Specify the state file to use
    #[arg(
        default_value_t = String::from("state.ron")
    )]
    pub state_file: String,

    /// Create a new state file with the user supplied log folder
    #[arg(long)]
    pub init: Option<String>,

    /// Send a test notification
    #[arg(long)]
    pub test_notification: Option<String>,
}
