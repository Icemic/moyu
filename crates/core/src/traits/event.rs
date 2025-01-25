use serde::Serialize;

pub trait Event: Serialize + Send + 'static {
    fn name(&self) -> &'static str;
}
