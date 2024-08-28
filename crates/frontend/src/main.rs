use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Input {
    DisplayPort1,
    DisplayPort2,
    Hdmi1,
    Hdmi2,
}

impl From<Input> for u8 {
    fn from(value: Input) -> Self {
        match value {
            Input::DisplayPort1 => 0x0F,
            Input::DisplayPort2 => 0x10,
            Input::Hdmi1 => 0x11,
            Input::Hdmi2 => 0x12,
        }
    }
}

impl TryFrom<u8> for Input {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0F => Ok(Input::DisplayPort1),
            0x10 => Ok(Input::DisplayPort2),
            0x11 => Ok(Input::Hdmi1),
            0x12 => Ok(Input::Hdmi2),
            _ => Err(()),
        }
    }
}

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
    /// Get a display's input
    Get { display: String },
    /// Set a display's input
    Set { display: String, input: Input },
}

fn main() -> ExitCode {
    let args = Args::parse();

    match args.command {
        Command::List => match backend::get_display_names() {
            Ok(displays) => {
                for display in displays {
                    println!("{display}");
                }
            }
            Err(err) => {
                eprintln!("chmi: error: {}", err);
                return ExitCode::FAILURE;
            }
        },
        Command::Get { display } => match backend::get_input(&display) {
            Ok(value) => {
                println!(
                    "{}",
                    Input::try_from(value)
                        .unwrap()
                        .to_possible_value()
                        .unwrap()
                        .get_name()
                );
            }
            Err(err) => {
                eprintln!("chmi: error: {}", err);
                return ExitCode::FAILURE;
            }
        },
        Command::Set { display, input } => {
            backend::set_input(&display, input.into())
        }
    };

    ExitCode::SUCCESS
}
