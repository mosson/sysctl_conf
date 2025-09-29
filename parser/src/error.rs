use crate::lexer::{self, token::Location};

#[derive(thiserror::Error, std::fmt::Debug)]
pub enum Error {
    #[error("{1:?}で文法エラーです:  {0}")]
    SyntaxError(String, Location),
    #[error("{0}")]
    LexerError(String),
}

impl From<lexer::error::Error> for Error {
    fn from(value: lexer::error::Error) -> Self {
        Self::LexerError(value.to_string())
    }
}

impl From<&lexer::error::Error> for Error {
    fn from(value: &lexer::error::Error) -> Self {
        Self::LexerError(value.to_string())
    }
}
