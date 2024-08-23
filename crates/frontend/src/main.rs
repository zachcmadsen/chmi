use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Set a display's input
    Set {
        /// The display name
        display: String,
        /// The input to switch to
        #[arg(value_enum)]
        input: Input,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum Input {
    DisplayPort1,
    DisplayPort2,
    Hdmi1,
    Hdmi2,
}

fn main() {
    let _args = Args::parse();
}
