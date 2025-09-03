pub mod conf;

use std::io::{BufRead, BufReader};

use clap::Parser;

use crate::conf::Config;

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(clap::Parser, Debug)]
#[command(version = "0.1.0")]
#[command(about = "sysctl.conf like parser")]
#[command(long_about = None)]
pub struct AppConfig {
    #[arg(value_name = "FILE", default_value = "-")]
    file: String,
}

pub fn get_config() -> MyResult<AppConfig> {
    Ok(AppConfig::try_parse().map_err(|e| e.to_string())?)
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            std::fs::File::open(filename)
                .map_err(|e| format!("{}: {}", e.to_string(), filename))?,
        ))),
    }
}

pub fn run(config: AppConfig) -> MyResult<()> {
    let result = Config::parse(open(&config.file)?)?;

    println!("{}", serde_json::to_string(&result)?);

    Ok(())
}
