use std::collections::{HashMap, VecDeque};

use crate::error::Error;

pub mod error;

#[derive(Debug, PartialEq)]
pub struct Statement<T = Value>(Path, T);

impl<T> Statement<T> {
    pub fn new(path: Path, value: T) -> Self {
        Self(path, value)
    }
}

impl Statement<Value> {
    pub fn evaluate(
        statements: Vec<Statement<Value>>,
        schema: Option<HashMap<Path, SchemaType>>,
    ) -> Result<Value, Error> {
        let mut result = Value::Object(HashMap::new());

        for Statement(mut path, value) in statements.into_iter() {
            let key = path.to_string();

            match schema.as_ref() {
                Some(schema) => match schema.get(&path) {
                    Some(schema_type) => {
                        value
                            .check(schema_type)
                            .map_err(|s| Error::MismatchedType(format!("`{}` は {}", key, s,)))?;
                    }
                    _ => {}
                },
                _ => {}
            }

            let mut cursor_object = &mut result;

            while let Some(fragment) = path.pop() {
                if path.last() {
                    match cursor_object {
                        Value::Object(object) => match object.entry(fragment) {
                            std::collections::hash_map::Entry::Occupied(mut entry) => {
                                *entry.get_mut() = value;
                            }
                            std::collections::hash_map::Entry::Vacant(vacant) => {
                                vacant.insert(value);
                            }
                        },
                        _ => return Err(Error::ObjectOverride(key)),
                    }

                    break;
                } else {
                    cursor_object = match cursor_object {
                        Value::Object(object) => object
                            .entry(fragment)
                            .or_insert(Value::Object(HashMap::new())),
                        _ => unreachable!("走査中に構築するオブジェクトの構造が壊れている"),
                    };
                }
            }
        }

        Ok(result)
    }
}

