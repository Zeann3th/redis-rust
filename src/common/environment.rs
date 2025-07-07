use std::collections::HashMap;

pub struct Environment {
    pub values: HashMap<String, String>,
}

#[allow(dead_code)]
impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    pub fn del(&mut self, key: &str) {
        self.values.remove(key);
    }
}
