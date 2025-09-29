/// std::io::BufRead からの読み出し時のエラーを表現する
#[derive(std::fmt::Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Peekバッファの範囲外へのpeek_backが要求されました")]
    PeekBackError,
    #[error("PeekされていないConsumeが発生しました")]
    ConsumeError,
    #[error("")]
    EOF(usize, usize),
    #[error(
        "Line: {1}, Position: {2} で不正なバイト（{0}）を検知しました。多バイト区切りが破損している可能性があります"
    )]
    InvalidUTF8(u8, usize, usize),
    #[error("Line: {1}, Position: {2} で不正なコードポイント（{0}）を検知しました")]
    InvalidCodepoint(u32, usize, usize),
    #[error("{0}")]
    ReadError(String),
}
