pub mod config;

use std::io::{BufRead, BufReader};

use clap::Parser;

use crate::config::{Config, schema::Schema};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(clap::Parser, Debug)]
#[command(version = "0.1.0")]
#[command(about = "sysctl.conf like parser")]
#[command(long_about = None)]
pub struct AppConfig {
    #[arg(value_name = "FILE", default_value = "-")]
    file: String,
    #[arg(short, long, required = true, value_name = "SCHEMA_FILE")]
    schema_file: String,
}

pub fn get_config() -> MyResult<AppConfig> {
    let result = AppConfig::try_parse()
        .map_err(|e| e.to_string())
        .and_then(|config| {
            if config.file == "-" && config.schema_file == "-" {
                Err("スキーマと入力ファイルの両方を標準入力にできません".to_string())
            } else {
                Ok(config)
            }
        });

    result.map_err(|e| e.into())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            std::fs::File::open(filename).map_err(|e| format!("{}: {}", e, filename))?,
        ))),
    }
}

pub fn run(config: AppConfig) -> MyResult<()> {
    let schema = Schema::parse(open(&config.schema_file)?)?;
    let result = Config::parse(open(&config.file)?, schema)?;

    println!("{}", serde_json::to_string(&result)?);

    Ok(())
}
