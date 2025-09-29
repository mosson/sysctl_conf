use std::marker::PhantomData;

use node::{Path, Statement, Value};

use crate::{
    error::Error,
    lexer::{
        Lexer,
        token::{Token, Type},
    },
};

pub mod char_reader;
pub mod error;
mod lexer;

pub struct Parser<T, U = Value>
where
    T: std::io::BufRead,
    U: From<String>,
{
    lexer: Lexer<T>,
    ignore: bool,
    _marker: PhantomData<U>,
}

#[allow(dead_code)]
impl<T, U> Parser<T, U>
where
    T: std::io::BufRead,
    U: From<String>,
{
    pub fn new(reader: T) -> Self {
        Self {
            lexer: lexer::Lexer::new(reader),
            ignore: false,
            _marker: PhantomData,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement<U>>, Error> {
        let mut statements = vec![];

        loop {
            match self.lexer.peek().as_ref()? {
                Token {
                    loc: _,
                    ty: Type::EOF,
                } => break,
                Token {
                    loc: _,
                    ty: Type::Ident(_),
                } => statements.push(self.parse_statement()?),
                Token {
                    loc,
                    ty: Type::Ignore,
                } => {
                    match self.ignore {
                        true => {
                            return Err(Error::SyntaxError(
                                "Ignoreが複数回指定されています。".into(),
                                loc.clone(),
                            ));
                        }
                        false => {
                            self.ignore = true;
                            self.lexer.next()?;
                            continue;
                        }
                    };
                }
                Token {
                    loc: _,
                    ty: Type::Comment,
                } => {
                    self.read_until_line_end()?;
                }
                Token {
                    loc: _,
                    ty: Type::Space,
                } => {
                    self.lexer.next()?;
                    continue;
                }
                Token {
                    loc: _,
                    ty: Type::Return,
                } => {
                    self.ignore = false;
                    self.lexer.next()?;
                    continue;
                }
                Token { loc, ty: _ } => {
                    return Err(Error::SyntaxError(
                        "行頭はコメントか識別子かIgnoreのみ認められています".into(),
                        loc.clone(),
                    ));
                }
            }
        }

        Ok(statements.into_iter().filter_map(|v| v).collect())
    }

    fn read_until_line_end(&mut self) -> Result<(), Error> {
        loop {
            match self.lexer.next()? {
                Token {
                    loc: _,
                    ty: Type::Return,
                }
                | Token {
                    loc: _,
                    ty: Type::EOF,
                } => break,
                _ => continue,
            };
        }

        self.ignore = false;

        Ok(())
    }

    fn parse_statement(&mut self) -> Result<Option<Statement<U>>, Error> {
        let path = match self.parse_key() {
            Err(Error::SyntaxError(s, l)) => {
                if self.ignore {
                    match self.lexer.peek() {
                        Ok(Token { loc, ty: _ }) => {
                            if loc.position.start().cmp(&1).is_ne() {
                                self.read_until_line_end()?;
                            }
                        }
                        _ => {}
                    }
                    self.ignore = false;
                    return Ok(None);
                } else {
                    return Err(Error::SyntaxError(s, l));
                }
            }
            Err(e) => Err(e),
            Ok(v) => Ok(v),
        }?;

        let value = match self.parse_value() {
            Err(Error::SyntaxError(s, l)) => {
                if self.ignore {
                    match self.lexer.peek() {
                        Ok(Token { loc, ty: _ }) => {
                            if loc.position.start().cmp(&1).is_ne() {
                                self.read_until_line_end()?;
                            }
                        }
                        _ => {}
                    }
                    self.ignore = false;
                    return Ok(None);
                } else {
                    return Err(Error::SyntaxError(s, l));
                }
            }
            Err(e) => Err(e),
            Ok(v) => Ok(v),
        }?;

        Ok(Some(Statement::new(path, value)))
    }

    fn parse_key(&mut self) -> Result<Path, Error> {
        let mut path = Path::new();
        match self.lexer.next()? {
            Token {
                loc: _,
                ty: Type::Ident(value),
            } => path.push(value),
            _ => unreachable!("peekと内容が違う"),
        };
        let mut value_phase = false;

        loop {
            match self.lexer.peek().as_ref()? {
                Token {
                    loc: _,
                    ty: Type::Dot,
                } => {
                    if value_phase {
                        break;
                    } else {
                        self.lexer.next()?;
                        continue;
                    }
                }
                Token {
                    loc: _,
                    ty: Type::Ident(_),
                } => {
                    if value_phase {
                        break;
                    } else {
                        match self.lexer.next()? {
                            Token {
                                loc: _,
                                ty: Type::Ident(value),
                            } => {
                                path.push(value);
                            }
                            _ => unreachable!("peek結果と異なる"),
                        }
                        continue;
                    }
                }
                Token {
                    loc: _,
                    ty: Type::Space,
                }
                | Token {
                    loc: _,
                    ty: Type::Equal,
                } => {
                    value_phase = true;
                    self.lexer.next()?;
                    continue;
                }
                _ => match self.lexer.next()? {
                    Token { loc, ty: _ } => {
                        return Err(Error::SyntaxError(
                            "キーの読み出しに失敗しました。".into(),
                            loc,
                        ));
                    }
                },
            }
        }

        Ok(path)
    }

    fn parse_value(&mut self) -> Result<U, Error> {
        let mut total_value = match self.lexer.next()? {
            Token {
                loc: _,
                ty: Type::Ident(value),
            } => value,
            Token {
                loc: _,
                ty: Type::Dot,
            } => ".".to_string(),
            Token { loc, ty: _ } => {
                return Err(Error::SyntaxError(
                    "値は識別子以外を指定できません".into(),
                    loc,
                ));
            }
        };

        loop {
            match self.lexer.next()? {
                Token {
                    loc: _,
                    ty: Type::Space,
                } => {
                    total_value.push(' ');
                    continue;
                }
                Token {
                    loc: _,
                    ty: Type::Dot,
                } => {
                    total_value.push('.');
                    continue;
                }
                Token {
                    loc: _,
                    ty: Type::Ident(value),
                } => {
                    total_value.push_str(value.as_str());
                    continue;
                }
                Token {
                    loc: _,
                    ty: Type::Return,
                }
                | Token {
                    loc: _,
                    ty: Type::EOF,
                } => {
                    self.ignore = false;
                    break Ok(U::from(total_value.trim().to_string()));
                }
                Token { loc, ty: _ } => {
                    break Err(Error::SyntaxError(
                        "値の後は改行か末尾しか認められません".into(),
                        loc,
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use node::SchemaType;
    use pretty_assertions::assert_eq;

    #[rstest::rstest]
    #[
        case(
            "endpoint = localhost:3000",
            Ok(
                vec![
                    Statement::new(
                        Path::from(VecDeque::from(vec!["endpoint".to_string()])),
                        Value::from("localhost:3000".to_string()),
                    )
                ]
            )
        )
    ]
    #[
        case(
            "debug = true",
            Ok(
                vec![
                    Statement::new(
                        Path::from(VecDeque::from(vec!["debug".to_string()])),
                        Value::from("true".to_string()),
                    )
                ]
            )
        )
    ]
    #[
        case(
            "log.file = /var/log/console.log",
            Ok(
                vec![
                    Statement::new(
                        Path::from(VecDeque::from(vec!["log".to_string(), "file".to_string()])),
                        Value::from("/var/log/console.log".to_string()),
                    )
                ]
            )
        )
    ]
    #[
        case(
            "# debug = true",
            Ok(vec![])
        )
    ]
    #[
        case(
            "- debug = true",
            Ok(
                vec![
                    Statement::new(
                        Path::from(VecDeque::from(vec!["debug".to_string()])),
                        Value::from("true".to_string()),
                    )
                ]
            )
        )
    ]
    #[
        case(
            "- debug =",
            Ok(vec![])
        )
    ]
    #[
        case(
            "debug =",
            Err("Location { line: 1, position: 7..=7 }で文法エラーです:  キーの読み出しに失敗しました。".to_string())
        )
    ]
    #[
        case(
            "- debug =\n- debug2 = true\n- debug3 =\n-debug4 = false",
            Ok(
                vec![
                    Statement::new(
                        Path::from(VecDeque::from(vec!["debug2".to_string()])),
                        Value::from("true".to_string()),
                    ),
                    Statement::new(
                        Path::from(VecDeque::from(vec!["debug4".to_string()])),
                        Value::from("false".to_string()),
                    )
                ]
            )
        )
    ]
    #[
        case(
            "endpoint = localhost:3000\n# debug = true\nlog.file = /var/log/console.log\nlog.name = default.log",
            Ok(
                vec![
                    Statement::new(
                        Path::from(VecDeque::from(vec!["endpoint".to_string()])),
                        Value::from("localhost:3000".to_string()),
                    ),
                    Statement::new(
                        Path::from(VecDeque::from(vec!["log".to_string(), "file".to_string()])),
                        Value::from("/var/log/console.log".to_string()),
                    ),
                    Statement::new(
                        Path::from(VecDeque::from(vec!["log".to_string(), "name".to_string()])),
                        Value::from("default.log".to_string()),
                    )
                ]
            )
        )
    ]
    fn test_parse(#[case] input: &str, #[case] expected: Result<Vec<Statement>, String>) {
        let cursor = std::io::Cursor::new(input);
        let reader = std::io::BufReader::new(cursor);
        let mut parser = Parser::new(reader);

        let result = parser.parse();
        if expected.is_ok() {
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected.unwrap());
        } else {
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().to_string(), expected.unwrap_err());
        }

        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn test_parse_schema() {
        let input = r#"endpoint -> string
            debug -> bool
            log.file -> string
            log.name -> string
            retry -> integer
            num -> float
        "#
        .trim();
        let expected = vec![
            Statement::new(
                Path::from(VecDeque::from(["endpoint".to_string()])),
                SchemaType::String,
            ),
            Statement::new(
                Path::from(VecDeque::from(["debug".to_string()])),
                SchemaType::Boolean,
            ),
            Statement::new(
                Path::from(VecDeque::from(["log".to_string(), "file".to_string()])),
                SchemaType::String,
            ),
            Statement::new(
                Path::from(VecDeque::from(["log".to_string(), "name".to_string()])),
                SchemaType::String,
            ),
            Statement::new(
                Path::from(VecDeque::from(["retry".to_string()])),
                SchemaType::Integer,
            ),
            Statement::new(
                Path::from(VecDeque::from(["num".to_string()])),
                SchemaType::Float,
            ),
        ];

        let cursor = std::io::Cursor::new(input);
        let reader = std::io::BufReader::new(cursor);
        let mut parser = Parser::<_, SchemaType>::new(reader);

        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }
}
