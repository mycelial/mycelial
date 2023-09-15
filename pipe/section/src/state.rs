//! Section State trait

pub trait State: Send + Sync + std::fmt::Debug + Clone {
    fn new() -> Self;
    fn get<T>(&self, key: &str) -> Option<T>;
    fn set<T>(&mut self, key: &str, value: T);
}
