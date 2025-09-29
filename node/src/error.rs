#[derive(thiserror::Error, std::fmt::Debug)]
pub enum Error {
    #[error("{0}")]
    MismatchedType(String),
    #[error("値が割り当てられているキーにオブジェクトを再割り当てできません（{0}）")]
    ObjectOverride(String),
}
