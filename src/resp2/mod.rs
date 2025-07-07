pub mod command;
pub mod serialization;

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use command::*;
use serialization::*;

use crate::common::Environment;

pub struct Resp2 {
    pub kind: Command,
    pub data: Vec<String>,
    pub is_array: bool,
    pub stream: Option<std::net::TcpStream>,
    pub environment: Arc<Mutex<Environment>>,
}

impl Resp2 {
    pub fn new() -> Self {
        Resp2 {
            kind: Command::UNDEFINED,
            data: Vec::new(),
            is_array: true,
            stream: None,
            environment: Arc::new(Mutex::new(Environment::new())),
        }
    }

    pub fn set_stream(&mut self, stream: std::net::TcpStream) {
        self.stream = Some(stream);
    }

    pub fn set_environment(&mut self, environment: Arc<Mutex<Environment>>) {
        self.environment = environment;
    }

    pub fn process_deserialization(&mut self, input: &str) -> Result<(), String> {
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
            .map(|cmd| Command::from_str(cmd))
            .unwrap_or(Command::UNDEFINED);

        Ok(())
    }

    pub fn reflect(&mut self) -> Result<(), String> {
        let stream = self.stream.as_mut().ok_or("Missing stream")?;

        match self.kind {
            Command::PING => {
                stream
                    .write_all(b"+PONG\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Command::ECHO => {
                let msg = self.data.get(1).cloned().unwrap_or_default();
                let response = format!("+{}\r\n", msg);
                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Command::SET => {
                if self.data.len() < 3 {
                    return Err("SET command requires at least 3 arguments".to_string());
                }
                let key = &self.data[1];
                let value = &self.data[2];
                self.environment
                    .lock()
                    .map_err(|e| e.to_string())?
                    .set(key.clone(), value.clone());
                stream
                    .write_all(b"+OK\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Command::GET => {
                if self.data.len() < 2 {
                    return Err("GET command requires at least 2 arguments".to_string());
                }
                let key = &self.data[1];
                let env = self.environment.lock().map_err(|e| e.to_string())?;
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
        self.process_deserialization(input)
    }
}

impl Deserialize<Vec<u8>> for Resp2 {
    fn deserialize(&mut self, input: Vec<u8>) -> Result<(), String> {
        let input = String::from_utf8(input).map_err(|e| e.to_string())?;
        self.process_deserialization(&input)
    }
}