impl Statement<SchemaType> {
    pub fn to_tuple(self) -> (Path, SchemaType) {
        (self.0, self.1)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Path(VecDeque<String>);

impl Path {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    pub fn push(&mut self, fragment: String) {
        self.0.push_back(fragment);
    }

    pub fn pop(&mut self) -> Option<String> {
        self.0.pop_front()
    }

    pub fn last(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|f| format!("{}", f))
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl From<VecDeque<String>> for Path {
    fn from(value: VecDeque<String>) -> Self {
        Self(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Object(HashMap<String, Value>),
}

#[allow(dead_code)]
impl Value {
    pub fn format(&self) -> String {
        fn inner(value: &Value, level: usize) -> String {
            match value {
                Value::String(v) => format!("\"{}\"", v),
                Value::Number(v) => format!("{}", v),
                Value::Boolean(v) => format!("{}", v),
                Value::Object(object) => {
                    let mut output = String::new();
                    output.push_str("{\n");
                    output.push_str(
                        object
                            .iter()
                            .map(|(k, v)| {
                                format!(
                                    "{}\"{}\": {}",
                                    "  ".repeat(level + 1),
                                    k,
                                    inner(v, level + 1)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(",\n")
                            .as_str(),
                    );
                    output.push_str("\n");
                    output.push_str("  ".repeat(level).as_str());
                    output.push_str("}");
                    output
                }
            }
        }

        inner(self, 0)
    }

    fn check(&self, schema_type: &SchemaType) -> Result<(), String> {
        match (self, schema_type) {
            (Value::Boolean(_), SchemaType::Boolean) => Ok(()),
            (Value::String(_), SchemaType::String) => Ok(()),
            (Value::Number(_), SchemaType::Float) => Ok(()),
            (Value::Number(v), SchemaType::Integer) => match v.to_string().parse::<isize>() {
                Ok(_) => Ok(()),
                Err(_) => Err(format!(
                    "`{}` 型として指定されていますが `{}` は `{}` として解釈できません",
                    schema_type.format(),
                    self.format(),
                    schema_type.format()
                )),
            },
            _ => Err(format!(
                "`{}` 型として指定されていますが `{}` は `{}` として解釈できません",
                schema_type.format(),
                self.format(),
                schema_type.format()
            )),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        let s = &value[..];
        if let Some(v) = parse_number(s) {
            v
        } else if let Some(v) = parse_boolean(s) {
            v
        } else {
            Value::String(value)
        }
    }
}

fn parse_number(input: &str) -> Option<Value> {
    let mut value = String::new();
    let mut iter = input.chars();
    let first_letter = iter.next();

    if first_letter.is_none() {
        return None;
    }
    let first_letter = first_letter.unwrap();

    match first_letter {
        '-' | '1'..='9' | '0' | '.' | 'e' | 'E' => value.push(first_letter),
        _ => return None,
    }

    for c in iter {
        match c {
            '-' | '1'..='9' | '0' | '.' | 'e' | 'E' => value.push(c),
            _ => return None,
        }
    }

    match value.parse::<f64>() {
        Ok(v) => Some(Value::Number(v)),
        Err(_) => None,
    }
}

fn parse_boolean(input: &str) -> Option<Value> {
    match input {
        "true" => Some(Value::Boolean(true)),
        "false" => Some(Value::Boolean(false)),
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
pub enum SchemaType {
    Integer,
    Float,
    Boolean,
    String,
}

impl From<String> for SchemaType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "integer" => Self::Integer,
            "bool" => Self::Boolean,
            "float" => Self::Float,
            _ => Self::String,
        }
    }
}

impl SchemaType {
    fn format(&self) -> String {
        match self {
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "bool",
            _ => "string",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case("Hello, 世界", Value::String("Hello, 世界".into()))]
    #[case("42", Value::Number(42f64))]
    #[case("-123", Value::Number(-123f64))]
    #[case("3.14159", Value::Number(3.14159f64))]
    #[case("1.23e4", Value::Number(1.23e4f64))]
    #[case("true", Value::Boolean(true))]
    #[case("false", Value::Boolean(false))]
    #[case("null", Value::String("null".into()))]
    fn test_value_from(#[case] input: &str, #[case] expected: Value) {
        assert_eq!(Value::from(input.to_string()), expected);
    }

    #[rstest::rstest]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string()])),
                Value::from("123".to_string()),
            )
        ],
        Ok(
            Value::Object(HashMap::from([
                ("foo".to_string(), Value::Number(123f64))
            ]))
        )
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "bar".to_string()])),
                Value::from("123".to_string()),
            ),
        ],
        Ok(
            Value::Object(HashMap::from([
                (
                    "foo".to_string(),
                    Value::Object(HashMap::from([
                        ("bar".to_string(), Value::Number(123f64))
                    ]))
                )
            ]))
        )
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "bar".to_string()])),
                Value::from("123".to_string()),
            ),
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "baz".to_string()])),
                Value::from("456".to_string()),
            ),
        ],
        Ok(
            Value::Object(HashMap::from([
                (
                    "foo".to_string(),
                    Value::Object(HashMap::from([
                        ("bar".to_string(), Value::Number(123f64)),
                        ("baz".to_string(), Value::Number(456f64))
                    ]))
                )
            ]))
        )
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "bar".to_string()])),
                Value::from("123".to_string()),
            ),
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "baz".to_string()])),
                Value::from("456".to_string()),
            ),
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "hoge".to_string(), "fuga".to_string()])),
                Value::from("789".to_string()),
            ),
        ],
        Ok(
            Value::Object(HashMap::from([
                (
                    "foo".to_string(),
                    Value::Object(HashMap::from([
                        ("bar".to_string(), Value::Number(123f64)),
                        ("baz".to_string(), Value::Number(456f64)),
                        (
                            "hoge".to_string(),
                            Value::Object(HashMap::from([
                                ("fuga".to_string(), Value::Number(789f64))
                            ]))
                        ),
                    ]))
                )
            ]))
        )
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string()])),
                Value::from("456".to_string()),
            ),
            Statement::new(
                Path::from(VecDeque::from(["foo".to_string(), "bar".to_string()])),
                Value::from("123".to_string()),
            ),
        ],
        Err("値が割り当てられているキーにオブジェクトを再割り当てできません（foo.bar）".to_string())
    )]
    fn test_evaluate(#[case] input: Vec<Statement>, #[case] expected: Result<Value, String>) {
        let result = Statement::evaluate(input, None);

        if expected.is_ok() {
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected.unwrap());
        } else {
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().to_string(), expected.unwrap_err());
        }
    }

    #[rstest::rstest]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["endpoint".to_string()])),
                Value::from("localhost:3000".to_string()),
            ),
        ],
        Some(
            HashMap::from([
                (
                    Path::from(VecDeque::from(["endpoint".to_string()])),
                    SchemaType::String
                )
            ])
        ),
        Ok(
            Value::Object(HashMap::from([
                (
                    "endpoint".to_string(),
                    Value::String("localhost:3000".to_string())
                )
            ]))
        )
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["endpoint".to_string()])),
                Value::from("localhost:3000".to_string()),
            ),
        ],
        Some(
            HashMap::from([
                (
                    Path::from(VecDeque::from(["debug".to_string()])),
                    SchemaType::Boolean
                )
            ])
        ),
        Ok(
            Value::Object(HashMap::from([
                (
                    "endpoint".to_string(),
                    Value::String("localhost:3000".to_string())
                )
            ]))
        )
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["endpoint".to_string()])),
                Value::from("localhost:3000".to_string()),
            ),
        ],
        Some(
            HashMap::from([
                (
                    Path::from(VecDeque::from(["endpoint".to_string()])),
                    SchemaType::Integer
                )
            ])
        ),
        Err("`endpoint` は `integer` 型として指定されていますが `\"localhost:3000\"` は `integer` として解釈できません")
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["endpoint".to_string()])),
                Value::from("localhost:3000".to_string()),
            ),
        ],
        Some(
            HashMap::from([
                (
                    Path::from(VecDeque::from(["endpoint".to_string()])),
                    SchemaType::Boolean
                )
            ])
        ),
        Err("`endpoint` は `bool` 型として指定されていますが `\"localhost:3000\"` は `bool` として解釈できません")
    )]
    #[case(
        vec![
            Statement::new(
                Path::from(VecDeque::from(["log".to_string(), "file".to_string()])),
                Value::from("./var/log/file".to_string()),
            ),
        ],
        Some(
            HashMap::from([
                (
                    Path::from(VecDeque::from(["log".to_string(), "file".to_string()])),
                    SchemaType::Float
                )
            ])
        ),
        Err("`log.file` は `float` 型として指定されていますが `\"./var/log/file\"` は `float` として解釈できません")
    )]
    fn test_evaluate_with_schema(
        #[case] statements: Vec<Statement>,
        #[case] schema: Option<HashMap<Path, SchemaType>>,
        #[case] expected: Result<Value, &str>,
    ) {
        let result = Statement::evaluate(statements, schema);

        if expected.is_ok() {
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected.unwrap());
        } else {
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string().as_str(),
                expected.unwrap_err()
            );
        }
    }
}
