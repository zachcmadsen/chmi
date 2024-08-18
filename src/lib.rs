mod cache;
mod cap;
mod monitor;
mod parse;
mod windows;

use monitor::Monitor;

pub fn get_monitors() -> anyhow::Result<Vec<Box<dyn Monitor>>> {
    let monitors = windows::get_monitors()?;
    let mut new_monitors: Vec<Box<dyn Monitor>> = Vec::new();

    for monitor in monitors {
        new_monitors.push(Box::new(monitor));
    }

    Ok(new_monitors)
}
