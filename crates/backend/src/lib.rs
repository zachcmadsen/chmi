use thiserror::Error;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unable to find display '{0}'")]
    DisplayNotFound(String),
}

pub fn get_display_names() -> Vec<String> {
    platform::get_display_names()
}

pub fn get_input(display_name: &str) -> Result<u8, Error> {
    platform::get_input(display_name)
}

pub fn set_input(display_name: &str, input: u8) {
    todo!()
}
