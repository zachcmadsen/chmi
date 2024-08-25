use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List available displays
    List,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::List => {
            let displays = backend::get_display_names();
            for display in displays {
                println!("{display}");
            }
        }
    }
}
