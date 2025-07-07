pub mod command;
pub mod info;
pub mod serialization;

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use command::*;
use serialization::*;

use crate::{common::Environment, resp2::info::InfoSection};

pub struct Resp2 {
    kind: Resp2Command,
    data: Vec<String>,
    is_array: bool,
    stream: Option<std::net::TcpStream>,
    environment: Arc<Mutex<Environment>>,
}

#[allow(dead_code)]
impl Resp2 {
    pub fn new(environment: Arc<Mutex<Environment>>) -> Self {
        Resp2 {
            kind: Resp2Command::UNDEFINED,
            data: Vec::new(),
            is_array: true,
            stream: None,
            environment,
        }
    }

    pub fn set_kind(&mut self, kind: Resp2Command) {
        self.kind = kind;
    }

    pub fn set_data(&mut self, data: Vec<String>) {
        self.data = data;
    }

    pub fn set_is_array(&mut self, is_array: bool) {
        self.is_array = is_array;
    }

    pub fn set_stream(&mut self, stream: std::net::TcpStream) {
        self.stream = Some(stream);
    }

    pub fn set_environment(&mut self, environment: Arc<Mutex<Environment>>) {
        self.environment = environment;
    }

    fn handle_deserialization(&mut self, input: &str) -> Result<(), String> {
        let mut parts = input.split("\r\n");

        let len_str = parts.next().ok_or("Missing RESP2 array header")?;
        if !len_str.starts_with('*') {
            return Err(format!("Invalid RESP2 array header: '{}'", len_str));
        }

        let expected_len = len_str[1..]
            .parse::<usize>()
            .map_err(|_| format!("Invalid RESP2 array length: '{}'", len_str))?;

        self.data.clear();
        self.data.reserve(expected_len);

        for _ in 0..expected_len {
            let size_str = parts.next().ok_or("Missing $<size> header")?;
            if !size_str.starts_with('$') {
                return Err(format!("Expected $, got '{}'", size_str));
            }

            let size = size_str[1..]
                .parse::<usize>()
                .map_err(|_| format!("Invalid bulk string size: '{}'", size_str))?;

            let value = parts.next().ok_or("Missing bulk string content")?;
            if value.len() != size {
                return Err(format!(
                    "Data length mismatch: expected {}, got '{}'",
                    size, value
                ));
            }

            self.data.push(value.to_string());
        }

        self.kind = self
            .data
            .first()
            .map(|cmd| Resp2Command::from_str(cmd))
            .unwrap_or(Resp2Command::UNDEFINED);

        Ok(())
    }

    pub fn reflect(&mut self) -> Result<(), String> {
        let stream = self.stream.as_mut().ok_or("Missing stream")?;

        match self.kind {
            Resp2Command::PING => {
                stream
                    .write_all(b"+PONG\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::ECHO => {
                let msg = self.data.get(1).cloned().unwrap_or_default();
                let response = format!("+{}\r\n", msg);
                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::SET => {
                if self.data.len() < 3 {
                    return Err("SET command requires at least 3 arguments".to_string());
                }
                let key = &self.data[1];
                let value = &self.data[2];

                let mut exp: Option<u64> = None;
                if self.data.len() == 5 && self.data[3].to_uppercase() == "PX" {
                    let ms = &self.data[4];
                    exp = match ms.parse::<u64>() {
                        Ok(val) => Some(val),
                        Err(_) => {
                            return Err(format!("Invalid PX value: '{}'", ms));
                        }
                    };
                }

                self.environment.lock().map_err(|e| e.to_string())?.set(
                    key.clone(),
                    value.clone(),
                    exp,
                );
                stream
                    .write_all(b"+OK\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::GET => {
                if self.data.len() < 2 {
                    return Err("GET command requires at least 2 arguments".to_string());
                }
                let key = &self.data[1];
                let mut env = self.environment.lock().map_err(|e| e.to_string())?;
                match env.get(key) {
                    Some(val) => {
                        let response = format!("+{}\r\n", val);
                        stream
                            .write_all(response.as_bytes())
                            .map_err(|e| format!("Failed to write to stream: {}", e))?;
                    }
                    None => {
                        stream
                            .write_all(b"$-1\r\n")
                            .map_err(|e| format!("Failed to write to stream: {}", e))?;
                    }
                }
            }
            Resp2Command::INFO => {
                if self.data.len() < 2 {
                    return Err("INFO command requires at least 1 argument".to_string());
                }
                let section = &self.data[1];
                let response: Vec<u8> = match info::InfoSection::from_str(section) {
                    InfoSection::REPLICATION => {
                        let mut resp2_obj = Resp2::new(self.environment.clone());
                        resp2_obj.set_kind(Resp2Command::INFO);
                        resp2_obj.set_data(vec![format!(
                            "role:{}",
                            self.environment.lock().unwrap().role()
                        )]);
                        resp2_obj.set_is_array(false);

                        resp2_obj.serialize()
                    }
                };

                stream
                    .write_all(&response)
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            _ => {
                let err = b"-ERR unknown command\r\n";
                stream
                    .write_all(err)
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
        }

        Ok(())
    }
}

impl Serialize<String> for Resp2 {
    fn serialize(&self) -> String {
        let mut out = String::new();
        if self.is_array {
            out.push_str(&format!("*{}\r\n", self.data.len()));
        }
        for part in &self.data {
            out.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
        }

        out
    }
}

impl Serialize<Vec<u8>> for Resp2 {
    fn serialize(&self) -> Vec<u8> {
        let mut out = String::new();
        if self.is_array {
            out.push_str(&format!("*{}\r\n", self.data.len()));
        }

        let mut out = out.into_bytes();

        for part in &self.data {
            out.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
            out.extend_from_slice(part.as_bytes());
            out.push(b'\r');
            out.push(b'\n');
        }

        out
    }
}

impl Deserialize<&str> for Resp2 {
    fn deserialize(&mut self, input: &str) -> Result<(), String> {
        self.handle_deserialization(input)
    }
}

impl Deserialize<Vec<u8>> for Resp2 {
    fn deserialize(&mut self, input: Vec<u8>) -> Result<(), String> {
        let input = String::from_utf8(input).map_err(|e| e.to_string())?;
        self.handle_deserialization(&input)
    }
}
