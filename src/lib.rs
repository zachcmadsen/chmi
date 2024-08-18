mod cache;
mod cap;
mod monitor;
mod parse;

use cap::{Capabilities, Input};

pub trait Monitor2 {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &Capabilities;
    fn input(&self) -> anyhow::Result<Input>;
    fn set_input(&mut self, input: Input) -> anyhow::Result<()>;
}

pub fn get_monitors2() -> anyhow::Result<Vec<Box<dyn Monitor2>>> {
    let monitors = monitor::get_monitors()?;
    let mut new_monitors: Vec<Box<dyn Monitor2>> = Vec::new();

    for monitor in monitors {
        new_monitors.push(Box::new(monitor));
    }

    Ok(new_monitors)
}
