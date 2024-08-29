use thiserror::Error;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unable to find display '{0}'")]
    DisplayNotFound(String),
    #[error("unexpected OS error, try '--verbose' for more information")]
    Os,
}

pub fn get_display_names() -> Result<Vec<String>, Error> {
    platform::get_display_names()
}

pub fn get_input(display_name: &str) -> Result<u8, Error> {
    platform::get_input(display_name)
}

// TODO: Check that the input actually changed after setting it?
pub fn set_input(_display_name: &str, _input: u8) {
    todo!()
}
