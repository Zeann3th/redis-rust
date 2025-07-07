use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

#[allow(dead_code)]
pub struct Environment {
    role: String,
    master_replid: String,
    master_repl_offset: u64,
    values: HashMap<String, (String, Option<SystemTime>)>,
}

#[allow(dead_code)]
impl Environment {
    pub fn new(role: String, master_replid: String, master_repl_offset: u64) -> Self {
        Environment {
            role,
            master_replid,
            master_repl_offset,
            values: HashMap::new(),
        }
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn master_replid(&self) -> &str {
        &self.master_replid
    }

    pub fn master_repl_offset(&self) -> u64 {
        self.master_repl_offset
    }

    pub fn set(&mut self, key: String, value: String, px: Option<u64>) {
        let expiry = px.map(|ms| SystemTime::now() + Duration::from_millis(ms));
        self.values.insert(key, (value, expiry));
    }

    pub fn get(&mut self, key: &str) -> Option<&str> {
        let expired = self
            .values
            .get(key)
            .and_then(|(_, exp)| *exp)
            .map_or(false, |expiry| SystemTime::now() >= expiry);

        if expired {
            self.values.remove(key);
            return None;
        }

        self.values.get(key).map(|(val, _)| val.as_str())
    }

    pub fn del(&mut self, key: &str) {
        self.values.remove(key);
    }
}
