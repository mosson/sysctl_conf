// cSpell:disable
use thiserror::Error;
// cSpell:enable

#[derive(Error, Debug)]
pub enum Error {
    #[error("`key {1} value` の書式を満たしていません: {0}")]
    InvalidKeyValuePair(String, String),
    #[error("未定義のデータ型です: {0}")]
    UndefinedType(String),
    #[error("未定義のスキーマです: {0}")]
    UndefinedSchema(String),
    #[error("スキーマ違反です: {0} は {1} として解釈できません（{2}）")]
    InvalidSchema(String, String, String),
    #[error("{0}")]
    Unknown(String),
}
