use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use rand::{distr::Alphanumeric, Rng};

pub struct Environment {
    role: String,
    port: u16,
    master_replid: String,
    master_repl_offset: u64,
    slaves: Vec<SlaveConnection>,
    values: HashMap<String, (String, Option<SystemTime>)>,
}

#[allow(dead_code)]
pub struct SlaveConnection {
    stream: Arc<Mutex<TcpStream>>,
    offset: u64,
}

#[allow(dead_code)]
impl SlaveConnection {
    pub fn new(stream: Arc<Mutex<TcpStream>>, offset: u64) -> Self {
        SlaveConnection { stream, offset }
    }

    pub fn stream(&self) -> Arc<Mutex<TcpStream>> {
        Arc::clone(&self.stream)
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }
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
    pub fn new(role: String, port: u16) -> Self {
        Environment {
            role,
            port,
            master_replid: generate_replid(),
            master_repl_offset: 0,
            slaves: Vec::new(),
            values: HashMap::new(),
        }
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn port(&self) -> u16 {
        self.port
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

    pub fn add_slave(&mut self, stream: Arc<Mutex<TcpStream>>, offset: u64) {
        self.slaves.push(SlaveConnection { stream, offset });
    }

    pub fn slaves(&self) -> &Vec<SlaveConnection> {
        &self.slaves
    }

    pub fn get_slave(&self, stream: &TcpStream) -> Option<&SlaveConnection> {
        if let Ok(target_addr) = stream.peer_addr() {
            self.slaves.iter().find(|slave| {
                slave
                    .stream
                    .lock()
                    .ok()
                    .and_then(|s| s.peer_addr().ok())
                    .map_or(false, |addr| addr == target_addr)
            })
        } else {
            None
        }
    }

    pub fn remove_slave(&mut self, stream: &TcpStream) {
        if let Ok(addr) = stream.peer_addr() {
            self.slaves.retain(|slave| {
                slave
                    .stream
                    .lock()
                    .ok()
                    .and_then(|s| s.peer_addr().ok())
                    .map_or(true, |a| a != addr)
            });
        }
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
}
