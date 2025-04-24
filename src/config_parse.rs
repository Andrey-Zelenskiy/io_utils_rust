// Copyright Andrey Zelenskiy, 2024
use std::fs;

use std::path::Path;

use toml;

use serde_json;

use serde::Deserialize;

/* ------------------------------ */
/* Generic initialization methods */
/* ------------------------------ */
#[derive(Clone)]
pub enum Config {
    Toml(toml::Table),
    Json(serde_json::Value),
}

impl From<toml::Table> for Config {
    fn from(value: toml::Table) -> Self {
        Self::Toml(value)
    }
}

impl From<serde_json::Value> for Config {
    fn from(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

impl Config {
    // Initialize Config from strings
    pub fn from_toml_str(config_str: &str) -> Self {
        Self::from(config_str.parse::<toml::Table>().unwrap_or_else(|_| {
            panic!("Unable to parse toml config string {config_str}")
        }))
    }

    pub fn from_json_str(config_str: &str) -> Self {
        Self::from(
            serde_json::from_str::<serde_json::Value>(config_str)
                .unwrap_or_else(|_| {
                    panic!("Unable to parse json string {config_str}")
                }),
        )
    }
}

pub fn load_config(filename: &Path) -> Config {
    match filename.extension() {
        Some(extension) => {
            let extension_str = extension.to_str().unwrap();
            match extension_str {
                "toml" => load_toml(filename),
                "json" => load_json(filename),
                _ => panic!(
                    "Config files with .{extension_str} extension \
                                  are not supported."
                ),
            }
        }
        None => panic!(
            "No extension found for config file {}.",
            filename.to_str().unwrap()
        ),
    }
}

// Method to deserialize a config into the target structure
pub trait FromConfig: for<'a> Deserialize<'a> {
    fn from_config(config: &Config, table_name: &str) -> Self {
        match config {
            Config::Toml(config) => {
                match config[table_name].clone().try_into() {
                    Ok(value) => value,
                    Err(e) => panic!(
                        "Failed to initialize the structure for sub-table {table_name}: {e}"
                    ),
                }
            }
            Config::Json(config) => {
                match serde_json::from_value(config[table_name].clone()) {
                    Ok(value) => value,
                    Err(e) => panic!(
                            "Failed to initialize the structure for sub-table {table_name}: {e}"
                        ),
                }
            }
        }
    }
}

impl<T: for<'a> Deserialize<'a>> FromConfig for T {}

/* ---------------------------------------- */
/* Method for loading data from .toml files */
/* ---------------------------------------- */

// Open a config.toml file and save the data as a toml::Value
fn load_toml(filename: &Path) -> Config {
    // Read the contents of the file
    let contents = fs::read_to_string(filename).unwrap_or_else(|_| {
        panic!("Problem opening the file: {}", filename.to_str().unwrap())
    });

    // Save the data to toml::Table
    Config::Toml(contents.parse::<toml::Table>().unwrap_or_else(|_| {
        panic!(
            "{} should contain a table-type data.",
            filename.to_str().unwrap()
        )
    }))
}

/* ---------------------------------------- */
/* Method for loading data from .json files */
/* ---------------------------------------- */

// Open a config.toml file and save the data as a toml::Value
fn load_json(filename: &Path) -> Config {
    // Read the contents of the file
    let contents = fs::read_to_string(filename).unwrap_or_else(|_| {
        panic!("Problem opening the file: {}", filename.to_str().unwrap())
    });

    // Save the data to toml::Table
    Config::Json(serde_json::from_str(&contents).unwrap_or_else(|_| {
        panic!(
            "{} should contain a table-type data.",
            filename.to_str().unwrap()
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::{load_config, Config, Deserialize, FromConfig, Path};

    use std::io::Write;

    #[derive(Deserialize)]
    struct TestStruct {
        x: u32,
        y: u32,
        z: u32,
    }

    fn touch(path: &Path) {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .unwrap();
    }

    fn rm(path: &Path) {
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    #[should_panic]
    fn wrong_extension() {
        let path = Path::new("file_with_wrong_extension.dat");
        load_config(path);
    }

    mod toml_tests {
        use super::*;

        #[test]
        fn open_file() {
            // Create a new config file
            let path = Path::new("test.toml");
            touch(path);

            // Try to open the config file
            let _test_config = load_config(path);

            // Delete the config file
            rm(path);
        }

        #[test]
        #[should_panic]
        fn file_not_found() {
            let path = Path::new("this_file_doesnt_exist.toml");
            load_config(path);
        }

        #[test]
        fn sturct_from_config() {
            let config = Config::Toml(
                toml::from_str(
                    r#"
            [data]
            x = 1
            y = 2
            z = 3
            "#,
                )
                .unwrap(),
            );
            let test_struct = TestStruct::from_config(&config, "data");

            assert_eq!(test_struct.x, 1);
            assert_eq!(test_struct.y, 2);
            assert_eq!(test_struct.z, 3);
        }
    }

    mod json_tests {
        use super::*;

        #[test]
        fn open_file() {
            // Create a new config file
            let path = Path::new("test.json");
            touch(path);
            let mut file = std::fs::OpenOptions::new()
                .append(false)
                .write(true)
                .open(path)
                .unwrap();
            let contents = r#"{"message": "test", "file": "test.json"}"#;
            write!(file, "{contents}").unwrap();

            // Try to open the config file
            let _test_config = load_config(path);

            // Delete the config file
            rm(path);
        }

        #[test]
        #[should_panic]
        fn file_not_found() {
            let path = Path::new("this_file_doesnt_exist.json");
            load_config(path);
        }

        #[test]
        fn sturct_from_config() {
            let config = Config::Json(
                serde_json::from_str(
                    r#"
                {
                    "data": {
                        "x": 1, 
                        "y": 2,
                        "z": 3
                    }
                }"#,
                )
                .unwrap(),
            );
            let test_struct = TestStruct::from_config(&config, "data");

            assert_eq!(test_struct.x, 1);
            assert_eq!(test_struct.y, 2);
            assert_eq!(test_struct.z, 3);
        }
    }
}
