/// std::io::BufRead からの読み出し時のエラーを表現する
pub mod error;

use crate::char_reader::error::Error;

/// 引数の std::io::BufRead から UTF-8 で１文字ずつ読み出すReader
/// utf8_char_width が nightly 、使えればそちらを利用するほうが良い
///
/// # Examples
///
/// ```
/// let source = r#"こんにちわ、World🫠"#;
/// let cursor = std::io::Cursor::new(source);
/// let handle = std::io::BufReader::new(cursor);
/// let mut char_reader = crate::parser::char_reader::CharReader::new(handle);
///
/// for (i, want) in source.chars().enumerate() {
///     let got = char_reader.read();
///
///     assert!(got.is_ok());
///
///     let(char, line, pos) = got.unwrap();
///     assert_eq!(want, char);
///     assert_eq!(line, 1);
///     assert_eq!(pos, i + 1);
///  }
/// ```
#[derive(std::fmt::Debug)]
pub struct CharReader<T>
where
    T: std::io::BufRead,
{
    reader: T,
    line: usize,
    position: usize,
    peek_buffer: std::collections::VecDeque<(char, usize, usize)>,
    peek_offset: usize,
}

#[allow(dead_code)]
impl<T> CharReader<T>
where
    T: std::io::BufRead,
{
    /// Reader を生成して返却する
    /// position は UTF-8 の文字数を表す
    /// 1文字目の解析で失敗する場合はpositionは0となる
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            line: 1,
            position: 0,
            peek_buffer: std::collections::VecDeque::new(),
            peek_offset: 0,
        }
    }

    /// 1文字先読みする
    /// 内部的には std::io::BufRead は1文字進む
    /// 外部的には peek 後に read しても peek と同じようを返す（peek していない場合は普通に std::io::BufRead から UTF-8 を１文字読む）
    pub fn peek(&mut self) -> Result<&(char, usize, usize), Error> {
        if self.peek_offset > 0 {
            Ok(self
                .peek_buffer
                .get(self.peek_buffer.len() - self.peek_offset)
                .map(|v| {
                    self.peek_offset -= 1;
                    v
                })
                .expect("peek_offsetアサイン時にpeek_bufferの内容を確認している"))
        } else {
            self.next().map(|result| {
                self.peek_buffer.push_back(result);
                self.peek_buffer
                    .back()
                    .expect("直前にpushしているため最後尾の取得に失敗しない")
            })
        }
    }

    /// peek のカーソルを１文字戻す
    /// peek が複数箇所から呼び出される場合にpeekが進みすぎていることを回避するために利用する
    /// peek に蓄えられた文字数以上にpeek_backすると Error::PeekBackError を返却する
    pub fn peek_back(&mut self) -> Result<(), Error> {
        if self.peek_buffer.len() < self.peek_offset + 1 {
            Err(Error::PeekBackError)
        } else {
            self.peek_offset += 1;
            Ok(())
        }
    }

    /// peek で蓄えられた文字を一気に引数の文字数分読み出す
    /// peek で蓄えられた文字数より多い文字数を指定すると Error::ConsumeError を返す
    pub fn consume(&mut self, i: usize) -> Result<String, Error> {
        let mut acc = Vec::new();
        for _ in 0..i {
            let (c, _, _) = self.peek_buffer.pop_front().ok_or(Error::ConsumeError)?;
            self.peek_offset = self.peek_offset.saturating_sub(1);
            acc.push(c);
        }

        Ok(acc.into_iter().collect::<String>())
    }

    /// peek で蓄えられた文字があればそれを、なければ reader から UTF-8 で１文字読み取り返却する
    /// reader の終端を読んでいる時は Error::EOF を返却する
    /// 多バイトの UTF-8 文字で続き文字が違反している場合は Error::InvalidUTF8 を返却する
    /// 読み取れた u32 が UTF-8 の文字に変換できない場合は Error::InvalidCodepoint を返却する
    pub fn read(&mut self) -> Result<(char, usize, usize), Error> {
        if self.peek_buffer.is_empty() {
            self.next()
        } else {
            // peek と良く似ているがこちらは実体を返却する
            Ok(self
                .peek_buffer
                .pop_front()
                .map(|v| {
                    self.peek_offset = self.peek_offset.saturating_sub(1);
                    v
                })
                .expect("peek_bufferを確認済みであるため必ず値は取れる"))
        }
    }

    fn next(&mut self) -> Result<(char, usize, usize), Error> {
        let mut buf = [0_u8; 1];
        self.reader
            .read(&mut buf)
            .map_err(|e| Error::ReadError(e.to_string()))
            .and_then(|v| {
                if v == 0 {
                    Err(Error::EOF(self.line, self.position))
                } else {
                    Ok(v)
                }
            })?;

        // utf8_char_width が利用できるようになればそちらを利用したほうが良い
        let codepoint = if 0b11111000 & buf[0] == 0b11110000 {
            // 4バイト文字
            let rest = self.read_rest::<3>()?;

            ((buf[0] as u32) & 0b0000_0111) << 18
                | ((rest[0] as u32) & 0b0011_1111) << 12
                | ((rest[1] as u32) & 0b0011_1111) << 6
                | (rest[2] as u32) & 0b0011_1111
        } else if buf[0] & 0b11110000 == 0b11100000 {
            // 3バイト文字
            let rest = self.read_rest::<2>()?;

            ((buf[0] as u32) & 0b0000_1111) << 12
                | ((rest[0] as u32) & 0b0011_1111) << 6
                | (rest[1] as u32) & 0b0011_1111
        } else if buf[0] & 0b11100000 == 0b11000000 {
            // 2バイト文字
            let rest = self.read_rest::<1>()?;

            ((buf[0] as u32) & 0b0001_1111) << 6 | (rest[0] as u32) & 0b0011_1111
        } else if buf[0] & 0b10000000 == 0 {
            // 1バイト文字
            buf[0] as u32
        } else {
            return Err(Error::InvalidUTF8(buf[0], self.line, self.position));
        };

        self.position += 1;

        char::from_u32(codepoint)
            .ok_or_else(|| Error::InvalidCodepoint(codepoint, self.line, self.position))
            .map(|c| {
                let r = (c, self.line, self.position);

                if c == '\n' {
                    self.line += 1;
                    self.position = 0;
                }

                r
            })
    }

    fn read_rest<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut rest = [0u8; N];
        self.reader
            .read(&mut rest)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::UnexpectedEof => Error::EOF(self.line, self.position),
                _ => Error::ReadError(e.to_string()),
            })
            .and_then(|v| {
                if v == 0 {
                    Err(Error::EOF(self.line, self.position))
                } else {
                    Ok(())
                }
            })?;

        for i in rest.iter() {
            if i & 0b1100_0000 != 0b1000_0000 {
                return Err(Error::InvalidUTF8(*i, self.line, self.position));
            }
        }

        Ok(rest)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_char_reader() {
        let source = r#"
        昨日、カフェで
        コーヒーを飲みながら
        漢字の勉強をしていたら、
        Friendが🫠の絵文字を
        送ってきて笑った。
        "#;

        let cursor = std::io::Cursor::new(source);
        let handle = std::io::BufReader::new(cursor);
        let mut char_reader = CharReader::new(handle);
        let mut current_pos = 0;
        let mut current_line = 1;
        let mut prev_return = false;

        for want in source.chars().take(8) {
            let got = char_reader.peek();
            assert!(got.is_ok());
            let (char, line, pos) = got.unwrap();

            if prev_return {
                current_pos = 1;
                current_line += 1;
            } else {
                current_pos += 1;
            }
            prev_return = want == '\n';
            assert_eq!(want, *char);
            assert_eq!(current_line, *line);
            assert_eq!(current_pos, *pos);
        }

        for _ in 0..8 {
            char_reader.peek_back().unwrap();
        }
        current_pos = 0;
        current_line = 1;
        let mut prev_return = false;

        for want in source.chars().take(10) {
            let got = char_reader.peek();
            assert!(got.is_ok());
            let (char, line, pos) = got.unwrap();
            if prev_return {
                current_pos = 1;
                current_line += 1;
            } else {
                current_pos += 1;
            }
            prev_return = want == '\n';
            assert_eq!(want, *char);
            assert_eq!(current_line, *line);
            assert_eq!(current_pos, *pos);
        }

        current_pos = 0;
        current_line = 1;
        let mut prev_return = false;

        for want in source.chars() {
            let got = char_reader.read();
            assert!(got.is_ok());
            let (char, line, pos) = got.unwrap();
            if prev_return {
                current_pos = 1;
                current_line += 1;
            } else {
                current_pos += 1;
            }
            prev_return = want == '\n';
            assert_eq!(want, char);
            assert_eq!(current_line, line);
            assert_eq!(current_pos, pos);
        }

        let e = char_reader.read();
        assert!(e.is_err());
        assert_eq!(e.unwrap_err(), Error::EOF(current_line, current_pos));
    }

    // https://x.com/jetbrains/status/1966787838663397726
    #[test]
    fn test_tweet() {
        let data: [u8; 38] = [
            0b01001000, 0b01100001, 0b01110000, 0b01110000, 0b01111001, 0b00100000, 0b01010000,
            0b01110010, 0b01101111, 0b01100111, 0b01110010, 0b01100001, 0b01101101, 0b01101101,
            0b01100101, 0b01110010, 0b00100111, 0b01110011, 0b00100000, 0b01000100, 0b01100001,
            0b01111001, 0b00100000, 0b01100110, 0b01110010, 0b01101111, 0b01101101, 0b00100000,
            0b01001010, 0b01100101, 0b01110100, 0b01000010, 0b01110010, 0b01100001, 0b01101001,
            0b01101110, 0b01110011, 0b00100001,
        ];

        let cursor = Cursor::new(data);
        let handle = std::io::BufReader::new(cursor);
        let mut char_reader = CharReader::new(handle);
        let mut buf = vec![];

        loop {
            match char_reader.next() {
                Err(Error::EOF(_, _)) => break,
                Err(e) => panic!("{}", e),
                Ok((c, _, _)) => buf.push(c),
            }
        }

        assert_eq!(
            buf.into_iter().collect::<String>(),
            "Happy Programmer's Day from JetBrains!"
        )
    }

    #[test]
    fn test_peek_and_read() {
        let source = "abcdef";
        let cursor = std::io::Cursor::new(source);
        let handle = std::io::BufReader::new(cursor);
        let mut char_reader = CharReader::new(handle);

        let result = char_reader.peek();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.0, 'a');
        assert_eq!(result.1, 1);
        assert_eq!(result.2, 1);

        let result = char_reader.peek_back();
        assert!(result.is_ok());

        let result = char_reader.peek_back();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::PeekBackError);

        let result = char_reader.peek();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.0, 'a');
        assert_eq!(result.1, 1);
        assert_eq!(result.2, 1);

        let result = char_reader.peek();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.0, 'b');
        assert_eq!(result.1, 1);
        assert_eq!(result.2, 2);

        let result = char_reader.peek();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'c');

        let result = char_reader.read();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'a');

        let result = char_reader.read();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'b');

        let result = char_reader.read();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'c');

        let result = char_reader.read();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'd');

        let result = char_reader.peek_back();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::PeekBackError);

        let result = char_reader.peek();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'e');

        let result = char_reader.peek_back();
        assert!(result.is_ok());

        let result = char_reader.read();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'e');

        let result = char_reader.peek();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 'f');

        let result = char_reader.consume(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "f".to_string());

        let result = char_reader.consume(1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::ConsumeError);
    }

    #[test]
    fn test_invalid_utf8() {
        let source = &[0b11110000, 0b11110000];
        let cursor = std::io::Cursor::new(source);
        let handle = std::io::BufReader::new(cursor);
        let mut char_reader = CharReader::new(handle);

        let result = char_reader.read();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::InvalidUTF8(0b11110000, 1, 0));

        let source = &[0b1111_0111, 0b1011_1111, 0b1011_1111, 0b1011_1111];
        let cursor = std::io::Cursor::new(source);
        let handle = std::io::BufReader::new(cursor);
        let mut char_reader = CharReader::new(handle);

        let expected = ((0b1111_0111 as u32) & 0b0000_0111) << 18
            | ((0b1011_1111 as u32) & 0b0011_1111) << 12
            | ((0b1011_1111 as u32) & 0b0011_1111) << 6
            | (0b1011_1111 as u32) & 0b0011_1111;

        let result = char_reader.read();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::InvalidCodepoint(expected, 1, 1));
    }
}
