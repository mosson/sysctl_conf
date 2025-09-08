pub mod error;
pub mod schema;

use std::{collections::HashMap, io::BufRead};

use serde::Serialize;

use crate::config::{error::Error, schema::Schema};

#[derive(Debug, Serialize)]
// jsonのキー名に出力させない
#[serde(untagged)]
#[allow(dead_code)]
enum Entry {
    Value(String),
    Nest(Config),
}

impl Entry {
    fn set(&mut self, keys: &mut Vec<&str>, value: &str) -> Result<(), Error> {
        match keys.pop() {
            None => {}
            Some(key) => {
                // last
                if keys.is_empty() {
                    match self {
                        Self::Value(v) => *v = value.to_string(),
                        Self::Nest(config) => {
                            config
                                .0
                                .entry(key.to_string())
                                .and_modify(|v| *v = Self::Value(value.to_string()))
                                .or_insert(Self::Value(value.to_string()));
                        }
                    }
                } else {
                    match self {
                        Self::Value(_) => unreachable!("文字列のネスト差し替えが機能していない"),
                        Self::Nest(config) => {
                            config
                                .0
                                .entry(key.to_string())
                                .or_insert(Self::Nest(Config::new()))
                                .set(keys, value)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Config(HashMap<String, Entry>);

impl Config {
    pub fn parse<T: BufRead>(handle: T, schema: Schema) -> Result<Self, Error> {
        let mut config = Self::new();

        for line in handle.lines() {
            let line = line
                .map_err(|e| Error::Unknown(e.to_string()))?
                .trim()
                .to_string();

            // Blank lines and lines that start with “#” or “;” are ignored.
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            let pair = line
                .split("=")
                .take(2)
                .map(|s| s.trim())
                .collect::<Vec<_>>();

            if pair.len() != 2 {
                return Err(Error::InvalidKeyValuePair(
                    line.to_string(),
                    "=".to_string(),
                ));
            }

            let (key, value) = (pair[0], pair[1]);

            let value_check = match schema.get(key) {
                Some(value_checker) => value_checker.check(value),
                None => {
                    // If a line begins with a single “-”, a failing attempt to set the　value is ignored.
                    if line.starts_with('-') {
                        continue;
                    } else {
                        return Err(Error::UndefinedSchema(key.to_string()));
                    }
                }
            };

            if let Err(e) = value_check {
                // If a line begins with a single “-”, a failing attempt to set the　value is ignored.
                if line.starts_with('-') {
                    continue;
                } else {
                    return Err(e);
                }
            }

            let mut keys = key.split('.').rev().collect::<Vec<_>>();
            let key = keys.last().unwrap();

            let entry = config
                .0
                .entry(key.to_string())
                .and_modify(|v| {
                    if keys.len() <= 1 {
                        return;
                    }

                    if let Entry::Value(_) = *v {
                        *v = Entry::Nest(Config::new());
                    }
                })
                .or_insert_with(|| {
                    if keys.len() > 1 {
                        Entry::Nest(Config::new())
                    } else {
                        Entry::Value(value.to_string())
                    }
                });

            if let Entry::Nest(_) = *entry {
                keys.pop();
            }

            entry.set(&mut keys, value)?;
        }

        Ok(config)
    }

    fn new() -> Self {
        Config(HashMap::new())
    }
}
