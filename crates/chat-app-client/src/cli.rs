use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(
        short = 's',
        long = "stdout",
        action,
        help = "Controls if it logs to stdout/stderr instead of to a file"
    )]
    pub is_to_std_out: bool,
}
