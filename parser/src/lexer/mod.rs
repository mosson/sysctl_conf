use std::io::BufRead;

use crate::{
    char_reader::{self, CharReader},
    lexer::{
        error::Error,
        token::{Token, Type},
    },
};

pub mod error;
pub mod token;

pub struct Lexer<T>
where
    T: BufRead,
{
    reader: CharReader<T>,
    peeking: Option<Result<Token, Error>>,
}

impl<T> Lexer<T>
where
    T: BufRead,
{
    pub fn new(reader: T) -> Self {
        Self {
            reader: CharReader::new(reader),
            peeking: None,
        }
    }

    pub fn peek(&mut self) -> &Result<Token, Error> {
        if self.peeking.is_none() {
            self.peeking = Some(self.next());
        }

        self.peeking.as_ref().unwrap()
    }

    pub fn next(&mut self) -> Result<Token, Error> {
        if self.peeking.is_some() {
            return self.peeking.take().unwrap();
        }

        let result = self.reader.read();
        if let Err(char_reader::error::Error::EOF(line, pos)) = result {
            return Ok(Token::new(line, pos..=pos, Type::EOF));
        }
        let (c, line, pos) = result?;

        match c {
            ' ' | '\t' | '\r' => {
                let mut last_pos = pos;
                loop {
                    let peek_result = self.reader.peek();
                    if let Err(char_reader::error::Error::EOF(_, _)) = peek_result {
                        break;
                    }
                    let (peek_char, _, peek_pos) = peek_result?;

                    if let Some(Type::Space) = Self::resolve_token(peek_char, *peek_pos) {
                        let _ = std::mem::replace(&mut last_pos, *peek_pos);
                        self.reader.read()?;
                    } else {
                        break;
                    }
                }

                Ok(Token::new(line, pos..=last_pos, Type::Space))
            }
            '\n' => Ok(Token::new(line, pos..=pos, Type::Return)),
            '.' => Ok(Token::new(line, pos..=pos, Type::Dot)),
            '=' => Ok(Token::new(line, pos..=pos, Type::Equal)),
            '#' | ';' if pos == 1 => Ok(Token::new(line, pos..=pos, Type::Comment)),
            '-' if pos == 1 => Ok(Token::new(line, pos..=pos, Type::Ignore)),
            _ => {
                let mut last_pos = pos;
                let mut value = String::new();
                value.push(c);

                loop {
                    let peek_result = self.reader.peek();
                    if let Err(char_reader::error::Error::EOF(_, _)) = peek_result {
                        break;
                    }
                    let (peek_char, _, peek_pos) = peek_result?;

                    if let None = Self::resolve_token(peek_char, *peek_pos) {
                        value.push(*peek_char);
                        let _ = std::mem::replace(&mut last_pos, *peek_pos);
                        self.reader.read()?;

                        // `->` も `=` とみなす（confとschemaの解析処理を分けたくないため）
                        if value.as_str() == "->" {
                            return Ok(Token::new(line, pos..=last_pos, Type::Equal));
                        }
                    } else {
                        break;
                    }
                }

                Ok(Token::new(line, pos..=last_pos, Type::Ident(value)))
            }
        }
    }

    fn resolve_token(c: &char, pos: usize) -> Option<Type> {
        match c {
            ' ' | '\t' | '\r' => Some(Type::Space),
            '\n' => Some(Type::Return),
            '.' => Some(Type::Dot),
            '=' => Some(Type::Equal),
            '#' | ';' if pos == 1 => Some(Type::Comment),
            '-' if pos == 1 => Some(Type::Ignore),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[rstest::rstest]
    #[case("\n", vec![Token::new(1, 1..=1, Type::Return)])]
    #[case(" ", vec![Token::new(1, 1..=1, Type::Space)])]
    #[case("  ", vec![Token::new(1, 1..=2, Type::Space)])]
    #[case(" \t ", vec![Token::new(1, 1..=3, Type::Space)])]
    #[
        case(
            " \r \n   ",
            vec![
                Token::new(1, 1..=3, Type::Space),
                Token::new(1, 4..=4, Type::Return),
                Token::new(2, 1..=3, Type::Space),
            ]
        )
    ]
    #[case(".", vec![Token::new(1, 1..=1, Type::Dot)])]
    #[case("=", vec![Token::new(1, 1..=1, Type::Equal)])]
    #[case("#", vec![Token::new(1, 1..=1, Type::Comment)])]
    #[case(";", vec![Token::new(1, 1..=1, Type::Comment)])]
    #[case("abc", vec![Token::new(1, 1..=3, Type::Ident("abc".to_string()))])]
    #[
        case(
            "abc.def",
            vec![
                Token::new(1, 1..=3, Type::Ident("abc".to_string())),
                Token::new(1, 4..=4, Type::Dot),
                Token::new(1, 5..=7, Type::Ident("def".to_string())),
            ]
        )
    ]
    #[
        case(
            "net.ipv4.conf.default.rp_filter = 1\n",
            vec![
                Token::new(1, 1..=3, Type::Ident("net".to_string())),
                Token::new(1, 4..=4, Type::Dot),
                Token::new(1, 5..=8, Type::Ident("ipv4".to_string())),
                Token::new(1, 9..=9, Type::Dot),
                Token::new(1, 10..=13, Type::Ident("conf".to_string())),
                Token::new(1, 14..=14, Type::Dot),
                Token::new(1, 15..=21,Type::Ident("default".to_string())),
                Token::new(1, 22..=22, Type::Dot),
                Token::new(1, 23..=31, Type::Ident("rp_filter".to_string())),
                Token::new(1, 32..=32, Type::Space),
                Token::new(1, 33..=33, Type::Equal),
                Token::new(1, 34..=34, Type::Space),
                Token::new(1, 35..=35, Type::Ident("1".to_string())),
                Token::new(1, 36..=36, Type::Return),
            ],
        )
    ]
    #[
        case(
            "endpoint = localhost:3000\n# debug = true",
            vec![
                Token::new(1, 1..=8, Type::Ident("endpoint".to_string())),
                Token::new(1, 9..=9, Type::Space),
                Token::new(1, 10..=10, Type::Equal),
                Token::new(1, 11..=11, Type::Space),
                Token::new(1, 12..=25, Type::Ident("localhost:3000".to_string())),
                Token::new(1, 26..=26, Type::Return),
                Token::new(2, 1..=1, Type::Comment),
                Token::new(2, 2..=2, Type::Space),
                Token::new(2, 3..=7, Type::Ident("debug".to_string())),
                Token::new(2, 8..=8, Type::Space),
                Token::new(2, 9..=9, Type::Equal),
                Token::new(2, 10..=10, Type::Space),
                Token::new(2, 11..=14, Type::Ident("true".to_string())),
            ],
        )
    ]
    fn test_lexer(#[case] input: &str, #[case] expected: Vec<Token>) {
        let cursor = std::io::Cursor::new(input);
        let handle = std::io::BufReader::new(cursor);
        let mut lexer = Lexer::new(handle);

        for token in expected.into_iter() {
            let result = lexer.next();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), token);
        }

        let result = lexer.next();
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            Token {
                loc: _,
                ty: Type::EOF
            }
        ));
    }
}
