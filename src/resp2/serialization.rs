#[allow(dead_code)]
pub trait Serialize<T> {
    fn serialize_bulk_string(&self) -> T;
    fn serialize_array(&self) -> T;
}

pub trait Deserialize<T> {
    fn deserialize(&mut self, input: T) -> Result<(), String>;
}
