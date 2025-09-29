use crate::char_reader;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("")]
    EOF,
    #[error("{0}")]
    ReaderError(String),
}

impl From<char_reader::error::Error> for Error {
    fn from(e: char_reader::error::Error) -> Self {
        match e {
            char_reader::error::Error::EOF(_, _) => Self::EOF,
            _ => Self::ReaderError(e.to_string()),
        }
    }
}
