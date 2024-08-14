mod cache;
mod cap;
mod fmt;
mod monitor;
mod parse;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use argh::FromArgs;
use fmt::Formatter;
use tracing::{warn, Level};
use tracing_subscriber::FmtSubscriber;

// TODO: Add subcommands for "raw" mode
// TODO: Add an option to just try the window the terminal is on via MonitorFromWindow.
#[derive(FromArgs)]
#[argh(description = "chmi - change monitor input")]
struct Args {
    #[argh(switch, short = 'v', description = "use verbose output")]
    verbose: bool,

    #[argh(switch, description = "print version information")]
    version: bool,
}

fn get_choice(prompt: &str, choices: &[usize]) -> usize {
    let choices_string = choices
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>()
        .join("/");

    let choice: usize;
    loop {
        print!("{} ({}): ", prompt, choices_string);
        let _ = io::stdout().flush();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("reading from stdin should succeed");

        if let Ok(input) = input.trim().parse::<usize>() {
            if choices.contains(&input) {
                choice = input;
                break;
            }
        }
    }

    choice
}

fn main() -> ExitCode {
    let args: Args = argh::from_env();

    if args.version {
        // TODO: Print the commit hash too.
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if args.verbose {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .event_format(Formatter::new())
            .with_writer(io::stderr)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting the default global subscriber should succeed");
    }

    let mut monitors = match monitor::get_monitors() {
        Ok(monitors) => monitors,
        Err(_) => {
            // error!("failed to find monitors");
            return ExitCode::FAILURE;
        }
    };

    monitors.retain(|monitor| {
        let has_input_select = monitor.capabilities().supports_input_select();
        if !has_input_select {
            warn!(
                "ignoring monitor '{}' since it doesn't support input select",
                monitor.name()
            );
        }
        has_input_select
    });

    if monitors.is_empty() {
        eprintln!("chmi: unable to find a monitor, try `chmi --verbose` for more information");
        return ExitCode::SUCCESS;
    }

    let mut monitor_choices = Vec::new();
    for (i, monitor) in monitors.iter().enumerate() {
        monitor_choices.push(i + 1);
        println!("{}) {}", i + 1, monitor.name());
    }

    let monitor_choice = get_choice("Monitor", &monitor_choices);
    let monitor = &monitors[monitor_choice - 1];

    let curr_input = match monitor.input() {
        Ok(input) => input,
        Err(_) => {
            // error!("failed to detect the current input");
            return ExitCode::FAILURE;
        }
    };

    let inputs = monitor.capabilities().supported_inputs();

    let mut input_choices = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        input_choices.push(i + 1);

        if input == &curr_input {
            println!("{}) {} (*)", i + 1, input);
        } else {
            println!("{}) {}", i + 1, input);
        }
    }

    let input_choice = get_choice("Input", &input_choices);
    let input = &inputs[input_choice - 1];

    if let Err(_) = monitor.set_input(input) {
        // error!("failed to change input");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
