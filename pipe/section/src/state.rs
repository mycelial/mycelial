//! Section State trait

pub trait State: Send + Sync + std::fmt::Debug + Clone + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn new() -> Self;
    fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<Option<T>, Self::Error>;
    fn set<T: Clone + Send + Sync + 'static>(&mut self, key: &str, value: T) -> Result<(), Self::Error>;
}
