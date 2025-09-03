use assert_cmd::Command;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::fs;

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

const PRG: &str = "skill-check";

fn run(args: &[&str], expected_file: &str) -> MyResult<()> {
    let expected: Value = serde_json::from_str(&fs::read_to_string(expected_file)?).unwrap();
    let output = Command::cargo_bin(PRG)?.args(args).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    let value: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value, expected);

    Ok(())
}

#[test]
fn example1() -> MyResult<()> {
    run(&["tests/inputs/output1.txt"], "tests/expected/output1.json")
}

#[test]
fn example2() -> MyResult<()> {
    run(&["tests/inputs/output2.txt"], "tests/expected/output2.json")
}
