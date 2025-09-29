use std::{collections::HashMap, io::BufRead};

use node::{Path, SchemaType, Statement, Value};
use parser::Parser;

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(clap::Parser, std::fmt::Debug)]
#[command(version = "0.1.0")]
#[command(about = "sysctl.conf parser")]
#[command(long_about = None)]
pub struct Config {
    #[arg(value_name = "FILE", default_value = "-")]
    file: String,
    #[arg(short, long, value_name = "SCHEMA_FILE")]
    schema_file: Option<String>,
}

fn main() -> AppResult<()> {
    let result = <Config as clap::Parser>::try_parse()
        .map_err(|e| e.into())
        .and_then(|config| {
            if config.file == "-"
                && config.schema_file.is_some()
                && config.schema_file.as_ref().unwrap() == "-"
            {
                Err("スキーマと入力ファイルの両方を標準入力にできません"
                    .to_string()
                    .into())
            } else {
                Ok(config)
            }
        })
        .and_then(run);

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run(config: Config) -> AppResult<()> {
    let reader = open(config.file.as_str())?;
    let mut parser: Parser<_, Value> = Parser::new(reader);
    let statements = parser.parse()?;

    let schema = match config.schema_file {
        Some(path) => {
            let mut parser = Parser::<_, SchemaType>::new(open(path.as_str())?);

            let schema = parser
                .parse()?
                .into_iter()
                .map(Statement::to_tuple)
                .collect::<HashMap<Path, SchemaType>>();

            Some(schema)
        }
        None => None,
    };

    let value = Statement::evaluate(statements, schema)?;

    println!("{}", value.format());

    Ok(())
}

fn open(filename: &str) -> AppResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(std::io::BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(std::io::BufReader::new(
            std::fs::File::open(filename).map_err(|e| format!("{}: {}", e, filename))?,
        ))),
    }
}
