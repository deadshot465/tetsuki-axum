#[derive(Debug, Clone)]
pub enum Response<T> {
    Success(T),
    NotFound(String),
    InternalError(String),
}
