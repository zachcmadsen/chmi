use lexopt::prelude::*;

fn main() {
    let mut parser = lexopt::Parser::from_env();
    let Ok(Some(Value(command))) = parser.next() else {
        eprintln!("chmi: error: expected a command");
        return;
    };
    println!("command: {}", command.to_str().unwrap());
}
