#[allow(dead_code)]
pub trait Serialize<T> {
    fn serialize(&self) -> T;
}

pub trait Deserialize<T> {
    fn deserialize(&mut self, input: T) -> Result<(), String>;
}
