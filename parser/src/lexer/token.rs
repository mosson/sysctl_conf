#[derive(Debug, PartialEq, Clone)]
pub struct Location {
    pub line: usize,
    pub position: std::ops::RangeInclusive<usize>,
}

#[derive(Debug, PartialEq)]
pub enum Type {
    Space,
    Return,
    Dot,
    Equal,
    Ignore,
    Comment,
    Ident(String),
    EOF,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub(crate) loc: Location,
    pub(crate) ty: Type,
}

impl Token {
    pub fn new(line: usize, position: std::ops::RangeInclusive<usize>, ty: Type) -> Self {
        Self {
            loc: Location { line, position },
            ty,
        }
    }
}
