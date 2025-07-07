use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use rand::{distr::Alphanumeric, Rng};

#[allow(dead_code)]
pub struct Environment {
    role: String,
    port: u16,
    master_host: Option<String>,
    master_port: Option<u16>,
    master_replid: String,
    master_repl_offset: u64,
    values: HashMap<String, (String, Option<SystemTime>)>,
}

fn generate_replid() -> String {
    return rand::rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();
}

#[allow(dead_code)]
impl Environment {
    pub fn new(role: String, port: u16, host: Option<(String, u16)>) -> Self {
        Environment {
            role,
            port,
            master_host: host.clone().map(|(h, _)| h),
            master_port: host.clone().map(|(_, p)| p),
            master_replid: generate_replid(),
            master_repl_offset: 0,
            values: HashMap::new(),
        }
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn master_host(&self) -> Option<&str> {
        self.master_host.as_deref()
    }

    pub fn master_port(&self) -> Option<u16> {
        self.master_port
    }

    pub fn master_replid(&self) -> &str {
        &self.master_replid
    }

    pub fn master_repl_offset(&self) -> u64 {
        self.master_repl_offset
    }

    pub fn set_master_replid(&mut self, replid: String) {
        self.master_replid = replid;
    }

    pub fn set_master_repl_offset(&mut self, offset: u64) {
        self.master_repl_offset = offset;
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
