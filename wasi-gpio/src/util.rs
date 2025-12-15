pub type Shared<T> = std::sync::Arc<std::sync::Mutex<T>>;

pub trait SharedExt<T> {
    fn make_shared(value: T) -> Shared<T>;
}

impl<T> SharedExt<T> for Shared<T> {
    fn make_shared(value: T) -> Shared<T> {
        std::sync::Arc::new(std::sync::Mutex::new(value))
    }
}
