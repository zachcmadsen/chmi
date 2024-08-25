#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

pub fn get_display_names() -> Vec<String> {
    platform::get_display_names()
}
