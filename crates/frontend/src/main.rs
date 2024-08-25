use std::process::ExitCode;

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
    /// Get a display's current input
    Get { display: String },
}

fn main() -> ExitCode {
    let args = Args::parse();

    match args.command {
        Command::List => {
            let displays = backend::get_display_names();
            for display in displays {
                println!("{display}");
            }
        }
        Command::Get { display } => match backend::get_input(&display) {
            Ok(_) => (),
            Err(err) => {
                eprintln!("chmi: error: {}", err);
                return ExitCode::FAILURE;
            }
        },
    };

    ExitCode::SUCCESS
}
