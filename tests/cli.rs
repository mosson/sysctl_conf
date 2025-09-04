use assert_cmd::Command;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
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
    run(
        &["-s", "tests/inputs/schema.txt", "tests/inputs/output1.txt"],
        "tests/expected/output1.json",
    )
}

#[test]
fn example2() -> MyResult<()> {
    run(
        &["-s", "tests/inputs/schema.txt", "tests/inputs/output2.txt"],
        "tests/expected/output2.json",
    )
}

#[test]
fn double_stdin() -> MyResult<()> {
    let output = Command::cargo_bin(PRG)?
        .args(&["-s", "-", "-"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error_message = String::from_utf8(output.stderr).expect("invalid UTF-8");
    assert_eq!(
        error_message,
        "スキーマと入力ファイルの両方を標準入力にできません\n"
    );

    Ok(())
}

#[test]
fn undefined_schema() -> MyResult<()> {
    let output = Command::cargo_bin(PRG)?
        .write_stdin("")
        .args(&["-s", "-", "tests/inputs/output1.txt"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error_message = String::from_utf8(output.stderr).expect("invalid UTF-8");
    assert_eq!(error_message, "未定義のスキーマです: endpoint\n");

    Ok(())
}

#[test]
fn skip_undefined_schema() -> MyResult<()> {
    let output = Command::cargo_bin(PRG)?
        .write_stdin(
            r#"
            debug -> bool
            log.file -> string
        "#,
        )
        .args(&["-s", "-", "tests/inputs/output3.txt"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let expected: Value = json!({
        "debug": "true",
        "log": {
            "file": "/var/log/console.log"
        }
    });

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    let value: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value, expected);

    Ok(())
}

#[test]
fn type_error() -> MyResult<()> {
    let output = Command::cargo_bin(PRG)?
        .write_stdin(
            r#"
            endpoint -> bool
            debug -> bool
            log.file -> bool
            log.name -> bool
            retry -> bool
        "#,
        )
        .args(&["-s", "-", "tests/inputs/output1.txt"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error_message = String::from_utf8(output.stderr).expect("invalid UTF-8");
    assert_eq!(
        error_message,
        "スキーマ違反です: localhost:3000 は bool として解釈できません（provided string was not `true` or `false`）\n"
    );

    Ok(())
}

#[test]
fn skip_type_error() -> MyResult<()> {
    let output = Command::cargo_bin(PRG)?
        .write_stdin(
            r#"
            endpoint -> bool
            debug -> bool
            log.file -> string
        "#,
        )
        .args(&["-s", "-", "tests/inputs/output3.txt"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let expected: Value = json!({
        "debug": "true",
        "log": {
            "file": "/var/log/console.log"
        }
    });

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    let value: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value, expected);

    Ok(())
}
