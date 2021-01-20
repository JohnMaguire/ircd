use serde::Deserialize;
use std::fs::read_to_string;
use toml::value::Datetime;

#[derive(Deserialize)]
pub struct Config {
    pub irc: Irc,
}

#[derive(Deserialize)]
pub struct Irc {
    pub hostname: String,
    pub created_at: Datetime,
}

pub fn get_config(path: &str) -> Result<Config, String> {
    let toml_config = read_to_string(path).or(Err(format!("Error opening file: {}", path)))?;
    let config: Config = toml::from_str(&toml_config)
        .or_else(|e| Err(format!("Error deserializing config file: {}", e)))?;

    Ok(config)
}
