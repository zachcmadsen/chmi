use std::fs;

use anyhow::{anyhow, Context};
use directories::ProjectDirs;
use rusqlite::Connection;

// TODO: Make a cache struct?

struct CapabilitiesCacheEntry {
    device_id: String,
    capabilities_string: String,
}

fn get_connection() -> anyhow::Result<Connection> {
    let project_dirs = ProjectDirs::from("", "", "chmi")
        .ok_or(anyhow!("failed to compute the cache directory location"))?;

    fs::create_dir_all(project_dirs.cache_dir())
        .context("failed to create the cache directory")?;

    let cache_path = project_dirs.cache_dir().join("capabilities.db");
    let connection = Connection::open(cache_path)
        .context("failed to open a connection to the SQLite database")?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS capabilities (
                id                  INTEGER PRIMARY KEY,
                device_id           TEXT NOT NULL,
                capabilities_string TEXT NOT NULL
            )",
            (),
        )
        .context("failed to create the capabilities table")?;

    Ok(connection)
}

pub fn get(device_id: &str) -> anyhow::Result<Option<String>> {
    let connection = get_connection()?;

    let mut statement = connection
        .prepare("SELECT device_id, capabilities_string FROM capabilities")?;

    let entries = statement.query_map((), |row| {
        Ok(CapabilitiesCacheEntry {
            device_id: row.get(0)?,
            capabilities_string: row.get(1)?,
        })
    })?;

    let entry = entries
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.device_id == device_id)
        .map(|entry| entry.capabilities_string);

    Ok(entry)
}

pub fn set(device_id: &str, capabilities_string: &str) -> anyhow::Result<()> {
    let connection = get_connection()?;
    connection.execute(
        "INSERT INTO capabilities (device_id, capabilities_string) VALUES (?1, ?2)",
        (device_id, capabilities_string))?;
    Ok(())
}
