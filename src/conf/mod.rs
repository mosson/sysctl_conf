use std::{collections::HashMap, io::BufRead};

use serde::Serialize;
// cSpell:disable
use thiserror::Error;
// cSpell:enable

#[derive(Error, Debug)]
pub enum Error {
    #[error("`key = value` の書式を満たしていません: {0}")]
    InvalidKeyValuePair(String),
    #[error("{0}")]
    Unknown(String),
}

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
                                .and_modify(|v| *v = Box::new(Self::Value(value.to_string())))
                                .or_insert(Box::new(Self::Value(value.to_string())));
                        }
                    }
                } else {
                    match self {
                        Self::Value(_) => unreachable!("文字列のネスト差し替えが機能していない"),
                        Self::Nest(config) => {
                            config
                                .0
                                .entry(key.to_string())
                                .or_insert(Box::new(Self::Nest(Config::new())))
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
pub struct Config(HashMap<String, Box<Entry>>);

impl Config {
    pub fn parse<T: BufRead>(handle: T) -> Result<Self, Error> {
        let mut config = Self::new();

        for line in handle.lines() {
            let line = line.map_err(|e| Error::Unknown(e.to_string()))?;

            // Blank lines and lines that start with “#” or “;” are ignored.
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // If a line begins with a single “-”, a failing attempt to set the　value is ignored.
            // sysctl.conf そのものではないため許可リストが存在しないので対応しない。

            let pair = line
                .split("=")
                .take(2)
                .map(|s| s.trim())
                .collect::<Vec<_>>();

            if pair.len() != 2 {
                return Err(Error::InvalidKeyValuePair(line));
            }

            let (mut keys, value) = (pair[0].split('.').rev().collect::<Vec<_>>(), pair[1]);
            let key = keys.last().unwrap();

            let entry = config
                .0
                .entry(key.to_string())
                .and_modify(|v| {
                    if keys.len() <= 1 {
                        return;
                    }

                    if let Entry::Value(_) = **v {
                        *v = Box::new(Entry::Nest(Config::new()));
                    }
                })
                .or_insert_with(|| {
                    if keys.len() > 1 {
                        Box::new(Entry::Nest(Config::new()))
                    } else {
                        Box::new(Entry::Value(value.to_string()))
                    }
                });

            if let Entry::Nest(_) = **entry {
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
